use std::sync::Arc;

use async_trait::async_trait;
use core_shared::ApiError;
use mod_memory::bootstrap::{CredentialModule, ProjectionSync};
use mod_memory::infra::repo::{CredentialRepository, ProjectionRepository, SearchRepository};

use crate::meilisearch::MeiliCredentialSearch;
use crate::surrealdb::SurrealCredentialStore;

#[derive(Clone)]
pub struct InfraHandles {
    pub authoritative_store: Arc<SurrealCredentialStore>,
    pub search_store: Arc<MeiliCredentialSearch>,
}

#[derive(Clone)]
pub struct InfraBundle {
    pub module: CredentialModule,
    pub handles: InfraHandles,
    pub projection_sync: Arc<dyn ProjectionSync>,
}

pub fn build_infra_bundle() -> InfraBundle {
    let authoritative_store = SurrealCredentialStore::new();
    let search_store = MeiliCredentialSearch::new();
    let sync = Arc::new(InMemoryProjectionSync {
        authoritative_store: authoritative_store.clone(),
        search_store: search_store.clone(),
    });

    let module = CredentialModule::new(
        authoritative_store.clone() as Arc<dyn CredentialRepository>,
        search_store.clone() as Arc<dyn SearchRepository>,
        sync.clone(),
        authoritative_store.clone(),
        search_store.clone(),
    );

    InfraBundle {
        module,
        handles: InfraHandles {
            authoritative_store,
            search_store,
        },
        projection_sync: sync,
    }
}

struct InMemoryProjectionSync {
    authoritative_store: Arc<SurrealCredentialStore>,
    search_store: Arc<MeiliCredentialSearch>,
}

#[async_trait]
impl ProjectionSync for InMemoryProjectionSync {
    async fn sync_pending(&self) -> Result<(), ApiError> {
        let jobs = self.authoritative_store.pending_jobs().await?;
        for job in jobs {
            let Some(credential) = self
                .authoritative_store
                .load_for_projection(&job.credential_id)
                .await?
            else {
                continue;
            };

            self.search_store.upsert(credential.to_projection()).await?;
            self.authoritative_store
                .mark_job_completed(job.job_id)
                .await?;
        }

        Ok(())
    }
}
