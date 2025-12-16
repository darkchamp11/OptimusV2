use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Strongly-typed language enum
/// Start strict - will extend dynamically later
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Java,
    Rust,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Python => write!(f, "python"),
            Language::Java => write!(f, "java"),
            Language::Rust => write!(f, "rust"),
        }
    }
}

/// Job Input (Immutable)
/// A job is write-once - never mutate input fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub id: Uuid,
    pub language: Language,
    pub source_code: String,
    pub stdin: Option<String>,
    pub timeout_ms: u64,
}

/// Job State Machine
/// Explicitly models lifecycle states
/// Backs: GET /job/{id}, retry logic, metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
}

/// Execution Output
/// Written by workers, read by API, stored in Redis/object storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_serialization() {
        let lang = Language::Python;
        let json = serde_json::to_string(&lang).unwrap();
        assert_eq!(json, "\"python\"");
        
        let deserialized: Language = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Language::Python);
    }

    #[test]
    fn test_job_request_serialization() {
        let job = JobRequest {
            id: Uuid::new_v4(),
            language: Language::Java,
            source_code: "public class Main {}".to_string(),
            stdin: Some("test input".to_string()),
            timeout_ms: 5000,
        };
        
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: JobRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.language, Language::Java);
        assert_eq!(deserialized.timeout_ms, 5000);
    }

    #[test]
    fn test_job_status_serialization() {
        let status = JobStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");
    }

    #[test]
    fn test_execution_result_structure() {
        let result = ExecutionResult {
            job_id: Uuid::new_v4(),
            status: JobStatus::Completed,
            stdout: "Hello, World!".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            execution_time_ms: 123,
        };
        
        assert_eq!(result.status, JobStatus::Completed);
        assert_eq!(result.exit_code, Some(0));
    }
}
