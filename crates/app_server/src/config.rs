use std::{env, net::SocketAddr, time::Duration};

use core_shared::StartupError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub http: HttpConfig,
    pub surrealdb: SurrealDbConfig,
    pub meilisearch: MeilisearchConfig,
    pub limits: LimitsConfig,
    pub timeouts: TimeoutConfig,
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub listen_addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct SurrealDbConfig {
    pub url: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct MeilisearchConfig {
    pub http_addr: String,
    pub master_key: String,
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
        Ok(Self {
            http: HttpConfig {
                listen_addr: parse_socket_addr("APP_LISTEN_ADDR")?,
            },
            surrealdb: SurrealDbConfig {
                url: required_env("SURREALDB_URL")?,
                namespace: required_env("SURREALDB_NAMESPACE")?,
                database: required_env("SURREALDB_DATABASE")?,
                username: required_env("SURREALDB_USERNAME")?,
                password: required_env("SURREALDB_PASSWORD")?,
            },
            meilisearch: MeilisearchConfig {
                http_addr: required_env("MEILI_HTTP_ADDR")?,
                master_key: required_env("MEILI_MASTER_KEY")?,
            },
            limits: LimitsConfig {
                max_request_body_bytes: parse_usize_with_default(
                    "APP_MAX_REQUEST_BODY_BYTES",
                    10 * 1024 * 1024,
                )?,
            },
            timeouts: TimeoutConfig {
                normalization_timeout: Duration::from_secs(parse_u64_with_default(
                    "APP_NORMALIZATION_TIMEOUT_SECS",
                    30,
                )?),
            },
        })
    }

    pub fn for_test() -> Self {
        Self {
            http: HttpConfig {
                listen_addr: "127.0.0.1:3000"
                    .parse()
                    .expect("test listen addr must be valid"),
            },
            surrealdb: SurrealDbConfig {
                url: "ws://127.0.0.1:8000/rpc".to_owned(),
                namespace: "memory".to_owned(),
                database: "memory".to_owned(),
                username: "root".to_owned(),
                password: "root".to_owned(),
            },
            meilisearch: MeilisearchConfig {
                http_addr: "http://127.0.0.1:7700".to_owned(),
                master_key: "local-dev-key".to_owned(),
            },
            limits: LimitsConfig {
                max_request_body_bytes: 10 * 1024 * 1024,
            },
            timeouts: TimeoutConfig {
                normalization_timeout: Duration::from_secs(30),
            },
        }
    }
}

fn required_env(key: &'static str) -> Result<String, StartupError> {
    env::var(key).map_err(|_| StartupError::MissingEnv {
        key: key.to_owned(),
    })
}

fn parse_socket_addr(key: &'static str) -> Result<SocketAddr, StartupError> {
    let value = required_env(key)?;
    value
        .parse::<SocketAddr>()
        .map_err(|error| StartupError::InvalidEnv {
            key: key.to_owned(),
            value,
            reason: error.to_string(),
        })
}

fn parse_usize_with_default(key: &'static str, default: usize) -> Result<usize, StartupError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<usize>()
            .map_err(|error| StartupError::InvalidEnv {
                key: key.to_owned(),
                value,
                reason: error.to_string(),
            }),
        Err(_) => Ok(default),
    }
}

fn parse_u64_with_default(key: &'static str, default: u64) -> Result<u64, StartupError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|error| StartupError::InvalidEnv {
                key: key.to_owned(),
                value,
                reason: error.to_string(),
            }),
        Err(_) => Ok(default),
    }
}
