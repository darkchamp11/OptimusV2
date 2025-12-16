use crate::types::Language;

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
