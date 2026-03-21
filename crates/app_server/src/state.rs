use std::sync::Arc;

use core_infra::InMemorySurrealDb;
use core_shared::StartupError;
use mod_memory::MemoryModule;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    config: AppConfig,
    memory_module: MemoryModule,
    probe_snapshot: ProbeSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeStatus {
    Ready,
    Degraded,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeSnapshot {
    pub service: ProbeStatus,
    pub database: ProbeStatus,
    pub search: ProbeStatus,
}

impl AppState {
    pub async fn bootstrap(_config: AppConfig) -> Result<Self, StartupError> {
        Err(StartupError::MissingEnv {
            key: "runtime bootstrap is not configured in this slice".to_owned(),
        })
    }

    pub fn for_memory_ingest_test(
        config: AppConfig,
        probe_snapshot: ProbeSnapshot,
        db: Arc<InMemorySurrealDb>,
    ) -> Self {
        let memory_module = MemoryModule::fixture(db, config.timeouts.normalization_timeout);
        Self {
            config,
            memory_module,
            probe_snapshot,
        }
    }

    pub fn memory_ingest(&self) -> &MemoryModule {
        &self.memory_module
    }

    pub fn max_request_body_bytes(&self) -> usize {
        self.config.limits.max_request_body_bytes
    }

    pub async fn readiness(&self) -> ProbeSnapshot {
        self.probe_snapshot
    }
}

impl ProbeSnapshot {
    pub const fn ready() -> Self {
        Self {
            service: ProbeStatus::Ready,
            database: ProbeStatus::Ready,
            search: ProbeStatus::Ready,
        }
    }

    pub const fn new(service: ProbeStatus, database: ProbeStatus, search: ProbeStatus) -> Self {
        Self {
            service,
            database,
            search,
        }
    }
}
