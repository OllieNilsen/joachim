import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";

// ---------------------------------------------------------------------------
// Mandatory resource tagging: ALL AWS resources get Project: joachim
// ---------------------------------------------------------------------------

const projectTags = { Project: "joachim" };

pulumi.runtime.registerStackTransformation((args) => {
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
const accountId = pulumi.output(aws.getCallerIdentity()).accountId;
const region = pulumi.output(aws.getRegion()).name;
const runnerAmiId = config.require("runnerAmiId");
const runnerInstanceType = config.get("runnerInstanceType") || "c7g.xlarge";
const sccacheBucketConfigName =
    config.get("sccacheBucketName") || "joachim-ci-sccache-us-east-1";

// ---------------------------------------------------------------------------
// 1. Networking: Dedicated VPC for CI runners
// ---------------------------------------------------------------------------

const ciVpc = new awsx.ec2.Vpc("joachim-ci-vpc", {
    numberOfAvailabilityZones: 2,
    natGateways: { strategy: "None" },
    subnetStrategy: "Auto",
});

const runnerSecurityGroup = new aws.ec2.SecurityGroup("joachim-ci-sg", {
    vpcId: ciVpc.vpcId,
    description: "Security group for ephemeral JOACHIM CI runners",
    ingress: [],
    egress: [
        {
            protocol: "-1",
            fromPort: 0,
            toPort: 0,
            cidrBlocks: ["0.0.0.0/0"],
        },
    ],
});

// ---------------------------------------------------------------------------
// 2. Launch template: IMDSv2, no SSH, hardened
// ---------------------------------------------------------------------------

const runnerLaunchTemplate = new aws.ec2.LaunchTemplate(
    "joachim-ci-launch-template",
    {
        namePrefix: "joachim-ci-",
        description: "Hardened launch template for ephemeral JOACHIM CI runners",
        imageId: runnerAmiId,
        instanceType: runnerInstanceType,
        vpcSecurityGroupIds: [runnerSecurityGroup.id],
        networkInterfaces: [
            {
                associatePublicIpAddress: "true",
                securityGroups: [runnerSecurityGroup.id],
                subnetId: undefined,
            },
        ],
        metadataOptions: {
            httpEndpoint: "enabled",
            httpTokens: "required",
            httpPutResponseHopLimit: 1,
        },
        updateDefaultVersion: true,
    },
);

// ---------------------------------------------------------------------------
// 3. sccache S3 bucket with lifecycle policies
// ---------------------------------------------------------------------------

const cacheBucket = new aws.s3.BucketV2("joachim-sccache-bucket", {
    bucket: sccacheBucketConfigName,
    forceDestroy: true,
});

new aws.s3.BucketLifecycleConfigurationV2("joachim-sccache-lifecycle", {
    bucket: cacheBucket.id,
    rules: [
        {
            id: "expire-pr-caches",
            filter: { prefix: "pr-" },
            status: "Enabled",
            expiration: { days: 14 },
        },
        {
            id: "expire-main-caches",
            filter: { prefix: "main/" },
            status: "Enabled",
            expiration: { days: 90 },
        },
    ],
});

// ---------------------------------------------------------------------------
// 4. IAM: Runner instance role (what the EC2 instance can do)
// ---------------------------------------------------------------------------

const runnerRole = new aws.iam.Role("joachim-ci-instance-role", {
    assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
        Service: "ec2.amazonaws.com",
    }),
    description: "IAM role assumed by ephemeral JOACHIM CI runner instances",
});

new aws.iam.RolePolicy("joachim-ci-s3-policy", {
    role: runnerRole.id,
    policy: pulumi
        .all([cacheBucket.arn])
        .apply(([bucketArn]) =>
            JSON.stringify({
                Version: "2012-10-17",
                Statement: [
                    {
                        Sid: "ReadWriteOwnCachePrefix",
                        Effect: "Allow",
                        Action: ["s3:GetObject", "s3:PutObject", "s3:ListBucket"],
                        Resource: [bucketArn, `${bucketArn}/pr-*`],
                    },
                    {
                        Sid: "ReadOnlyMainCachePrefix",
                        Effect: "Allow",
                        Action: ["s3:GetObject", "s3:ListBucket"],
                        Resource: [bucketArn, `${bucketArn}/main/*`],
                    },
                ],
            }),
        ),
});

const runnerInstanceProfile = new aws.iam.InstanceProfile(
    "joachim-ci-profile",
    { role: runnerRole.name },
);

// ---------------------------------------------------------------------------
// 5. IAM: OIDC controller role (what GitHub Actions can do)
// ---------------------------------------------------------------------------

const githubOidcProviderArn = pulumi.interpolate`arn:aws:iam::${accountId}:oidc-provider/token.actions.githubusercontent.com`;

const controllerRole = new aws.iam.Role("joachim-ci-controller-role", {
    description:
        "IAM role assumed by GitHub Actions to spawn/terminate JOACHIM CI runners",
    assumeRolePolicy: pulumi
        .all([accountId])
        .apply(([id]) =>
            JSON.stringify({
                Version: "2012-10-17",
                Statement: [
                    {
                        Effect: "Allow",
                        Principal: {
                            Federated: `arn:aws:iam::${id}:oidc-provider/token.actions.githubusercontent.com`,
                        },
                        Action: "sts:AssumeRoleWithWebIdentity",
                        Condition: {
                            StringEquals: {
                                "token.actions.githubusercontent.com:aud":
                                    "sts.amazonaws.com",
                            },
                            StringLike: {
                                "token.actions.githubusercontent.com:sub":
                                    "repo:OllieNilsen/joachim:*",
                            },
                        },
                    },
                ],
            }),
        ),
});

new aws.iam.RolePolicy("joachim-ci-controller-policy", {
    role: controllerRole.id,
    policy: pulumi
        .all([
            ciVpc.publicSubnetIds,
            runnerSecurityGroup.id,
            runnerInstanceProfile.arn,
            accountId,
            region,
        ])
        .apply(([subnetIds, sgId, _profileArn, accId, rgn]) =>
            JSON.stringify({
                Version: "2012-10-17",
                Statement: [
                    {
                        Effect: "Allow",
                        Action: "ec2:RunInstances",
                        Resource: [
                            `arn:aws:ec2:${rgn}:${accId}:instance/*`,
                            `arn:aws:ec2:${rgn}:${accId}:volume/*`,
                            `arn:aws:ec2:${rgn}:${accId}:network-interface/*`,
                            `arn:aws:ec2:${rgn}:${accId}:key-pair/*`,
                            `arn:aws:ec2:${rgn}:${accId}:security-group/${sgId}`,
                            `arn:aws:ec2:${rgn}:${accId}:launch-template/*`,
                            ...subnetIds.map(
                                (id: string) =>
                                    `arn:aws:ec2:${rgn}:${accId}:subnet/${id}`,
                            ),
                            `arn:aws:ec2:${rgn}::image/*`,
                        ],
                        Condition: {
                            StringEquals: {
                                "aws:RequestTag/ManagedBy": "github-actions-ci",
                            },
                        },
                    },
                    {
                        Effect: "Allow",
                        Action: "ec2:CreateTags",
                        Resource: `arn:aws:ec2:${rgn}:${accId}:instance/*`,
                        Condition: {
                            StringEquals: { "ec2:CreateAction": "RunInstances" },
                        },
                    },
                    {
                        Effect: "Allow",
                        Action: "iam:PassRole",
                        Resource: [runnerRole.arn],
                    },
                    {
                        Effect: "Allow",
                        Action: [
                            "ec2:TerminateInstances",
                            "ec2:DescribeInstances",
                            "ec2:DescribeSpotInstanceRequests",
                        ],
                        Resource: "*",
                    },
                ],
            }),
        ),
});

// ---------------------------------------------------------------------------
// 6. Webhook ingest: signature validation, dedup, EventBridge
// ---------------------------------------------------------------------------

const githubWebhookSecret = config.requireSecret("githubWebhookSecret");

const webhookDeliveryTable = new aws.dynamodb.Table(
    "joachim-ci-webhook-deliveries",
    {
        attributes: [{ name: "deliveryId", type: "S" }],
        hashKey: "deliveryId",
        billingMode: "PAY_PER_REQUEST",
        ttl: { attributeName: "expiresAt", enabled: true },
    },
);

const ingestDlq = new aws.sqs.Queue("joachim-ci-webhook-ingest-dlq", {
    messageRetentionSeconds: 1209600,
});

const ingestRole = new aws.iam.Role("joachim-ci-webhook-ingest-role", {
    assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
        Service: "lambda.amazonaws.com",
    }),
});

new aws.iam.RolePolicyAttachment("joachim-ci-webhook-ingest-basic-logs", {
    role: ingestRole.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaBasicExecutionRole,
});

new aws.iam.RolePolicy("joachim-ci-webhook-ingest-policy", {
    role: ingestRole.id,
    policy: pulumi
        .all([webhookDeliveryTable.arn])
        .apply(([tableArn]) =>
            JSON.stringify({
                Version: "2012-10-17",
                Statement: [
                    {
                        Sid: "DedupDelivery",
                        Effect: "Allow",
                        Action: ["dynamodb:PutItem"],
                        Resource: [tableArn],
                    },
                    {
                        Sid: "PublishToEventBridge",
                        Effect: "Allow",
                        Action: ["events:PutEvents"],
                        Resource: ["*"],
                    },
                    {
                        Sid: "DlqWrite",
                        Effect: "Allow",
                        Action: ["sqs:SendMessage"],
                        Resource: [ingestDlq.arn],
                    },
                ],
            }),
        ),
});

const ingestFn = new aws.lambda.Function("joachim-ci-webhook-ingest", {
    runtime: "nodejs20.x",
    role: ingestRole.arn,
    handler: "index.handler",
    timeout: 15,
    memorySize: 256,
    deadLetterConfig: { targetArn: ingestDlq.arn },
    environment: {
        variables: {
            WEBHOOK_SECRET: githubWebhookSecret,
            DELIVERY_TABLE: webhookDeliveryTable.name,
            REPLAY_WINDOW_SECONDS: "900",
            REPO: "OllieNilsen/joachim",
        },
    },
    code: new pulumi.asset.AssetArchive({
        "index.js": new pulumi.asset.StringAsset(`
const crypto = require("crypto");
const { DynamoDB, EventBridge } = require("@aws-sdk/client-dynamodb");
const { EventBridgeClient, PutEventsCommand } = require("@aws-sdk/client-eventbridge");
const ddb = new DynamoDB();
const events = new EventBridgeClient();

function safeEqual(a, b) {
  const left = Buffer.from(a || "");
  const right = Buffer.from(b || "");
  if (left.length !== right.length) return false;
  return crypto.timingSafeEqual(left, right);
}

exports.handler = async (event) => {
  const headers = Object.fromEntries(
    Object.entries(event.headers || {}).map(([k, v]) => [String(k).toLowerCase(), v]),
  );
  const signature = headers["x-hub-signature-256"] || "";
  const deliveryId = headers["x-github-delivery"] || "";
  const ghEvent = headers["x-github-event"] || "";

  if (!signature || !deliveryId || !ghEvent) {
    return { statusCode: 400, body: "missing headers" };
  }

  const bodyRaw = event.isBase64Encoded
    ? Buffer.from(event.body || "", "base64").toString("utf8")
    : (event.body || "");

  const expected = "sha256=" + crypto.createHmac("sha256", process.env.WEBHOOK_SECRET || "").update(bodyRaw).digest("hex");
  if (!safeEqual(expected, signature)) {
    return { statusCode: 401, body: "invalid signature" };
  }

  let payload;
  try { payload = JSON.parse(bodyRaw); } catch { return { statusCode: 400, body: "invalid json" }; }

  if (ghEvent !== "workflow_job" || payload.action !== "completed") {
    return { statusCode: 202, body: "ignored" };
  }

  if ((payload.repository && payload.repository.full_name) !== process.env.REPO) {
    return { statusCode: 403, body: "repo mismatch" };
  }

  const completedAt = payload.workflow_job && payload.workflow_job.completed_at;
  if (!completedAt) { return { statusCode: 400, body: "missing completed_at" }; }

  const replayWindow = Number(process.env.REPLAY_WINDOW_SECONDS || "900");
  const ageSeconds = Math.floor((Date.now() - new Date(completedAt).getTime()) / 1000);
  if (Number.isNaN(ageSeconds) || ageSeconds < 0 || ageSeconds > replayWindow) {
    return { statusCode: 401, body: "replay window exceeded" };
  }

  const expiresAt = Math.floor(Date.now() / 1000) + 86400;
  try {
    await ddb.putItem({
      TableName: process.env.DELIVERY_TABLE,
      Item: { deliveryId: { S: deliveryId }, expiresAt: { N: String(expiresAt) } },
      ConditionExpression: "attribute_not_exists(deliveryId)",
    });
  } catch (err) {
    if (err && err.name === "ConditionalCheckFailedException") {
      return { statusCode: 202, body: "duplicate" };
    }
    throw err;
  }

  const labels = (payload.workflow_job && payload.workflow_job.labels) || [];
  const ec2Label = labels.find((l) => typeof l === "string" && l.startsWith("ec2-instance-id:"));
  const ec2InstanceId = ec2Label ? ec2Label.split(":")[1] : undefined;

  await events.send(new PutEventsCommand({
    Entries: [{
      Source: "github.actions",
      DetailType: "workflow_job.completed",
      EventBusName: "default",
      Detail: JSON.stringify({
        repository: payload.repository.full_name,
        workflow_job_id: payload.workflow_job.id,
        run_id: payload.workflow_job.run_id,
        ec2_instance_id: ec2InstanceId,
        delivery_id: deliveryId,
        completed_at: payload.workflow_job.completed_at,
      }),
    }],
  }));

  return { statusCode: 200, body: "ok" };
};
        `),
    }),
});

const webhookApi = new aws.apigatewayv2.Api("joachim-ci-webhook-api", {
    protocolType: "HTTP",
});

const webhookIntegration = new aws.apigatewayv2.Integration(
    "joachim-ci-webhook-integration",
    {
        apiId: webhookApi.id,
        integrationType: "AWS_PROXY",
        integrationUri: ingestFn.arn,
        integrationMethod: "POST",
        payloadFormatVersion: "2.0",
    },
);

new aws.apigatewayv2.Route("joachim-ci-webhook-route", {
    apiId: webhookApi.id,
    routeKey: "POST /github/webhook",
    target: pulumi.interpolate`integrations/${webhookIntegration.id}`,
});

new aws.apigatewayv2.Stage("joachim-ci-webhook-stage", {
    apiId: webhookApi.id,
    name: "$default",
    autoDeploy: true,
});

new aws.lambda.Permission("joachim-ci-webhook-api-permission", {
    action: "lambda:InvokeFunction",
    function: ingestFn.name,
    principal: "apigateway.amazonaws.com",
    sourceArn: pulumi.interpolate`${webhookApi.executionArn}/*/*`,
});

// ---------------------------------------------------------------------------
// 7. GC Lambda: terminate orphaned runners
// ---------------------------------------------------------------------------

const gcDlq = new aws.sqs.Queue("joachim-ci-gc-dlq", {
    messageRetentionSeconds: 1209600,
});

const gcRole = new aws.iam.Role("joachim-ci-gc-role", {
    assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
        Service: "lambda.amazonaws.com",
    }),
});

new aws.iam.RolePolicyAttachment("joachim-ci-gc-basic-logs", {
    role: gcRole.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaBasicExecutionRole,
});

new aws.iam.RolePolicy("joachim-ci-gc-policy", {
    role: gcRole.id,
    policy: pulumi
        .all([accountId, region])
        .apply(([accId, rgn]) =>
            JSON.stringify({
                Version: "2012-10-17",
                Statement: [
                    {
                        Sid: "DescribeInstances",
                        Effect: "Allow",
                        Action: ["ec2:DescribeInstances", "ec2:DescribeTags"],
                        Resource: "*",
                    },
                    {
                        Sid: "TerminateTaggedRunners",
                        Effect: "Allow",
                        Action: ["ec2:TerminateInstances"],
                        Resource: `arn:aws:ec2:${rgn}:${accId}:instance/*`,
                        Condition: {
                            StringEquals: {
                                "ec2:ResourceTag/ManagedBy": "github-actions-ci",
                            },
                        },
                    },
                    {
                        Sid: "DlqWrite",
                        Effect: "Allow",
                        Action: ["sqs:SendMessage"],
                        Resource: gcDlq.arn,
                    },
                ],
            }),
        ),
});

const gcFn = new aws.lambda.Function("joachim-ci-gc", {
    runtime: "nodejs20.x",
    role: gcRole.arn,
    handler: "index.handler",
    timeout: 30,
    memorySize: 256,
    environment: {
        variables: {
            REQUIRED_TAG_KEY: "ManagedBy",
            REQUIRED_TAG_VALUE: "github-actions-ci",
            MIN_INSTANCE_AGE_SECONDS: "300",
            DRY_RUN: "false",
        },
    },
    deadLetterConfig: { targetArn: gcDlq.arn },
    code: new pulumi.asset.AssetArchive({
        "index.js": new pulumi.asset.StringAsset(`
const { EC2Client, DescribeInstancesCommand, TerminateInstancesCommand } = require("@aws-sdk/client-ec2");
const ec2 = new EC2Client();

exports.handler = async (event) => {
  const detail = event && event.detail ? event.detail : {};
  const instanceId = detail.ec2_instance_id || detail.instance_id;
  if (!instanceId) { return { ok: true, skipped: true }; }

  const requiredTagKey = process.env.REQUIRED_TAG_KEY || "ManagedBy";
  const requiredTagValue = process.env.REQUIRED_TAG_VALUE || "github-actions-ci";
  const minAgeSeconds = Number(process.env.MIN_INSTANCE_AGE_SECONDS || "300");
  const dryRun = (process.env.DRY_RUN || "false") === "true";

  const desc = await ec2.send(new DescribeInstancesCommand({ InstanceIds: [instanceId] }));
  const inst = (desc.Reservations || []).flatMap(r => r.Instances || [])[0];
  if (!inst) { return { ok: true, skipped: true }; }

  const state = inst.State && inst.State.Name;
  if (state === "terminated" || state === "shutting-down") { return { ok: true, skipped: true }; }

  const launch = inst.LaunchTime ? new Date(inst.LaunchTime).getTime() : Date.now();
  const ageSeconds = Math.floor((Date.now() - launch) / 1000);
  if (ageSeconds < minAgeSeconds) { return { ok: true, skipped: true }; }

  const tags = Object.fromEntries((inst.Tags || []).map(t => [t.Key, t.Value]));
  if (tags[requiredTagKey] !== requiredTagValue) { return { ok: true, skipped: true }; }

  if (dryRun) { return { ok: true, dryRun: true, wouldTerminate: instanceId }; }

  await ec2.send(new TerminateInstancesCommand({ InstanceIds: [instanceId] }));
  console.log("Terminated orphan runner", instanceId);
  return { ok: true, terminated: instanceId };
};
        `),
    }),
});

const gcRule = new aws.cloudwatch.EventRule("joachim-ci-gc-rule", {
    eventPattern: JSON.stringify({
        source: ["github.actions"],
        "detail-type": ["workflow_job.completed"],
        detail: { repository: ["OllieNilsen/joachim"] },
    }),
});

new aws.cloudwatch.EventTarget("joachim-ci-gc-target", {
    rule: gcRule.name,
    arn: gcFn.arn,
    retryPolicy: { maximumEventAgeInSeconds: 3600, maximumRetryAttempts: 5 },
    deadLetterConfig: { arn: gcDlq.arn },
});

new aws.lambda.Permission("joachim-ci-gc-event-permission", {
    action: "lambda:InvokeFunction",
    function: gcFn.name,
    principal: "events.amazonaws.com",
    sourceArn: gcRule.arn,
});

// ---------------------------------------------------------------------------
// 8. Spot interruption interceptor
// ---------------------------------------------------------------------------

const spotInterceptorRole = new aws.iam.Role(
    "joachim-ci-spot-interceptor-role",
    {
        assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
            Service: "lambda.amazonaws.com",
        }),
    },
);

new aws.iam.RolePolicyAttachment("joachim-ci-spot-interceptor-basic-logs", {
    role: spotInterceptorRole.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaBasicExecutionRole,
});

new aws.iam.RolePolicy("joachim-ci-spot-interceptor-policy", {
    role: spotInterceptorRole.id,
    policy: pulumi
        .all([accountId, region])
        .apply(([accId, rgn]) =>
            JSON.stringify({
                Version: "2012-10-17",
                Statement: [
                    {
                        Sid: "DescribeAndTerminate",
                        Effect: "Allow",
                        Action: ["ec2:DescribeInstances", "ec2:TerminateInstances"],
                        Resource: [
                            "*",
                            `arn:aws:ec2:${rgn}:${accId}:instance/*`,
                        ],
                    },
                ],
            }),
        ),
});

const githubRunnerPat = config.getSecret("githubRunnerPat");

const spotInterceptorFn = new aws.lambda.Function(
    "joachim-ci-spot-interceptor",
    {
        runtime: "nodejs20.x",
        role: spotInterceptorRole.arn,
        handler: "index.handler",
        timeout: 20,
        memorySize: 256,
        environment: {
            variables: { GITHUB_PAT: githubRunnerPat || "" },
        },
        code: new pulumi.asset.AssetArchive({
            "index.js": new pulumi.asset.StringAsset(`
const { EC2Client, DescribeInstancesCommand, TerminateInstancesCommand } = require("@aws-sdk/client-ec2");
const https = require("https");
const ec2 = new EC2Client();

function requestRerun(repo, runId, token) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: "api.github.com",
      path: "/repos/" + repo + "/actions/runs/" + runId + "/rerun",
      method: "POST",
      headers: {
        "Authorization": "Bearer " + token,
        "Accept": "application/vnd.github+json",
        "User-Agent": "joachim-ci-spot-interceptor",
        "Content-Type": "application/json",
      },
    };
    const req = https.request(options, (res) => {
      res.statusCode >= 200 && res.statusCode < 300 ? resolve() : reject(new Error("rerun failed: " + res.statusCode));
    });
    req.on("error", reject);
    req.end(JSON.stringify({}));
  });
}

exports.handler = async (event) => {
  const detail = event && event.detail ? event.detail : {};
  const instanceId = detail["instance-id"];
  if (!instanceId) { return { ok: true, skipped: true }; }

  const resp = await ec2.send(new DescribeInstancesCommand({ InstanceIds: [instanceId] }));
  const inst = (resp.Reservations || []).flatMap(r => r.Instances || [])[0];
  if (!inst) { return { ok: true, skipped: true }; }

  const tags = Object.fromEntries((inst.Tags || []).map(t => [t.Key, t.Value]));
  if (tags.ManagedBy !== "github-actions-ci") { return { ok: true, skipped: true }; }

  const state = inst.State && inst.State.Name;
  if (state !== "terminated" && state !== "shutting-down") {
    await ec2.send(new TerminateInstancesCommand({ InstanceIds: [instanceId] }));
  }

  const token = process.env.GITHUB_PAT;
  const repo = tags.Repo;
  const runId = tags.RunId;
  if (token && repo && runId) {
    try { await requestRerun(repo, runId, token); } catch (err) { console.log("rerun failed", err.message); }
  }

  return { ok: true, instanceId, repo, runId };
};
            `),
        }),
    },
);

const spotInterruptRule = new aws.cloudwatch.EventRule(
    "joachim-ci-spot-interrupt-rule",
    {
        eventPattern: JSON.stringify({
            source: ["aws.ec2"],
            "detail-type": ["EC2 Spot Instance Interruption Warning"],
        }),
    },
);

new aws.cloudwatch.EventTarget("joachim-ci-spot-interrupt-target", {
    rule: spotInterruptRule.name,
    arn: spotInterceptorFn.arn,
});

new aws.lambda.Permission("joachim-ci-spot-interrupt-permission", {
    action: "lambda:InvokeFunction",
    function: spotInterceptorFn.name,
    principal: "events.amazonaws.com",
    sourceArn: spotInterruptRule.arn,
});

// ---------------------------------------------------------------------------
// 9. CloudWatch alarms and dashboard
// ---------------------------------------------------------------------------

new aws.cloudwatch.MetricAlarm("joachim-ci-gc-dlq-alarm", {
    alarmDescription: "JOACHIM CI GC DLQ has pending messages",
    namespace: "AWS/SQS",
    metricName: "ApproximateNumberOfMessagesVisible",
    statistic: "Sum",
    period: 300,
    evaluationPeriods: 1,
    threshold: 0,
    comparisonOperator: "GreaterThanThreshold",
    dimensions: { QueueName: gcDlq.name },
});

new aws.cloudwatch.MetricAlarm("joachim-ci-gc-errors-alarm", {
    alarmDescription: "JOACHIM CI GC Lambda errors",
    namespace: "AWS/Lambda",
    metricName: "Errors",
    statistic: "Sum",
    period: 300,
    evaluationPeriods: 1,
    threshold: 0,
    comparisonOperator: "GreaterThanThreshold",
    dimensions: { FunctionName: gcFn.name },
});

new aws.cloudwatch.MetricAlarm("joachim-ci-spot-errors-alarm", {
    alarmDescription: "JOACHIM CI spot interceptor Lambda errors",
    namespace: "AWS/Lambda",
    metricName: "Errors",
    statistic: "Sum",
    period: 300,
    evaluationPeriods: 1,
    threshold: 0,
    comparisonOperator: "GreaterThanThreshold",
    dimensions: { FunctionName: spotInterceptorFn.name },
});

new aws.cloudwatch.Dashboard("joachim-ci-reliability-dashboard", {
    dashboardName: "joachim-ci-reliability",
    dashboardBody: pulumi
        .all([region, gcFn.name, spotInterceptorFn.name, gcDlq.name])
        .apply(([rgn, gcName, spotName, gcDlqName]) =>
            JSON.stringify({
                widgets: [
                    {
                        type: "metric",
                        width: 12,
                        height: 6,
                        properties: {
                            title: "GC Lambda Invocations/Errors",
                            region: rgn,
                            stat: "Sum",
                            period: 300,
                            metrics: [
                                ["AWS/Lambda", "Invocations", "FunctionName", gcName],
                                [".", "Errors", ".", "."],
                            ],
                        },
                    },
                    {
                        type: "metric",
                        width: 12,
                        height: 6,
                        properties: {
                            title: "Spot Interceptor Invocations/Errors",
                            region: rgn,
                            stat: "Sum",
                            period: 300,
                            metrics: [
                                [
                                    "AWS/Lambda",
                                    "Invocations",
                                    "FunctionName",
                                    spotName,
                                ],
                                [".", "Errors", ".", "."],
                            ],
                        },
                    },
                    {
                        type: "metric",
                        width: 24,
                        height: 6,
                        properties: {
                            title: "GC DLQ Backlog",
                            region: rgn,
                            stat: "Maximum",
                            period: 300,
                            metrics: [
                                [
                                    "AWS/SQS",
                                    "ApproximateNumberOfMessagesVisible",
                                    "QueueName",
                                    gcDlqName,
                                ],
                            ],
                        },
                    },
                ],
            }),
        ),
});

// ---------------------------------------------------------------------------
// 10. Exports (contract for workflows)
// ---------------------------------------------------------------------------

export const vpcId = ciVpc.vpcId;
export const publicSubnetIds = ciVpc.publicSubnetIds;
export const primarySubnetId = ciVpc.publicSubnetIds.apply((ids) => ids[0]);
export const securityGroupId = runnerSecurityGroup.id;
export const sccacheBucketName = cacheBucket.bucket;
export const runnerInstanceProfileArn = runnerInstanceProfile.arn;
export const controllerRoleArn = controllerRole.arn;
export const launchTemplateId = runnerLaunchTemplate.id;
export const gcLambdaName = gcFn.name;
export const gcDlqUrl = gcDlq.url;
export const webhookIngestUrl = webhookApi.apiEndpoint;
export const webhookIngestDlqUrl = ingestDlq.url;
export const spotInterceptorLambdaName = spotInterceptorFn.name;
export const runnerAmiOutput = runnerAmiId;
export const runnerInstanceTypeOutput = runnerInstanceType;

// Repo-vars contract outputs
export const AWS_ROLE_ARN_CI_RUNNER = controllerRole.arn;
export const CI_RUNNER_AMI_ID = runnerAmiId;
export const CI_RUNNER_INSTANCE_TYPE = runnerInstanceType;
export const CI_RUNNER_SUBNET_ID = ciVpc.publicSubnetIds.apply((ids) => ids[0]);
export const CI_RUNNER_SECURITY_GROUP_ID = runnerSecurityGroup.id;
export const CI_RUNNER_INSTANCE_PROFILE = runnerRole.name;
export const SCCACHE_S3_BUCKET = cacheBucket.bucket;
