use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use core_infra::MeilisearchService;
use core_infra::surrealdb::{
    InMemorySurrealDb, PersistedIndexJobRecord, ProjectionRehydrationBundle, SurrealDbService,
};
use core_shared::{AppError, AppResult};

use crate::infra::indexer::{
    IndexingJob, OutboxStatus, ProjectionIndexPort, ProjectionInput, ProjectionSearchHit,
    ProjectionSearchQuery, ProjectionSearchResult,
};
use crate::infra::repo::IndexingOutboxRepository;

pub const MEMORY_ITEMS_INDEX_UID: &str = "memory_items_v1";
const PRIMARY_KEY: &str = "urn";
const MAX_CONTENT_PREVIEW_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryItemProjectionDocument {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub document_type: String,
    pub content_preview: String,
    pub content_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionIndexSettingsSnapshot {
    pub index_uid: String,
    pub primary_key: String,
    pub filterable_attributes: Vec<String>,
    pub sortable_attributes: Vec<String>,
    pub searchable_attributes: Vec<String>,
    pub displayed_attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeMeiliProjectionIndex {
    service: Arc<MeilisearchService>,
}

impl RuntimeMeiliProjectionIndex {
    pub fn new(service: Arc<MeilisearchService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMeiliProjectionIndex {
    state: Arc<Mutex<InMemoryProjectionState>>,
}

#[derive(Debug, Default)]
struct InMemoryProjectionState {
    available: bool,
    settings: Option<ProjectionIndexSettingsSnapshot>,
    documents: BTreeMap<String, MemoryItemProjectionDocument>,
}

impl InMemoryMeiliProjectionIndex {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemoryProjectionState {
                available: true,
                settings: None,
                documents: BTreeMap::new(),
            })),
        }
    }

    pub fn set_available(&self, available: bool) {
        self.state
            .lock()
            .expect("projection state poisoned")
            .available = available;
    }

    pub fn settings_snapshot(&self) -> Option<ProjectionIndexSettingsSnapshot> {
        self.state
            .lock()
            .expect("projection state poisoned")
            .settings
            .clone()
    }

    pub fn documents(&self) -> Vec<MemoryItemProjectionDocument> {
        self.state
            .lock()
            .expect("projection state poisoned")
            .documents
            .values()
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryIndexingOutboxRepository {
    db: Arc<InMemorySurrealDb>,
}

impl InMemoryIndexingOutboxRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }
}

#[derive(Clone)]
pub struct RuntimeIndexingOutboxRepository {
    db: Arc<SurrealDbService>,
}

impl RuntimeIndexingOutboxRepository {
    pub fn new(db: Arc<SurrealDbService>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProjectionIndexPort for RuntimeMeiliProjectionIndex {
    async fn ensure_index(&self) -> AppResult<()> {
        self.service
            .ensure_index_settings(MEMORY_ITEMS_INDEX_UID, &runtime_settings())
            .await
    }

    async fn upsert(&self, documents: &[ProjectionInput]) -> AppResult<()> {
        self.ensure_index().await?;

        let documents = documents
            .iter()
            .map(MemoryItemProjectionDocument::from)
            .collect::<Vec<_>>();
        self.service
            .add_or_replace_documents(MEMORY_ITEMS_INDEX_UID, PRIMARY_KEY, &documents)
            .await
    }

    async fn search(&self, query: &ProjectionSearchQuery) -> AppResult<ProjectionSearchResult> {
        let filter = build_filter(query);
        let response = self
            .service
            .search_documents::<MemoryItemProjectionDocument>(
                MEMORY_ITEMS_INDEX_UID,
                query.query.as_deref(),
                filter.as_deref(),
                query.limit,
                query.offset,
            )
            .await?;

        Ok(ProjectionSearchResult {
            total: response.estimated_total_hits.unwrap_or(response.hits.len()),
            limit: response.limit.unwrap_or(query.limit),
            offset: response.offset.unwrap_or(query.offset),
            items: response
                .hits
                .into_iter()
                .map(|hit| ProjectionSearchHit {
                    urn: hit.result.urn,
                    source_id: hit.result.source_id,
                    sequence: hit.result.sequence,
                    document_type: hit.result.document_type,
                    content_preview: hit.result.content_preview,
                    score: hit.ranking_score.map(|score| score as f32),
                })
                .collect(),
        })
    }

    async fn is_available(&self) -> bool {
        self.service.readiness().await.is_ready
    }
}

#[async_trait]
impl ProjectionIndexPort for InMemoryMeiliProjectionIndex {
    async fn ensure_index(&self) -> AppResult<()> {
        let mut state = self.state.lock().expect("projection state poisoned");
        ensure_available(&state)?;
        state.settings = Some(settings_snapshot());
        Ok(())
    }

    async fn upsert(&self, documents: &[ProjectionInput]) -> AppResult<()> {
        let mut state = self.state.lock().expect("projection state poisoned");
        ensure_available(&state)?;
        state.settings = Some(settings_snapshot());
        for document in documents.iter().map(MemoryItemProjectionDocument::from) {
            state.documents.insert(document.urn.clone(), document);
        }
        Ok(())
    }

    async fn search(&self, query: &ProjectionSearchQuery) -> AppResult<ProjectionSearchResult> {
        let state = self.state.lock().expect("projection state poisoned");
        ensure_available(&state)?;

        let query_text = query.query.as_deref().map(str::to_ascii_lowercase);
        let mut items = state
            .documents
            .values()
            .filter(|document| {
                query
                    .source_id
                    .is_none_or(|source_id| source_id == document.source_id)
                    && query
                        .document_type
                        .as_ref()
                        .is_none_or(|document_type| document_type == &document.document_type)
                    && query_text.as_ref().is_none_or(|needle| {
                        document
                            .content_preview
                            .to_ascii_lowercase()
                            .contains(needle)
                            || document.urn.to_ascii_lowercase().contains(needle)
                            || document.source_id.to_string().contains(needle)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();

        items.sort_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .reverse()
                .then_with(|| left.sequence.cmp(&right.sequence))
                .then_with(|| left.urn.cmp(&right.urn))
        });

        let total = items.len();
        let paged = items
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|document| ProjectionSearchHit {
                urn: document.urn,
                source_id: document.source_id,
                sequence: document.sequence,
                document_type: document.document_type,
                content_preview: document.content_preview,
                score: None,
            })
            .collect();

        Ok(ProjectionSearchResult {
            total,
            limit: query.limit,
            offset: query.offset,
            items: paged,
        })
    }

    async fn is_available(&self) -> bool {
        self.state
            .lock()
            .expect("projection state poisoned")
            .available
    }
}

#[async_trait]
impl IndexingOutboxRepository for InMemoryIndexingOutboxRepository {
    async fn claim_next_job(&self, now: OffsetDateTime) -> AppResult<Option<IndexingJob>> {
        Ok(self.db.claim_next_index_job(now).map(job_from_record))
    }

    async fn rehydrate_projection_inputs(
        &self,
        source_id: Uuid,
    ) -> AppResult<Vec<ProjectionInput>> {
        let bundle = self
            .db
            .rehydrate_projection(source_id)
            .ok_or_else(|| AppError::not_found(format!("source '{}' was not found", source_id)))?;
        Ok(projection_inputs_from_bundle(bundle))
    }

    async fn update_job(&self, job: &IndexingJob) -> AppResult<()> {
        self.db.update_index_job(job_to_record(job))
    }
}

#[async_trait]
impl IndexingOutboxRepository for RuntimeIndexingOutboxRepository {
    async fn claim_next_job(&self, now: OffsetDateTime) -> AppResult<Option<IndexingJob>> {
        Ok(self
            .db
            .claim_next_index_job(now)
            .await?
            .map(job_from_record))
    }

    async fn rehydrate_projection_inputs(
        &self,
        source_id: Uuid,
    ) -> AppResult<Vec<ProjectionInput>> {
        let bundle = self
            .db
            .rehydrate_projection(source_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("source '{}' was not found", source_id)))?;
        Ok(projection_inputs_from_bundle(bundle))
    }

    async fn update_job(&self, job: &IndexingJob) -> AppResult<()> {
        self.db.update_index_job(job_to_record(job)).await
    }
}

impl From<&ProjectionInput> for MemoryItemProjectionDocument {
    fn from(value: &ProjectionInput) -> Self {
        Self {
            urn: value.urn.clone(),
            source_id: value.source_id,
            sequence: value.sequence,
            document_type: value.document_type.clone(),
            content_preview: value.content_preview.clone(),
            content_hash: value.content_hash.clone(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn runtime_settings() -> meilisearch_sdk::settings::Settings {
    meilisearch_sdk::settings::Settings::new()
        .with_filterable_attributes(["source_id", "document_type"])
        .with_sortable_attributes(["sequence", "created_at", "updated_at"])
        .with_searchable_attributes(["content_preview", "urn", "source_id", "content_hash"])
        .with_displayed_attributes([
            "urn",
            "source_id",
            "sequence",
            "document_type",
            "content_preview",
            "content_hash",
            "created_at",
            "updated_at",
        ])
}

fn settings_snapshot() -> ProjectionIndexSettingsSnapshot {
    ProjectionIndexSettingsSnapshot {
        index_uid: MEMORY_ITEMS_INDEX_UID.to_owned(),
        primary_key: PRIMARY_KEY.to_owned(),
        filterable_attributes: vec!["source_id".to_owned(), "document_type".to_owned()],
        sortable_attributes: vec![
            "sequence".to_owned(),
            "created_at".to_owned(),
            "updated_at".to_owned(),
        ],
        searchable_attributes: vec![
            "content_preview".to_owned(),
            "urn".to_owned(),
            "source_id".to_owned(),
            "content_hash".to_owned(),
        ],
        displayed_attributes: vec![
            "urn".to_owned(),
            "source_id".to_owned(),
            "sequence".to_owned(),
            "document_type".to_owned(),
            "content_preview".to_owned(),
            "content_hash".to_owned(),
            "created_at".to_owned(),
            "updated_at".to_owned(),
        ],
    }
}

fn ensure_available(state: &InMemoryProjectionState) -> AppResult<()> {
    if state.available {
        Ok(())
    } else {
        Err(AppError::search_degraded(
            "Meilisearch projection is unavailable",
        ))
    }
}

fn build_filter(query: &ProjectionSearchQuery) -> Option<String> {
    let mut filters = Vec::new();
    if let Some(source_id) = query.source_id {
        filters.push(format!("source_id = \"{source_id}\""));
    }
    if let Some(document_type) = query.document_type.as_ref() {
        filters.push(format!("document_type = \"{document_type}\""));
    }

    if filters.is_empty() {
        None
    } else {
        Some(filters.join(" AND "))
    }
}

fn job_from_record(record: PersistedIndexJobRecord) -> IndexingJob {
    IndexingJob {
        job_id: record.job_id,
        source_id: record.source_id,
        status: OutboxStatus::from_str(&record.status).unwrap_or(OutboxStatus::Retryable),
        retry_count: record.retry_count,
        last_error: record.last_error,
        available_at: record.available_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn job_to_record(job: &IndexingJob) -> PersistedIndexJobRecord {
    PersistedIndexJobRecord {
        job_id: job.job_id,
        source_id: job.source_id,
        status: job.status.as_str().to_owned(),
        retry_count: job.retry_count,
        last_error: job.last_error.clone(),
        available_at: job.available_at,
        created_at: job.created_at,
        updated_at: job.updated_at,
    }
}

fn projection_inputs_from_bundle(bundle: ProjectionRehydrationBundle) -> Vec<ProjectionInput> {
    let document_type = bundle.source.document_type;
    bundle
        .memory_items
        .into_iter()
        .map(|item| ProjectionInput {
            urn: item.urn,
            source_id: item.source_id,
            sequence: item.sequence,
            document_type: document_type.clone(),
            content_preview: build_content_preview(&item.content),
            content_hash: item.content_hash,
            created_at: item.created_at,
            updated_at: item.updated_at,
        })
        .collect()
}

fn build_content_preview(content: &str) -> String {
    content.chars().take(MAX_CONTENT_PREVIEW_CHARS).collect()
}

pub fn backoff_available_at(
    now: OffsetDateTime,
    retry_count: u32,
    retry_delay: std::time::Duration,
) -> OffsetDateTime {
    let delay = retry_delay
        .checked_mul(retry_count.max(1))
        .unwrap_or(retry_delay);
    let seconds = delay.as_secs().min(i64::MAX as u64) as i64;
    let nanos = i64::from(delay.subsec_nanos());
    now + TimeDuration::seconds(seconds) + TimeDuration::nanoseconds(nanos)
}
