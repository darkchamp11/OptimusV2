// HTTP route handlers for the Optimus API

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use optimus_common::types::{JobRequest, Language};
use optimus_common::redis;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    pub language: Language,
    pub source_code: String,
    pub test_cases: Vec<TestCaseInput>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct TestCaseInput {
    pub input: String,
    pub expected_output: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_timeout() -> u64 {
    5000
}

fn default_weight() -> u32 {
    10
}

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub job_id: String,
}

/// POST /execute - Submit a job for execution
pub async fn submit_job(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubmitRequest>,
) -> impl IntoResponse {
    // Generate job ID
    let job_id = Uuid::new_v4();

    // Convert test case inputs to internal format
    let test_cases: Vec<optimus_common::types::TestCase> = payload
        .test_cases
        .into_iter()
        .enumerate()
        .map(|(idx, tc)| optimus_common::types::TestCase {
            id: (idx + 1) as u32,
            input: tc.input,
            expected_output: tc.expected_output,
            weight: tc.weight,
        })
        .collect();

    // Create job request
    let job = JobRequest {
        id: job_id,
        language: payload.language,
        source_code: payload.source_code,
        test_cases,
        timeout_ms: payload.timeout_ms,
    };

    // Push to Redis queue
    let mut conn = state.redis.clone();
    match redis::push_job(&mut conn, &job).await {
        Ok(_) => {
            println!(
                "✓ Job {} queued for {} ({} test cases)",
                job_id,
                job.language,
                job.test_cases.len()
            );
            
            (
                StatusCode::CREATED,
                Json(SubmitResponse {
                    job_id: job_id.to_string(),
                }),
            )
        }
        Err(e) => {
            eprintln!("✗ Failed to queue job {}: {}", job_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SubmitResponse {
                    job_id: format!("error: {}", e),
                }),
            )
        }
    }
}

/// GET /status - Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
