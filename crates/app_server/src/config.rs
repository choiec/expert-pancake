use std::{env, net::SocketAddr, time::Duration};

use core_infra::setup::{InfrastructureSettings, MeilisearchSettings, SurrealDbSettings};
use core_shared::StartupError;

const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_NORMALIZATION_TIMEOUT_SECS: u64 = 30;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_READY_TIMEOUT_MS: u64 = 1_000;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub http: HttpConfig,
    pub infrastructure: InfrastructureSettings,
    pub limits: LimitsConfig,
    pub timeouts: TimeoutConfig,
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub listen_addr: SocketAddr,
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
    pub fn from_env() -> Result<Self, StartupError> {
        let http = HttpConfig {
            listen_addr: parse_socket_addr(&required_env("APP_LISTEN_ADDR")?, "APP_LISTEN_ADDR")?,
        };

        let surrealdb = SurrealDbSettings {
            url: required_env("SURREALDB_URL")?,
            namespace: required_env("SURREALDB_NAMESPACE")?,
            database: required_env("SURREALDB_DATABASE")?,
            username: required_env("SURREALDB_USERNAME")?,
            password: required_env("SURREALDB_PASSWORD")?,
            connect_timeout: parse_duration_ms(
                "SURREALDB_CONNECT_TIMEOUT_MS",
                DEFAULT_CONNECT_TIMEOUT_MS,
            )?,
            readiness_timeout: parse_duration_ms(
                "SURREALDB_READY_TIMEOUT_MS",
                DEFAULT_READY_TIMEOUT_MS,
            )?,
        };

        let meilisearch = MeilisearchSettings {
            http_addr: required_env("MEILI_HTTP_ADDR")?,
            master_key: required_env("MEILI_MASTER_KEY")?,
            enabled: parse_bool("MEMORY_INGEST_ENABLED", true)?,
            connect_timeout: parse_duration_ms(
                "MEILI_CONNECT_TIMEOUT_MS",
                DEFAULT_CONNECT_TIMEOUT_MS,
            )?,
            readiness_timeout: parse_duration_ms(
                "MEILI_READY_TIMEOUT_MS",
                DEFAULT_READY_TIMEOUT_MS,
            )?,
        };

        let limits = LimitsConfig {
            max_request_body_bytes: parse_usize(
                "MEMORY_MAX_REQUEST_BODY_BYTES",
                DEFAULT_MAX_REQUEST_BODY_BYTES,
            )?,
        };

        let timeouts = TimeoutConfig {
            normalization_timeout: parse_duration_secs(
                "MEMORY_NORMALIZATION_TIMEOUT_SECS",
                DEFAULT_NORMALIZATION_TIMEOUT_SECS,
            )?,
        };

        Ok(Self {
            http,
            infrastructure: InfrastructureSettings {
                surrealdb,
                meilisearch,
            },
            limits,
            timeouts,
        })
    }

    pub fn for_test() -> Self {
        Self {
            http: HttpConfig {
                listen_addr: "127.0.0.1:3000".parse().expect("valid socket address"),
            },
            infrastructure: InfrastructureSettings {
                surrealdb: SurrealDbSettings {
                    url: "ws://127.0.0.1:8000/rpc".to_string(),
                    namespace: "memory".to_string(),
                    database: "memory".to_string(),
                    username: "root".to_string(),
                    password: "root".to_string(),
                    connect_timeout: Duration::from_millis(DEFAULT_CONNECT_TIMEOUT_MS),
                    readiness_timeout: Duration::from_millis(DEFAULT_READY_TIMEOUT_MS),
                },
                meilisearch: MeilisearchSettings {
                    http_addr: "http://127.0.0.1:7700".to_string(),
                    master_key: "local-dev-key".to_string(),
                    enabled: true,
                    connect_timeout: Duration::from_millis(DEFAULT_CONNECT_TIMEOUT_MS),
                    readiness_timeout: Duration::from_millis(DEFAULT_READY_TIMEOUT_MS),
                },
            },
            limits: LimitsConfig {
                max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
            },
            timeouts: TimeoutConfig {
                normalization_timeout: Duration::from_secs(DEFAULT_NORMALIZATION_TIMEOUT_SECS),
            },
        }
    }
}

fn required_env(key: &str) -> Result<String, StartupError> {
    env::var(key).map_err(|_| StartupError::MissingEnv {
        key: key.to_string(),
    })
}

fn parse_socket_addr(value: &str, key: &str) -> Result<SocketAddr, StartupError> {
    value
        .parse::<SocketAddr>()
        .map_err(|error| StartupError::InvalidEnv {
            key: key.to_string(),
            value: value.to_string(),
            reason: error.to_string(),
        })
}

fn parse_duration_ms(key: &str, default: u64) -> Result<Duration, StartupError> {
    parse_u64(key, default).map(Duration::from_millis)
}

fn parse_duration_secs(key: &str, default: u64) -> Result<Duration, StartupError> {
    parse_u64(key, default).map(Duration::from_secs)
}

fn parse_u64(key: &str, default: u64) -> Result<u64, StartupError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|error| StartupError::InvalidEnv {
                key: key.to_string(),
                value,
                reason: error.to_string(),
            }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(StartupError::InvalidEnv {
            key: key.to_string(),
            value: "<non-utf8>".to_string(),
            reason: "value is not valid UTF-8".to_string(),
        }),
    }
}

fn parse_usize(key: &str, default: usize) -> Result<usize, StartupError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<usize>()
            .map_err(|error| StartupError::InvalidEnv {
                key: key.to_string(),
                value,
                reason: error.to_string(),
            }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(StartupError::InvalidEnv {
            key: key.to_string(),
            value: "<non-utf8>".to_string(),
            reason: "value is not valid UTF-8".to_string(),
        }),
    }
}

fn parse_bool(key: &str, default: bool) -> Result<bool, StartupError> {
    match env::var(key) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Ok(true),
            "0" | "false" | "no" => Ok(false),
            _ => Err(StartupError::InvalidEnv {
                key: key.to_string(),
                value,
                reason: "expected one of true/false/1/0/yes/no".to_string(),
            }),
        },
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(StartupError::InvalidEnv {
            key: key.to_string(),
            value: "<non-utf8>".to_string(),
            reason: "value is not valid UTF-8".to_string(),
        }),
    }
}
