use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use time::OffsetDateTime;
use tokio::time::timeout;
use uuid::Uuid;

use core_shared::{AppError, AppResult, IdGenerator};

use crate::domain::event::GraphProjectionEvent;
use crate::domain::normalization::{
    NormalizationInput, normalize_source, normalized_json_hash_from_str, raw_body_hash_from_str,
};
use crate::domain::source::{
    CANONICAL_ID_VERSION, DocumentType, IngestKind, NewSource, SourceSystemMetadata,
};
use crate::domain::source_external_id::{
    CanonicalSourceExternalId, canonicalize_direct_standard_payload,
};
use crate::domain::source_identity::deterministic_source_id;
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
    pub source_metadata: Value,
    pub semantic_payload_hash: String,
    pub original_standard_id: Option<String>,
    pub raw_body_hash: Option<String>,
    pub ingest_kind: IngestKind,
}

impl RegisterSourceCommand {
    pub fn canonical(
        external_id: String,
        title: String,
        summary: Option<String>,
        document_type: DocumentType,
        authoritative_content: String,
        source_metadata: Value,
        raw_body: &str,
    ) -> AppResult<Self> {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err(AppError::validation("title is required"));
        }

        Ok(Self {
            external_id: CanonicalSourceExternalId::parse_canonical_uri(external_id.trim())?
                .canonical_uri(),
            title,
            summary: summary.and_then(trimmed_option),
            document_type,
            authoritative_content,
            source_metadata,
            semantic_payload_hash: normalized_json_hash_from_str(raw_body)?,
            original_standard_id: None,
            raw_body_hash: None,
            ingest_kind: IngestKind::Canonical,
        })
    }

    pub fn direct_standard(value: &Value, raw_body: &str) -> AppResult<Self> {
        let standard = canonicalize_direct_standard_payload(value)?;

        Ok(Self {
            external_id: standard.external_id.canonical_uri(),
            title: standard.title,
            summary: None,
            document_type: DocumentType::Json,
            authoritative_content: raw_body.to_owned(),
            source_metadata: Value::Object(Default::default()),
            semantic_payload_hash: normalized_json_hash_from_str(raw_body)?,
            original_standard_id: Some(standard.original_standard_id),
            raw_body_hash: Some(raw_body_hash_from_str(raw_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.external_id.trim().is_empty() {
            return Err(AppError::validation("external_id is required"));
        }
        if self.title.trim().is_empty() {
            return Err(AppError::validation("title is required"));
        }
        if self.semantic_payload_hash.trim().is_empty() {
            return Err(AppError::validation("semantic_payload_hash is required"));
        }
        if self.authoritative_content.len() > MAX_AUTHORITATIVE_CONTENT_BYTES {
            return Err(AppError::validation(
                "authoritative content exceeds the 10 MB ingest limit",
            ));
        }
        Ok(())
    }
}

fn trimmed_option(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
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
    pub source_metadata: Value,
    pub memory_items: Vec<RegisteredMemoryItem>,
    pub indexing_status: PublicIndexingStatus,
    pub replayed: bool,
    pub decision_reason: String,
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
        let canonical_external_id = command.external_id.clone();
        let semantic_payload_hash = command.semantic_payload_hash.clone();
        let original_standard_id = command.original_standard_id.clone();
        let ingest_kind = command.ingest_kind;
        let source_id = deterministic_source_id(CANONICAL_ID_VERSION, &command.external_id);
        let id_generator = self.id_generator.clone();

        let (new_source, memory_items) = timeout(self.timeout, async move {
            let new_source = NewSource::new(
                source_id,
                command.external_id.clone(),
                command.title.clone(),
                command.summary.clone(),
                command.document_type,
                command.source_metadata.clone(),
                SourceSystemMetadata::new(
                    command.ingest_kind,
                    command.semantic_payload_hash.clone(),
                    command.original_standard_id.clone(),
                    command.raw_body_hash.clone(),
                )?,
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
            SourceCreateOrReplay::Replay(bundle) => {
                tracing::info!(
                    canonical_external_id = %canonical_external_id,
                    canonical_id_version = CANONICAL_ID_VERSION,
                    semantic_payload_hash = %semantic_payload_hash,
                    ingest_kind = %ingest_kind.as_str(),
                    decision_reason = "REPLAY_SEMANTIC_MATCH",
                    "source registration replayed"
                );
                Ok(Self::into_result(
                    bundle,
                    true,
                    "REPLAY_SEMANTIC_MATCH".to_owned(),
                ))
            }
            SourceCreateOrReplay::Create(source) => {
                let job = self.indexing_port.create_job(source.source_id, created_at);
                let bundle = self
                    .memory_repo
                    .commit_registration(source, memory_items, job)
                    .await?;
                let event =
                    GraphProjectionEvent::source_registered(&bundle.source, &bundle.memory_items);
                let _ = self.graph_port.project(&event);
                let decision_reason = match command.ingest_kind {
                    IngestKind::Canonical => "MANUAL_CANONICAL_ACCEPTED",
                    IngestKind::DirectStandard => "DIRECT_STANDARD_CANONICALIZED",
                };
                tracing::info!(
                    source_id = %bundle.source.source_id,
                    canonical_external_id = %bundle.source.external_id,
                    canonical_id_version = CANONICAL_ID_VERSION,
                    semantic_payload_hash = %semantic_payload_hash,
                    original_standard_id = ?original_standard_id,
                    ingest_kind = %ingest_kind.as_str(),
                    decision_reason = decision_reason,
                    "source registration created"
                );
                Ok(Self::into_result(bundle, false, decision_reason.to_owned()))
            }
        }
    }

    fn into_result(
        bundle: SourceBundle,
        replayed: bool,
        decision_reason: String,
    ) -> RegisterSourceResult {
        RegisterSourceResult {
            source_id: bundle.source.source_id,
            external_id: bundle.source.external_id.clone(),
            document_type: bundle.source.document_type,
            source_metadata: bundle.source.public_source_metadata(),
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
            decision_reason,
        }
    }
}
