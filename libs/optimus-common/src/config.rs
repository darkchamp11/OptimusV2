use std::env;

/// Application configuration
/// Provides defaults with environment variable overrides
#[derive(Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub default_timeout_ms: u64,
    pub max_timeout_ms: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            default_timeout_ms: env::var("DEFAULT_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            max_timeout_ms: env::var("MAX_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30000),
        }
    }

    pub fn new() -> Self {
        Self::from_env()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.default_timeout_ms, 5000);
        assert_eq!(config.max_timeout_ms, 30000);
    }
}
