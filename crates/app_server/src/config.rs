use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub limits: LimitsConfig,
    pub timeouts: TimeoutConfig,
}

#[derive(Debug, Clone)]
pub struct LimitsConfig {
    pub max_request_body_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub normalization_timeout: Duration,
}

impl AppConfig {
    pub fn for_test() -> Self {
        Self {
            limits: LimitsConfig {
                max_request_body_bytes: 10 * 1024 * 1024,
            },
            timeouts: TimeoutConfig {
                normalization_timeout: Duration::from_secs(30),
            },
        }
    }
}
