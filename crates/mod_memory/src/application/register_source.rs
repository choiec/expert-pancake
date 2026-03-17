use std::sync::Arc;
use std::time::Duration;

use time::OffsetDateTime;
use tokio::time::timeout;
use uuid::Uuid;

use core_shared::{AppError, AppResult, IdGenerator};

use crate::domain::event::GraphProjectionEvent;
use crate::domain::normalization::{NormalizationInput, normalize_source};
use crate::domain::source::{DocumentType, IngestKind, NewSource, SourceSystemMetadata};
use crate::infra::graph::GraphProjectionPort;
use crate::infra::indexer::{IndexingPort, PublicIndexingStatus};
use crate::infra::repo::{MemoryRepository, SourceBundle, SourceCreateOrReplay, SourceRepository};

const MAX_AUTHORITATIVE_CONTENT_BYTES: usize = 10 * 1024 * 1024;

pub trait ClockPort: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl ClockPort for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterSourceCommand {
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: DocumentType,
    pub authoritative_content: String,
    pub source_metadata: serde_json::Value,
    pub canonical_payload_hash: String,
    pub ingest_kind: IngestKind,
}

impl RegisterSourceCommand {
    pub fn validate(&self) -> AppResult<()> {
        if self.external_id.trim().is_empty() {
            return Err(AppError::validation("external_id is required"));
        }
        if self.title.trim().is_empty() {
            return Err(AppError::validation("title is required"));
        }
        if self.canonical_payload_hash.trim().is_empty() {
            return Err(AppError::validation("canonical_payload_hash is required"));
        }
        if self.authoritative_content.len() > MAX_AUTHORITATIVE_CONTENT_BYTES {
            return Err(AppError::validation(
                "authoritative content exceeds the 10 MB ingest limit",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredMemoryItem {
    pub urn: String,
    pub sequence: u32,
    pub unit_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterSourceResult {
    pub source_id: Uuid,
    pub external_id: String,
    pub document_type: DocumentType,
    pub memory_items: Vec<RegisteredMemoryItem>,
    pub indexing_status: PublicIndexingStatus,
    pub replayed: bool,
}

pub struct RegisterSourceService {
    source_repo: Arc<dyn SourceRepository>,
    memory_repo: Arc<dyn MemoryRepository>,
    indexing_port: Arc<dyn IndexingPort>,
    graph_port: Arc<dyn GraphProjectionPort>,
    clock: Arc<dyn ClockPort>,
    id_generator: Arc<dyn IdGenerator>,
    timeout: Duration,
}

impl RegisterSourceService {
    pub fn new(
        source_repo: Arc<dyn SourceRepository>,
        memory_repo: Arc<dyn MemoryRepository>,
        indexing_port: Arc<dyn IndexingPort>,
        graph_port: Arc<dyn GraphProjectionPort>,
        clock: Arc<dyn ClockPort>,
        id_generator: Arc<dyn IdGenerator>,
        timeout: Duration,
    ) -> Self {
        Self {
            source_repo,
            memory_repo,
            indexing_port,
            graph_port,
            clock,
            id_generator,
            timeout,
        }
    }

    pub async fn execute(&self, command: RegisterSourceCommand) -> AppResult<RegisterSourceResult> {
        command.validate()?;
        let created_at = self.clock.now();
        let source_id = self.id_generator.new_uuid();
        let id_generator = self.id_generator.clone();

        let (new_source, memory_items) = timeout(self.timeout, async move {
            let new_source = NewSource::new(
                source_id,
                command.external_id.clone(),
                command.title.clone(),
                command.summary.clone(),
                command.document_type,
                command.source_metadata.clone(),
                SourceSystemMetadata {
                    canonical_payload_hash: command.canonical_payload_hash.clone(),
                    ingest_kind: command.ingest_kind,
                },
                created_at,
            )?;
            let memory_items = normalize_source(
                &NormalizationInput {
                    source_id,
                    external_id: &command.external_id,
                    title: &command.title,
                    summary: command.summary.as_deref(),
                    document_type: command.document_type,
                    authoritative_content: &command.authoritative_content,
                    source_metadata: &command.source_metadata,
                    created_at,
                },
                id_generator.as_ref(),
            )?;
            Ok::<_, AppError>((new_source, memory_items))
        })
        .await
        .map_err(|_| AppError::timeout("normalization exceeded the 30 second service budget"))??;

        match self
            .source_repo
            .prepare_create_or_replay(new_source.clone())
            .await?
        {
            SourceCreateOrReplay::Replay(bundle) => Ok(Self::into_result(bundle, true)),
            SourceCreateOrReplay::Create(source) => {
                let job = self.indexing_port.create_job(source.source_id, created_at);
                let bundle = self
                    .memory_repo
                    .commit_registration(source, memory_items, job)
                    .await?;
                let event =
                    GraphProjectionEvent::source_registered(&bundle.source, &bundle.memory_items);
                let _ = self.graph_port.project(&event);
                Ok(Self::into_result(bundle, false))
            }
        }
    }

    fn into_result(bundle: SourceBundle, replayed: bool) -> RegisterSourceResult {
        RegisterSourceResult {
            source_id: bundle.source.source_id,
            external_id: bundle.source.external_id.clone(),
            document_type: bundle.source.document_type,
            memory_items: bundle
                .memory_items
                .iter()
                .map(|item| RegisteredMemoryItem {
                    urn: item.urn.to_string(),
                    sequence: item.sequence,
                    unit_type: item.unit_type.as_str().to_owned(),
                })
                .collect(),
            indexing_status: bundle.indexing_status,
            replayed,
        }
    }
}
