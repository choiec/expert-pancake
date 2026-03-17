use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    Processing,
    Retryable,
    Completed,
    DeadLetter,
}

impl OutboxStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Retryable => "retryable",
            Self::Completed => "completed",
            Self::DeadLetter => "dead_letter",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "processing" => Some(Self::Processing),
            "retryable" => Some(Self::Retryable),
            "completed" => Some(Self::Completed),
            "dead_letter" => Some(Self::DeadLetter),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicIndexingStatus {
    Queued,
    Indexed,
    Deferred,
}

impl PublicIndexingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Indexed => "indexed",
            Self::Deferred => "deferred",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexingJob {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: OutboxStatus,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub available_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionInput {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub document_type: String,
    pub content_preview: String,
    pub content_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub trait IndexingPort: Send + Sync {
    fn create_job(&self, source_id: Uuid, created_at: OffsetDateTime) -> IndexingJob;

    fn registration_status(&self) -> PublicIndexingStatus;
}

#[derive(Debug, Clone)]
pub struct OutboxOnlyIndexer {
    search_available: bool,
}

impl OutboxOnlyIndexer {
    pub fn new(search_available: bool) -> Self {
        Self { search_available }
    }
}

impl IndexingPort for OutboxOnlyIndexer {
    fn create_job(&self, source_id: Uuid, created_at: OffsetDateTime) -> IndexingJob {
        IndexingJob {
            job_id: Uuid::new_v4(),
            source_id,
            status: OutboxStatus::Pending,
            retry_count: 0,
            last_error: None,
            available_at: created_at,
            created_at,
            updated_at: created_at,
        }
    }

    fn registration_status(&self) -> PublicIndexingStatus {
        if self.search_available {
            PublicIndexingStatus::Queued
        } else {
            PublicIndexingStatus::Deferred
        }
    }
}

pub fn derive_public_indexing_status(
    status: Option<OutboxStatus>,
    search_available: bool,
) -> PublicIndexingStatus {
    if !search_available {
        return PublicIndexingStatus::Deferred;
    }
    match status.unwrap_or(OutboxStatus::Pending) {
        OutboxStatus::Pending | OutboxStatus::Processing => PublicIndexingStatus::Queued,
        OutboxStatus::Completed => PublicIndexingStatus::Indexed,
        OutboxStatus::Retryable | OutboxStatus::DeadLetter => PublicIndexingStatus::Deferred,
    }
}
