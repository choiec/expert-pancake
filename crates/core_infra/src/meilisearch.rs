use meilisearch_sdk::client::Client;
use tokio::time::timeout;

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
}
