use std::sync::Arc;

use async_trait::async_trait;
use core_shared::ApiError;
use mod_memory::bootstrap::DependencyProbe;
use mod_memory::domain::credential::{
    CredentialSearchProjection, CredentialSearchResponse, SearchCredentialsQuery, extract_stringish,
};
use mod_memory::infra::repo::{ProjectionRepository, SearchRepository};
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MeiliCredentialSearch {
    projections: RwLock<Vec<CredentialSearchProjection>>,
    ready: RwLock<bool>,
}

impl MeiliCredentialSearch {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            projections: RwLock::new(Vec::new()),
            ready: RwLock::new(true),
        })
    }

    pub async fn set_ready(&self, ready: bool) {
        *self.ready.write().await = ready;
    }

    pub async fn projection_count(&self) -> usize {
        self.projections.read().await.len()
    }

    async fn ensure_ready(&self) -> Result<(), ApiError> {
        if *self.ready.read().await {
            Ok(())
        } else {
            Err(ApiError::service_unavailable(
                "Credential search projection is unavailable",
            ))
        }
    }
}

#[async_trait]
impl ProjectionRepository for MeiliCredentialSearch {
    async fn upsert(&self, projection: CredentialSearchProjection) -> Result<(), ApiError> {
        self.ensure_ready().await?;
        let mut projections = self.projections.write().await;

        if let Some(existing) = projections
            .iter_mut()
            .find(|item| item.credential_id == projection.credential_id)
        {
            *existing = projection;
            return Ok(());
        }

        projections.push(projection);
        Ok(())
    }
}

#[async_trait]
impl SearchRepository for MeiliCredentialSearch {
    async fn search(
        &self,
        query: &SearchCredentialsQuery,
    ) -> Result<CredentialSearchResponse, ApiError> {
        self.ensure_ready().await?;
        let normalized_q = query.q.as_ref().map(|value| value.to_lowercase());

        let filtered = self
            .projections
            .read()
            .await
            .iter()
            .filter(|projection| {
                query
                    .family
                    .is_none_or(|family| projection.family == family)
                    && query
                        .issuer_id
                        .as_ref()
                        .is_none_or(|issuer_id| projection.issuer_id.as_ref() == Some(issuer_id))
                    && normalized_q.as_ref().is_none_or(|needle| {
                        let name = projection
                            .name
                            .as_ref()
                            .and_then(extract_stringish)
                            .unwrap_or_default()
                            .to_lowercase();
                        let issuer = projection
                            .issuer
                            .as_ref()
                            .and_then(extract_stringish)
                            .unwrap_or_default()
                            .to_lowercase();
                        let preview = projection
                            .preview
                            .as_deref()
                            .unwrap_or_default()
                            .to_lowercase();

                        name.contains(needle) || issuer.contains(needle) || preview.contains(needle)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();

        let items = filtered
            .iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|projection| projection.to_hit(normalized_q.as_ref().map(|_| 1.0)))
            .collect::<Vec<_>>();

        Ok(CredentialSearchResponse {
            items,
            limit: query.limit,
            offset: query.offset,
        })
    }
}

#[async_trait]
impl DependencyProbe for MeiliCredentialSearch {
    async fn is_ready(&self) -> bool {
        *self.ready.read().await
    }
}
