#![forbid(unsafe_code)]
//! LLM-based supertagger for JOACHIM prompt injection detection.
//!
//! Takes raw text, sends it to Claude via AWS Bedrock, and parses the
//! structured JSON response into `Vec<TypeAssignment>` for the pregroup
//! core engine.

pub mod client;
pub mod error;
pub mod extract;
pub mod prompt;
pub mod types;

pub use client::{Supertagger, SupertaggerConfig, SupertaggerOutput};
pub use error::SupertaggerError;
