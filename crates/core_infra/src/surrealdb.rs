use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use core_shared::ApiError;
use mod_memory::bootstrap::DependencyProbe;
use mod_memory::domain::credential::{
    CredentialIndexJob, CredentialIndexJobStatus, RegistrationStatus, StandardCredential,
    conflict_details,
};
use mod_memory::infra::repo::{CredentialRepository, CredentialRepositoryResult};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Default)]
struct StoreState {
    credentials: HashMap<String, StandardCredential>,
    jobs: HashMap<Uuid, CredentialIndexJob>,
}

#[derive(Debug, Default)]
pub struct SurrealCredentialStore {
    state: RwLock<StoreState>,
    ready: RwLock<bool>,
}

impl SurrealCredentialStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: RwLock::new(StoreState::default()),
            ready: RwLock::new(true),
        })
    }

    pub async fn set_ready(&self, ready: bool) {
        *self.ready.write().await = ready;
    }

    pub async fn job_count(&self) -> usize {
        self.state.read().await.jobs.len()
    }

    async fn ensure_ready(&self) -> Result<(), ApiError> {
        if *self.ready.read().await {
            Ok(())
        } else {
            Err(ApiError::service_unavailable(
                "Authoritative credential store is unavailable",
            ))
        }
    }
}

#[async_trait]
impl CredentialRepository for SurrealCredentialStore {
    async fn register(
        &self,
        credential: StandardCredential,
    ) -> Result<CredentialRepositoryResult, ApiError> {
        self.ensure_ready().await?;

        let mut state = self.state.write().await;

        if let Some(existing) = state.credentials.get(&credential.credential_id) {
            if existing.semantic_payload_hash == credential.semantic_payload_hash {
                return Ok(CredentialRepositoryResult {
                    status: RegistrationStatus::Replayed,
                    credential: existing.clone(),
                });
            }

            return Err(ApiError::conflict(
                "Credential id already exists with a semantically different payload",
                Some(conflict_details(existing, &credential)),
            ));
        }

        let credential_id = credential.credential_id.clone();
        state
            .credentials
            .insert(credential_id.clone(), credential.clone());

        let job = CredentialIndexJob {
            job_id: Uuid::new_v4(),
            credential_id,
            status: CredentialIndexJobStatus::Pending,
            retry_count: 0,
            created_at: credential.created_at,
            updated_at: credential.updated_at,
        };

        state.jobs.insert(job.job_id, job);

        Ok(CredentialRepositoryResult {
            status: RegistrationStatus::Created,
            credential,
        })
    }

    async fn get(&self, credential_id: &str) -> Result<Option<StandardCredential>, ApiError> {
        self.ensure_ready().await?;
        Ok(self
            .state
            .read()
            .await
            .credentials
            .get(credential_id)
            .cloned())
    }

    async fn pending_jobs(&self) -> Result<Vec<CredentialIndexJob>, ApiError> {
        self.ensure_ready().await?;
        Ok(self
            .state
            .read()
            .await
            .jobs
            .values()
            .filter(|job| job.status == CredentialIndexJobStatus::Pending)
            .cloned()
            .collect())
    }

    async fn load_for_projection(
        &self,
        credential_id: &str,
    ) -> Result<Option<StandardCredential>, ApiError> {
        self.get(credential_id).await
    }

    async fn mark_job_completed(&self, job_id: Uuid) -> Result<(), ApiError> {
        self.ensure_ready().await?;
        if let Some(job) = self.state.write().await.jobs.get_mut(&job_id) {
            job.status = CredentialIndexJobStatus::Completed;
        }
        Ok(())
    }
}

#[async_trait]
impl DependencyProbe for SurrealCredentialStore {
    async fn is_ready(&self) -> bool {
        *self.ready.read().await
    }
}
