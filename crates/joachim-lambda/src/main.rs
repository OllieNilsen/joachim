//! Lambda handler for the JOACHIM detection API.
//!
//! Receives `POST /detect` with `{"text": "..."}`, runs the full detection
//! pipeline (supertag → parse → scope check), and returns a verdict JSON.
//!
//! Authentication is handled at the API Gateway layer (Cognito JWT authorizer).
//! The Lambda does not verify tokens itself.

use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Deserialize, Serialize};

use joachim_core::parser::{parse, ParseInput};
use joachim_core::scope::{check_scope, ScopePattern, Verdict};
use joachim_supertag::error::SupertaggerError;
use joachim_supertag::{Supertagger, SupertaggerConfig};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DetectRequest {
    text: String,
}

#[derive(Serialize)]
struct DetectResponse {
    verdict: &'static str,
    violations: Vec<ViolationDto>,
    prompt_version: String,
    timed_out: bool,
}

#[derive(Serialize)]
struct ViolationDto {
    pattern: String,
    source_pos: u16,
    target_pos: u16,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    message: String,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

async fn handler(event: Request, tagger: &Supertagger) -> Result<Response<Body>, Error> {
    // Parse request body.
    let body = match event.body() {
        Body::Text(s) => s.clone(),
        Body::Binary(b) => String::from_utf8_lossy(b).to_string(),
        Body::Empty => {
            return json_error(400, "bad_request", "request body is empty");
        }
    };

    let req: DetectRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return json_error(400, "bad_request", &format!("invalid JSON: {e}"));
        }
    };

    // Run the detection pipeline.
    let output = match tagger.supertag(&req.text).await {
        Ok(o) => o,
        Err(SupertaggerError::InputTooLong { limit, actual }) => {
            return json_error(
                400,
                "input_too_long",
                &format!("input is {actual} chars, max is {limit}"),
            );
        }
        Err(e) => {
            return json_error(502, "supertagger_error", &e.to_string());
        }
    };

    let graph = parse(&ParseInput(output.assignments.clone()), None);
    let verdict = check_scope(&graph, &output.assignments);

    // Build response.
    let (verdict_str, violations) = match &verdict {
        Verdict::Injection { violations } => (
            "Injection",
            violations
                .iter()
                .map(|v| ViolationDto {
                    pattern: match v.pattern {
                        ScopePattern::DirOverAg => "DirOverAg".to_owned(),
                        ScopePattern::RoleOverAg => "RoleOverAg".to_owned(),
                    },
                    source_pos: v.source_pos,
                    target_pos: v.target_pos,
                })
                .collect(),
        ),
        Verdict::Clean => ("Clean", Vec::new()),
        _ => ("Unknown", Vec::new()),
    };

    let resp = DetectResponse {
        verdict: verdict_str,
        violations,
        prompt_version: output.prompt_version.to_owned(),
        timed_out: graph.timed_out,
    };

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Body::Text(serde_json::to_string(&resp)?))?)
}

fn json_error(
    status: u16,
    error: &'static str,
    message: &str,
) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(&ErrorResponse {
        error,
        message: message.to_owned(),
    })?;
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(body))?)
}

// ---------------------------------------------------------------------------
// Main: pre-init Supertagger, then start Lambda runtime
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let config = SupertaggerConfig::default();
    let tagger = Supertagger::new(config)
        .await
        .expect("Failed to initialize Supertagger — check AWS credentials");

    run(service_fn(|event| handler(event, &tagger))).await
}

// ---------------------------------------------------------------------------
// Unit tests (request/response serialization only — no Bedrock)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_request_deserializes() {
        let json = r#"{"text": "hello world"}"#;
        let req: DetectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.text, "hello world");
    }

    #[test]
    fn detect_request_missing_text_fails() {
        let json = r#"{"other": "field"}"#;
        assert!(serde_json::from_str::<DetectRequest>(json).is_err());
    }

    #[test]
    fn detect_request_malformed_json_fails() {
        assert!(serde_json::from_str::<DetectRequest>("not json").is_err());
    }

    #[test]
    fn detect_response_serializes() {
        let resp = DetectResponse {
            verdict: "Clean",
            violations: vec![],
            prompt_version: "v1".to_owned(),
            timed_out: false,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"verdict\":\"Clean\""));
        assert!(json.contains("\"timed_out\":false"));
    }

    #[test]
    fn detect_response_with_violations_serializes() {
        let resp = DetectResponse {
            verdict: "Injection",
            violations: vec![ViolationDto {
                pattern: "DirOverAg".to_owned(),
                source_pos: 0,
                target_pos: 2,
            }],
            prompt_version: "v1".to_owned(),
            timed_out: false,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("DirOverAg"));
        assert!(json.contains("\"source_pos\":0"));
    }

    #[test]
    fn error_response_serializes() {
        let resp = ErrorResponse {
            error: "bad_request",
            message: "missing text field".to_owned(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\":\"bad_request\""));
    }
}
