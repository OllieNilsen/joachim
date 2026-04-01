import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";
import * as command from "@pulumi/command";

// ---------------------------------------------------------------------------
// Mandatory resource tagging: ALL AWS resources get Project: joachim
// ---------------------------------------------------------------------------

const projectTags = { Project: "joachim" };

// Resource types that do NOT support tags — skip them in the transformation.
const untaggableTypes = new Set([
    "aws:iam/rolePolicyAttachment:RolePolicyAttachment",
    "aws:iam/rolePolicy:RolePolicy",
    "aws:iam/instanceProfile:InstanceProfile",
    "aws:lambda/permission:Permission",
    "aws:cloudwatch/eventTarget:EventTarget",
    "aws:s3/bucketLifecycleConfigurationV2:BucketLifecycleConfigurationV2",
    "aws:apigatewayv2/integration:Integration",
    "aws:apigatewayv2/route:Route",
    "aws:apigatewayv2/stage:Stage",
    "aws:secretsmanager/secretVersion:SecretVersion",
    "aws:cloudwatch/logGroup:LogGroup",
]);

pulumi.runtime.registerStackTransformation((args) => {
    if (untaggableTypes.has(args.type)) {
        return { props: args.props, opts: args.opts };
    }
    if (args.props.tags !== undefined) {
        args.props.tags = { ...args.props.tags, ...projectTags };
    } else if (args.type.startsWith("aws:")) {
        args.props.tags = projectTags;
    }
    return { props: args.props, opts: args.opts };
});

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const config = new pulumi.Config();
const region = pulumi.output(aws.getRegion()).name;
const modelId =
    config.get("modelId") || "anthropic.claude-sonnet-4-20250514";
const smokeTestUsername = config.get("smokeTestUsername") || "smoke-test@joachim.dev";
const smokeTestPassword = config.requireSecret("smokeTestPassword");

// ---------------------------------------------------------------------------
// 1. Cognito User Pool + App Client
// ---------------------------------------------------------------------------

const userPool = new aws.cognito.UserPool("joachim-api-users", {
    name: "joachim-api-users",
    autoVerifiedAttributes: ["email"],
    usernameAttributes: ["email"],
    adminCreateUserConfig: {
        allowAdminCreateUserOnly: true,
    },
    passwordPolicy: {
        minimumLength: 12,
        requireLowercase: true,
        requireUppercase: true,
        requireNumbers: true,
        requireSymbols: false,
    },
});

const userPoolClient = new aws.cognito.UserPoolClient("joachim-api-client", {
    name: "joachim-api-client",
    userPoolId: userPool.id,
    explicitAuthFlows: ["ALLOW_USER_PASSWORD_AUTH", "ALLOW_REFRESH_TOKEN_AUTH"],
    generateSecret: false,
});

// ---------------------------------------------------------------------------
// 2. Lambda execution role
// ---------------------------------------------------------------------------

const lambdaRole = new aws.iam.Role("joachim-api-lambda-role", {
    assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
        Service: "lambda.amazonaws.com",
    }),
    description: "Execution role for JOACHIM detection Lambda",
});

new aws.iam.RolePolicyAttachment("joachim-api-lambda-basic-logs", {
    role: lambdaRole.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaBasicExecutionRole,
});

new aws.iam.RolePolicy("joachim-api-lambda-bedrock-policy", {
    role: lambdaRole.id,
    policy: region.apply((rgn) =>
        JSON.stringify({
            Version: "2012-10-17",
            Statement: [
                {
                    Effect: "Allow",
                    Action: ["bedrock:InvokeModel"],
                    Resource: [
                        `arn:aws:bedrock:${rgn}::foundation-model/${modelId}`,
                    ],
                },
            ],
        }),
    ),
});

// ---------------------------------------------------------------------------
// 3. Lambda function
// ---------------------------------------------------------------------------

// The Lambda code zip is provided by the deploy workflow.
// During initial Pulumi up, use a placeholder.
const lambdaFn = new aws.lambda.Function("joachim-api-detect", {
    runtime: "provided.al2023",
    architectures: ["arm64"],
    role: lambdaRole.arn,
    handler: "bootstrap",
    memorySize: 256,
    timeout: 60,
    environment: {
        variables: {
            MODEL_ID: modelId,
            AWS_REGION_OVERRIDE: region.apply((r) => r),
        },
    },
    code: new pulumi.asset.AssetArchive({
        bootstrap: new pulumi.asset.StringAsset("#!/bin/sh\nexit 1\n"),
    }),
});

// ---------------------------------------------------------------------------
// 4. API Gateway HTTP API + JWT authorizer
// ---------------------------------------------------------------------------

const api = new aws.apigatewayv2.Api("joachim-api", {
    protocolType: "HTTP",
    description: "JOACHIM prompt injection detection API",
});

const jwtAuthorizer = new aws.apigatewayv2.Authorizer(
    "joachim-api-jwt-auth",
    {
        apiId: api.id,
        authorizerType: "JWT",
        identitySources: ["$request.header.Authorization"],
        jwtConfiguration: {
            issuer: pulumi.interpolate`https://cognito-idp.${region}.amazonaws.com/${userPool.id}`,
            audiences: [userPoolClient.id],
        },
    },
);

const lambdaIntegration = new aws.apigatewayv2.Integration(
    "joachim-api-integration",
    {
        apiId: api.id,
        integrationType: "AWS_PROXY",
        integrationUri: lambdaFn.arn,
        integrationMethod: "POST",
        payloadFormatVersion: "2.0",
    },
);

new aws.apigatewayv2.Route("joachim-api-detect-route", {
    apiId: api.id,
    routeKey: "POST /detect",
    target: pulumi.interpolate`integrations/${lambdaIntegration.id}`,
    authorizationType: "JWT",
    authorizerId: jwtAuthorizer.id,
});

const apiStage = new aws.apigatewayv2.Stage("joachim-api-stage", {
    apiId: api.id,
    name: "$default",
    autoDeploy: true,
    defaultRouteSettings: {
        throttlingBurstLimit: 100,
        throttlingRateLimit: 50,
    },
    accessLogSettings: {
        destinationArn: new aws.cloudwatch.LogGroup("joachim-api-access-logs", {
            retentionInDays: 30,
        }).arn,
        format: JSON.stringify({
            requestId: "$context.requestId",
            ip: "$context.identity.sourceIp",
            requestTime: "$context.requestTime",
            httpMethod: "$context.httpMethod",
            routeKey: "$context.routeKey",
            status: "$context.status",
            protocol: "$context.protocol",
            responseLength: "$context.responseLength",
            integrationLatency: "$context.integrationLatency",
        }),
    },
});

new aws.lambda.Permission("joachim-api-gateway-permission", {
    action: "lambda:InvokeFunction",
    function: lambdaFn.name,
    principal: "apigateway.amazonaws.com",
    sourceArn: pulumi.interpolate`${api.executionArn}/*/*`,
});

// ---------------------------------------------------------------------------
// 5. CloudWatch alarms
// ---------------------------------------------------------------------------

new aws.cloudwatch.MetricAlarm("joachim-api-lambda-errors-alarm", {
    alarmDescription: "JOACHIM API Lambda errors",
    namespace: "AWS/Lambda",
    metricName: "Errors",
    statistic: "Sum",
    period: 300,
    evaluationPeriods: 1,
    threshold: 0,
    comparisonOperator: "GreaterThanThreshold",
    dimensions: { FunctionName: lambdaFn.name },
});

new aws.cloudwatch.MetricAlarm("joachim-api-lambda-duration-alarm", {
    alarmDescription: "JOACHIM API Lambda p99 duration > 30s",
    namespace: "AWS/Lambda",
    metricName: "Duration",
    extendedStatistic: "p99",
    period: 300,
    evaluationPeriods: 1,
    threshold: 30000,
    comparisonOperator: "GreaterThanThreshold",
    dimensions: { FunctionName: lambdaFn.name },
});

// ---------------------------------------------------------------------------
// 6. Smoke test user (idempotent)
// ---------------------------------------------------------------------------

const smokeTestUser = new command.local.Command("joachim-smoke-test-user", {
    create: pulumi.interpolate`
        aws cognito-idp admin-create-user \
            --user-pool-id ${userPool.id} \
            --username "${smokeTestUsername}" \
            --temporary-password "TempPass123!" \
            --message-action SUPPRESS \
            --region ${region} 2>/dev/null || true
        aws cognito-idp admin-set-user-password \
            --user-pool-id ${userPool.id} \
            --username "${smokeTestUsername}" \
            --password "${smokeTestPassword}" \
            --permanent \
            --region ${region}
    `,
});

const smokeTestSecret = new aws.secretsmanager.Secret(
    "joachim-smoke-test-secret",
    {
        name: "joachim/smoke-test-user",
        description: "Smoke test user credentials for JOACHIM API",
    },
);

new aws.secretsmanager.SecretVersion(
    "joachim-smoke-test-secret-version",
    {
        secretId: smokeTestSecret.id,
        secretString: pulumi
            .all([smokeTestUsername, smokeTestPassword])
            .apply(([user, pass]) =>
                JSON.stringify({ username: user, password: pass }),
            ),
    },
    { dependsOn: [smokeTestUser] },
);

// ---------------------------------------------------------------------------
// 7. Exports
// ---------------------------------------------------------------------------

export const apiUrl = api.apiEndpoint;
export const lambdaFunctionName = lambdaFn.name;
export const lambdaRoleArn = lambdaRole.arn;
export const userPoolId = userPool.id;
export const userPoolClientId = userPoolClient.id;
export const smokeTestSecretArn = smokeTestSecret.arn;
