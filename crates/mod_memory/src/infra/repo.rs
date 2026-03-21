use async_trait::async_trait;
use core_shared::ApiError;
use uuid::Uuid;

use crate::domain::credential::{
    CredentialIndexJob, CredentialSearchProjection, CredentialSearchResponse, RegistrationStatus,
    SearchCredentialsQuery, StandardCredential,
};

#[derive(Debug, Clone)]
pub struct CredentialRepositoryResult {
    pub status: RegistrationStatus,
    pub credential: StandardCredential,
}

#[async_trait]
pub trait CredentialRepository: Send + Sync {
    async fn register(
        &self,
        credential: StandardCredential,
    ) -> Result<CredentialRepositoryResult, ApiError>;

    async fn get(&self, credential_id: &str) -> Result<Option<StandardCredential>, ApiError>;

    async fn pending_jobs(&self) -> Result<Vec<CredentialIndexJob>, ApiError>;

    async fn load_for_projection(
        &self,
        credential_id: &str,
    ) -> Result<Option<StandardCredential>, ApiError>;

    async fn mark_job_completed(&self, job_id: Uuid) -> Result<(), ApiError>;
}

#[async_trait]
pub trait ProjectionRepository: Send + Sync {
    async fn upsert(&self, projection: CredentialSearchProjection) -> Result<(), ApiError>;
}

#[async_trait]
pub trait SearchRepository: Send + Sync {
    async fn search(
        &self,
        query: &SearchCredentialsQuery,
    ) -> Result<CredentialSearchResponse, ApiError>;
}
