use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use core_infra::surrealdb::{
    InMemorySurrealDb, PersistedIndexJobRecord, PersistedMemoryItemRecord, PersistedSourceBundle,
    PersistedSourceRecord, SurrealDbService,
};
use core_shared::{AppError, AppResult, MemoryItemUrn};

use crate::domain::memory_item::{MemoryItem, MemoryUnitType};
use crate::domain::source::{DocumentType, NewSource, Source};
use crate::infra::indexer::{OutboxStatus, derive_public_indexing_status};
use crate::infra::repo::{SourceBundle, SourceCreateOrReplay, SourceRepository};

type SearchAvailabilityProbe = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug, Clone)]
/// Fixture-backed repository used by storage contract tests.
pub struct SurrealSourceRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealSourceRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }
}

#[derive(Clone)]
/// Production/runtime repository backed by the real SurrealDB client.
pub struct RuntimeSurrealSourceRepository {
    db: Arc<SurrealDbService>,
    search_available: SearchAvailabilityProbe,
}

impl RuntimeSurrealSourceRepository {
    pub fn new(db: Arc<SurrealDbService>, search_available: SearchAvailabilityProbe) -> Self {
        Self {
            db,
            search_available,
        }
    }

    fn search_available(&self) -> bool {
        (self.search_available)()
    }
}

#[async_trait]
impl SourceRepository for SurrealSourceRepository {
    async fn prepare_create_or_replay(&self, source: NewSource) -> AppResult<SourceCreateOrReplay> {
        if let Some(existing) = self.db.lookup_source_by_external_id(&source.external_id) {
            let existing_hash = existing
                .source
                .source_metadata
                .pointer("/system/semantic_payload_hash")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if existing_hash == source.semantic_payload_hash() {
                return Ok(SourceCreateOrReplay::Replay(bundle_from_records(
                    existing,
                    self.db.search_available(),
                )?));
            }
            return Err(AppError::conflict(format!(
                "external_id '{}' is already registered with a different semantic payload",
                source.external_id
            )));
        }
        Ok(SourceCreateOrReplay::Create(source))
    }
}

#[async_trait]
impl SourceRepository for RuntimeSurrealSourceRepository {
    async fn prepare_create_or_replay(&self, source: NewSource) -> AppResult<SourceCreateOrReplay> {
        if let Some(existing) =
            fetch_source_bundle_by_external_id(self.db.as_ref(), &source.external_id).await?
        {
            let existing_hash = existing
                .source
                .source_metadata
                .pointer("/system/semantic_payload_hash")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if existing_hash == source.semantic_payload_hash() {
                return Ok(SourceCreateOrReplay::Replay(bundle_from_records(
                    existing,
                    self.search_available(),
                )?));
            }
            return Err(AppError::conflict(format!(
                "external_id '{}' is already registered with a different semantic payload",
                source.external_id
            )));
        }
        Ok(SourceCreateOrReplay::Create(source))
    }
}

pub(crate) async fn fetch_source_bundle_by_external_id(
    db: &SurrealDbService,
    external_id: &str,
) -> AppResult<Option<PersistedSourceBundle>> {
    let Some(source) = db.find_source_by_external_id(external_id).await? else {
        return Ok(None);
    };
    fetch_source_bundle_by_source_id(db, source.source_id).await
}

pub(crate) async fn fetch_source_bundle_by_source_id(
    db: &SurrealDbService,
    source_id: uuid::Uuid,
) -> AppResult<Option<PersistedSourceBundle>> {
    let Some(source) = db.find_source_by_source_id(source_id).await? else {
        return Ok(None);
    };
    let memory_items = db.find_memory_items_by_source_id(source_id).await?;
    let latest_job = db.find_latest_index_job_by_source_id(source_id).await?;
    Ok(Some(PersistedSourceBundle {
        source,
        memory_items,
        latest_job,
    }))
}

pub(crate) async fn fetch_memory_item_with_source(
    db: &SurrealDbService,
    urn: &str,
) -> AppResult<Option<(PersistedMemoryItemRecord, PersistedSourceRecord)>> {
    let Some(memory_item) = db.find_memory_item_by_urn(urn).await? else {
        return Ok(None);
    };
    let Some(source) = db.find_source_by_source_id(memory_item.source_id).await? else {
        return Err(AppError::internal(format!(
            "source '{}' is missing for memory item '{}'",
            memory_item.source_id, urn
        )));
    };
    Ok(Some((memory_item, source)))
}

pub(crate) fn bundle_from_records(
    bundle: PersistedSourceBundle,
    search_available: bool,
) -> AppResult<SourceBundle> {
    let indexing_status = derive_public_indexing_status(
        bundle
            .latest_job
            .as_ref()
            .and_then(|job| OutboxStatus::from_str(&job.status)),
        search_available,
    );
    Ok(SourceBundle {
        source: source_from_record(bundle.source)?,
        memory_items: bundle
            .memory_items
            .into_iter()
            .map(memory_item_from_record)
            .collect::<AppResult<Vec<_>>>()?,
        indexing_status,
    })
}

pub(crate) fn source_to_record(source: &NewSource) -> PersistedSourceRecord {
    PersistedSourceRecord {
        source_id: source.source_id,
        external_id: source.external_id.clone(),
        title: source.title.clone(),
        summary: source.summary.clone(),
        document_type: source.document_type.as_str().to_owned(),
        source_metadata: source.source_metadata.clone(),
        created_at: source.created_at,
        updated_at: source.updated_at,
    }
}

pub(crate) fn memory_item_to_record(item: &MemoryItem) -> PersistedMemoryItemRecord {
    PersistedMemoryItemRecord {
        urn: item.urn.to_string(),
        source_id: item.source_id,
        sequence: item.sequence,
        unit_type: item.unit_type.as_str().to_owned(),
        start_offset: item.start_offset,
        end_offset: item.end_offset,
        version: item.version.clone(),
        content: item.content.clone(),
        content_hash: item.content_hash.clone(),
        item_metadata: item.item_metadata.clone(),
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

pub(crate) fn job_to_record(job: &crate::infra::indexer::IndexingJob) -> PersistedIndexJobRecord {
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

pub(crate) fn source_from_record(record: PersistedSourceRecord) -> AppResult<Source> {
    Ok(Source {
        source_id: record.source_id,
        external_id: record.external_id,
        title: record.title,
        summary: record.summary,
        document_type: document_type_from_str(&record.document_type)?,
        source_metadata: record.source_metadata,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

pub(crate) fn memory_item_from_record(record: PersistedMemoryItemRecord) -> AppResult<MemoryItem> {
    Ok(MemoryItem {
        urn: MemoryItemUrn::new(record.urn),
        source_id: record.source_id,
        sequence: record.sequence,
        unit_type: memory_unit_type_from_str(&record.unit_type)?,
        start_offset: record.start_offset,
        end_offset: record.end_offset,
        version: record.version,
        content: record.content,
        content_hash: record.content_hash,
        item_metadata: record.item_metadata,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn document_type_from_str(value: &str) -> AppResult<DocumentType> {
    match value {
        "text" => Ok(DocumentType::Text),
        "markdown" => Ok(DocumentType::Markdown),
        "json" => Ok(DocumentType::Json),
        _ => Err(AppError::internal(format!(
            "unknown document_type '{value}'"
        ))),
    }
}

fn memory_unit_type_from_str(value: &str) -> AppResult<MemoryUnitType> {
    match value {
        "paragraph" => Ok(MemoryUnitType::Paragraph),
        "section" => Ok(MemoryUnitType::Section),
        "json_document" => Ok(MemoryUnitType::JsonDocument),
        "metadata_placeholder" => Ok(MemoryUnitType::MetadataPlaceholder),
        _ => Err(AppError::internal(format!("unknown unit_type '{value}'"))),
    }
}
