use std::sync::Arc;

use core_infra::{
    InMemorySurrealDb, InfrastructureSetup, MeilisearchClient, surrealdb::SurrealDbClient,
};
use core_shared::StartupError;
use mod_memory::MemoryModule;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    config: AppConfig,
    memory_module: MemoryModule,
    db: Arc<InMemorySurrealDb>,
    search_enabled: bool,
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
    pub async fn bootstrap(config: AppConfig) -> Result<Self, StartupError> {
        let setup = InfrastructureSetup::bootstrap_in_memory(
            SurrealDbClient::new(
                config.surrealdb.url.clone(),
                config.surrealdb.namespace.clone(),
                config.surrealdb.database.clone(),
                config.surrealdb.username.clone(),
            ),
            MeilisearchClient::new(
                config.meilisearch.http_addr.clone(),
                config.meilisearch.master_key.clone(),
            ),
        );
        let search_enabled = setup.meilisearch.readiness_probe().is_ok();
        let memory_module =
            MemoryModule::fixture(setup.db.clone(), config.timeouts.normalization_timeout);
        Ok(Self {
            config,
            memory_module,
            db: setup.db,
            search_enabled,
        })
    }

    pub fn for_memory_ingest_test(
        config: AppConfig,
        probe_snapshot: ProbeSnapshot,
        db: Arc<InMemorySurrealDb>,
    ) -> Self {
        let memory_module =
            MemoryModule::fixture(db.clone(), config.timeouts.normalization_timeout);
        Self {
            config,
            memory_module,
            db,
            search_enabled: probe_snapshot.search != ProbeStatus::Down,
        }
    }

    pub fn memory_ingest(&self) -> &MemoryModule {
        &self.memory_module
    }

    pub fn max_request_body_bytes(&self) -> usize {
        self.config.limits.max_request_body_bytes
    }

    pub async fn readiness(&self) -> ProbeSnapshot {
        let database = if self.db.readiness_probe().is_ok() {
            ProbeStatus::Ready
        } else {
            ProbeStatus::Down
        };
        let search = if self.search_enabled {
            ProbeStatus::Ready
        } else {
            ProbeStatus::Degraded
        };
        ProbeSnapshot::new(ProbeStatus::Ready, database, search)
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
