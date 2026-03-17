use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::Error as MeilisearchError;
use meilisearch_sdk::search::SearchResults;
use meilisearch_sdk::settings::Settings;
use serde::{Serialize, de::DeserializeOwned};
use tokio::time::timeout;

use core_shared::{AppError, AppResult};

use crate::setup::MeilisearchSettings;
use crate::surrealdb::DependencyReport;

#[derive(Debug)]
pub struct MeilisearchService {
    client: Client,
    settings: MeilisearchSettings,
}

pub async fn bootstrap(settings: MeilisearchSettings) -> MeilisearchService {
    let client = Client::new(
        settings.http_addr.clone(),
        Some(settings.master_key.clone()),
    )
    .expect("validated meilisearch configuration must build a client");

    MeilisearchService { client, settings }
}

impl MeilisearchService {
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn enabled(&self) -> bool {
        self.settings.enabled
    }

    pub async fn ensure_index_settings(
        &self,
        index_uid: &str,
        settings: &Settings,
    ) -> AppResult<()> {
        self.ensure_enabled()?;

        let task = self
            .client
            .index(index_uid)
            .set_settings(settings)
            .await
            .map_err(map_meilisearch_error)?;

        self.client
            .wait_for_task(task, None, Some(self.settings.connect_timeout))
            .await
            .map_err(map_meilisearch_error)?;

        Ok(())
    }

    pub async fn add_or_replace_documents<T: Serialize + Send + Sync>(
        &self,
        index_uid: &str,
        primary_key: &str,
        documents: &[T],
    ) -> AppResult<()> {
        self.ensure_enabled()?;

        let task = self
            .client
            .index(index_uid)
            .add_or_replace(documents, Some(primary_key))
            .await
            .map_err(map_meilisearch_error)?;

        self.client
            .wait_for_task(task, None, Some(self.settings.connect_timeout))
            .await
            .map_err(map_meilisearch_error)?;

        Ok(())
    }

    pub async fn search_documents<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        index_uid: &str,
        query: Option<&str>,
        filter: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> AppResult<SearchResults<T>> {
        self.ensure_enabled()?;

        let index = self.client.index(index_uid);
        let mut search = index.search();
        if let Some(query) = query.filter(|value| !value.trim().is_empty()) {
            search.with_query(query);
        }
        search.with_limit(limit);
        search.with_offset(offset);
        search.show_ranking_score = Some(true);
        if let Some(filter) = filter.filter(|value| !value.trim().is_empty()) {
            search.with_filter(filter);
        }

        search.execute().await.map_err(map_meilisearch_error)
    }

    pub async fn get_settings(&self, index_uid: &str) -> AppResult<Settings> {
        self.ensure_enabled()?;

        self.client
            .index(index_uid)
            .get_settings()
            .await
            .map_err(map_meilisearch_error)
    }

    pub async fn readiness(&self) -> DependencyReport {
        if !self.settings.enabled {
            return DependencyReport {
                is_ready: false,
                detail: Some("meilisearch is disabled by configuration".to_string()),
            };
        }

        let readiness = timeout(self.settings.readiness_timeout, self.client.health()).await;

        match readiness {
            Ok(Ok(_)) => DependencyReport {
                is_ready: true,
                detail: None,
            },
            Ok(Err(error)) => DependencyReport {
                is_ready: false,
                detail: Some(error.to_string()),
            },
            Err(_) => DependencyReport {
                is_ready: false,
                detail: Some(format!(
                    "readiness probe timed out after {} ms",
                    self.settings.readiness_timeout.as_millis()
                )),
            },
        }
    }

    fn ensure_enabled(&self) -> AppResult<()> {
        if self.settings.enabled {
            Ok(())
        } else {
            Err(AppError::search_degraded(
                "Meilisearch is disabled by configuration",
            ))
        }
    }
}

fn map_meilisearch_error(error: MeilisearchError) -> AppError {
    AppError::search_degraded(format!("Meilisearch request failed: {error}"))
}
