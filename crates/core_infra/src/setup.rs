use std::time::Duration;

use core_shared::StartupError;

use crate::{MeilisearchService, SurrealDbService, meilisearch, surrealdb};

#[derive(Debug, Clone)]
pub struct InfrastructureSettings {
    pub surrealdb: SurrealDbSettings,
    pub meilisearch: MeilisearchSettings,
}

#[derive(Debug)]
pub struct InfrastructureServices {
    pub surrealdb: SurrealDbService,
    pub meilisearch: MeilisearchService,
}

#[derive(Debug, Clone)]
pub struct SurrealDbSettings {
    pub url: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub connect_timeout: Duration,
    pub readiness_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct MeilisearchSettings {
    pub http_addr: String,
    pub master_key: String,
    pub enabled: bool,
    pub connect_timeout: Duration,
    pub readiness_timeout: Duration,
}

pub async fn bootstrap_infrastructure(
    settings: &InfrastructureSettings,
) -> Result<InfrastructureServices, StartupError> {
    let surrealdb = surrealdb::bootstrap(settings.surrealdb.clone()).await?;
    let meilisearch = meilisearch::bootstrap(settings.meilisearch.clone()).await;

    Ok(InfrastructureServices {
        surrealdb,
        meilisearch,
    })
}
