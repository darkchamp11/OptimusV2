use crate::types::{Language, JobRequest};
use redis::{AsyncCommands, RedisResult};

/// Redis queue semantics - defines only semantics, not runtime logic
/// Ensures API and worker never drift, Redis keys are deterministic,
/// and KEDA scaling remains predictable

pub const QUEUE_PREFIX: &str = "optimus:queue";
pub const RESULT_PREFIX: &str = "optimus:result";
pub const STATUS_PREFIX: &str = "optimus:status";

/// Generate deterministic queue name for a language
pub fn queue_name(language: &Language) -> String {
    format!("{}:{}", QUEUE_PREFIX, language)
}

/// Generate result key for a job
pub fn result_key(job_id: &uuid::Uuid) -> String {
    format!("{}:{}", RESULT_PREFIX, job_id)
}

/// Generate status key for a job
pub fn status_key(job_id: &uuid::Uuid) -> String {
    format!("{}:{}", STATUS_PREFIX, job_id)
}

/// Push a job to the language-specific queue
/// Uses RPUSH for FIFO semantics
pub async fn push_job(
    conn: &mut redis::aio::ConnectionManager,
    job: &JobRequest,
) -> RedisResult<()> {
    let queue = queue_name(&job.language);
    let payload = serde_json::to_string(job)
        .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "serialization error", e.to_string())))?;
    
    conn.rpush(&queue, payload).await
}

/// Pop a job from the language-specific queue
/// Uses BLPOP with timeout for graceful shutdown
pub async fn pop_job(
    conn: &mut redis::aio::ConnectionManager,
    language: &Language,
    timeout_seconds: f64,
) -> RedisResult<Option<JobRequest>> {
    let queue = queue_name(language);
    let result: Option<(String, String)> = conn.blpop(&queue, timeout_seconds).await?;
    
    match result {
        Some((_key, payload)) => {
            let job: JobRequest = serde_json::from_str(&payload)
                .map_err(|e| redis::RedisError::from((redis::ErrorKind::TypeError, "deserialization error", e.to_string())))?;
            Ok(Some(job))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Language;
    use uuid::Uuid;

    #[test]
    fn test_queue_naming() {
        assert_eq!(queue_name(&Language::Python), "optimus:queue:python");
        assert_eq!(queue_name(&Language::Java), "optimus:queue:java");
        assert_eq!(queue_name(&Language::Rust), "optimus:queue:rust");
    }

    #[test]
    fn test_result_key_deterministic() {
        let id = Uuid::new_v4();
        let key1 = result_key(&id);
        let key2 = result_key(&id);
        assert_eq!(key1, key2);
        assert!(key1.starts_with("optimus:result:"));
    }

    #[test]
    fn test_status_key_format() {
        let id = Uuid::new_v4();
        let key = status_key(&id);
        assert!(key.starts_with("optimus:status:"));
        assert!(key.contains(&id.to_string()));
    }
}
