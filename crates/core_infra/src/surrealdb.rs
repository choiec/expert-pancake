use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use core_shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SurrealDbClient {
    pub url: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
}

impl SurrealDbClient {
    pub fn new(
        url: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            namespace: namespace.into(),
            database: database.into(),
            username: username.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSourceRecord {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: String,
    pub source_metadata: Value,
    pub canonical_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedCredentialProofRecord {
    pub proof_type: String,
    pub proof_purpose: String,
    pub verification_method: String,
    pub created: Option<String>,
    pub cryptosuite: Option<String>,
    pub proof_value: Option<String>,
    pub jws: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedStandardCredentialRecord {
    pub source_id: Uuid,
    pub family: String,
    pub version: String,
    pub credential_id: String,
    pub credential_name: String,
    pub issuer_id: String,
    pub subject_id: Option<String>,
    pub raw_body: String,
    pub raw_body_hash: String,
    pub envelope: Value,
    pub normalized_envelope: Value,
    pub credential_subject: Value,
    pub achievement: Option<Value>,
    pub credential_schema: Vec<Value>,
    pub credential_status: Vec<Value>,
    pub evidence: Vec<Value>,
    pub refresh_service: Vec<Value>,
    pub terms_of_use: Vec<Value>,
    pub proofs: Vec<PersistedCredentialProofRecord>,
    pub verification: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedMemoryItemRecord {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub unit_type: String,
    pub start_offset: u32,
    pub end_offset: u32,
    pub version: String,
    pub content: String,
    pub content_hash: String,
    pub item_metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchProjectionRecord {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub document_type: String,
    pub content_preview: String,
    pub content_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedSourceBundle {
    pub source: PersistedSourceRecord,
    pub memory_items: Vec<PersistedMemoryItemRecord>,
    pub indexing_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRegistrationOutcome {
    Created(PersistedSourceBundle),
    Replay(PersistedSourceBundle),
}

#[derive(Debug, Clone)]
struct SurrealState {
    sources_by_external_id: HashMap<String, PersistedSourceRecord>,
    memory_by_urn: HashMap<String, PersistedMemoryItemRecord>,
    credentials_by_source_id: HashMap<Uuid, PersistedStandardCredentialRecord>,
    urns_by_source_id: HashMap<Uuid, Vec<String>>,
    search_docs: Vec<SearchProjectionRecord>,
    write_available: bool,
    search_available: bool,
}

impl Default for SurrealState {
    fn default() -> Self {
        Self {
            sources_by_external_id: HashMap::new(),
            memory_by_urn: HashMap::new(),
            credentials_by_source_id: HashMap::new(),
            urns_by_source_id: HashMap::new(),
            search_docs: Vec::new(),
            write_available: true,
            search_available: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemorySurrealDb {
    state: Arc<Mutex<SurrealState>>,
}

impl InMemorySurrealDb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_write_available(&self, available: bool) {
        self.state.lock().expect("state poisoned").write_available = available;
    }

    pub fn set_search_available(&self, available: bool) {
        self.state.lock().expect("state poisoned").search_available = available;
    }

    pub fn readiness_probe(&self) -> AppResult<()> {
        if self.state.lock().expect("state poisoned").write_available {
            Ok(())
        } else {
            Err(AppError::storage_unavailable(
                "SurrealDB write path is unavailable",
            ))
        }
    }

    pub fn commit_registration(
        &self,
        source: PersistedSourceRecord,
        memory_items: Vec<PersistedMemoryItemRecord>,
        credential: Option<PersistedStandardCredentialRecord>,
    ) -> AppResult<CommitRegistrationOutcome> {
        let mut state = self.state.lock().expect("state poisoned");
        if !state.write_available {
            return Err(AppError::storage_unavailable(
                "SurrealDB write path is unavailable",
            ));
        }

        if let Some(existing) = state
            .sources_by_external_id
            .get(&source.external_id)
            .cloned()
        {
            let items = Self::items_for_source(&state, existing.source_id);
            if existing.canonical_hash == source.canonical_hash {
                return Ok(CommitRegistrationOutcome::Replay(PersistedSourceBundle {
                    source: existing,
                    memory_items: items,
                    indexing_status: Self::public_indexing_status(state.search_available),
                }));
            }
            return Err(AppError::conflict(format!(
                "external_id '{}' is already registered with different content",
                source.external_id
            )));
        }

        let source_id = source.source_id;
        let external_id = source.external_id.clone();
        if let Some(credential) = credential {
            state.credentials_by_source_id.insert(source_id, credential);
        }
        for item in &memory_items {
            state.memory_by_urn.insert(item.urn.clone(), item.clone());
            state
                .urns_by_source_id
                .entry(source_id)
                .or_default()
                .push(item.urn.clone());
        }

        if state.search_available {
            for item in &memory_items {
                state.search_docs.push(SearchProjectionRecord {
                    urn: item.urn.clone(),
                    source_id,
                    sequence: item.sequence,
                    document_type: source.document_type.clone(),
                    content_preview: item.content.chars().take(500).collect(),
                    content_hash: item.content_hash.clone(),
                    created_at: item.created_at,
                    updated_at: item.updated_at,
                    score: None,
                });
            }
        }

        state
            .sources_by_external_id
            .insert(external_id, source.clone());

        Ok(CommitRegistrationOutcome::Created(PersistedSourceBundle {
            source,
            memory_items,
            indexing_status: Self::public_indexing_status(state.search_available),
        }))
    }

    pub fn get_source_bundle(&self, source_id: Uuid) -> Option<PersistedSourceBundle> {
        let state = self.state.lock().expect("state poisoned");
        let source = state
            .sources_by_external_id
            .values()
            .find(|item| item.source_id == source_id)?
            .clone();
        Some(PersistedSourceBundle {
            source,
            memory_items: Self::items_for_source(&state, source_id),
            indexing_status: Self::public_indexing_status(state.search_available),
        })
    }

    pub fn get_memory_item(
        &self,
        urn: &str,
    ) -> Option<(PersistedMemoryItemRecord, PersistedSourceRecord, String)> {
        let state = self.state.lock().expect("state poisoned");
        let item = state.memory_by_urn.get(urn)?.clone();
        let source = state
            .sources_by_external_id
            .values()
            .find(|candidate| candidate.source_id == item.source_id)?
            .clone();
        Some((
            item,
            source,
            Self::public_indexing_status(state.search_available),
        ))
    }

    pub fn get_standard_credential(
        &self,
        source_id: Uuid,
    ) -> Option<PersistedStandardCredentialRecord> {
        self.state
            .lock()
            .expect("state poisoned")
            .credentials_by_source_id
            .get(&source_id)
            .cloned()
    }

    pub fn search(
        &self,
        query: Option<&str>,
        source_id: Option<Uuid>,
        document_type: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> AppResult<(usize, Vec<SearchProjectionRecord>)> {
        let state = self.state.lock().expect("state poisoned");
        if !state.search_available {
            return Err(AppError::search_unavailable(
                "search projection is unavailable",
            ));
        }

        let mut items: Vec<_> = state
            .search_docs
            .iter()
            .filter(|item| source_id.is_none_or(|id| item.source_id == id))
            .filter(|item| document_type.is_none_or(|kind| item.document_type == kind))
            .filter(|item| {
                query.is_none_or(|needle| {
                    item.content_preview
                        .to_lowercase()
                        .contains(&needle.to_lowercase())
                })
            })
            .cloned()
            .collect();

        for item in &mut items {
            item.score = query.map(|needle| {
                if item
                    .content_preview
                    .to_lowercase()
                    .contains(&needle.to_lowercase())
                {
                    1.0
                } else {
                    0.0
                }
            });
        }

        let total = items.len();
        let paged = items.into_iter().skip(offset).take(limit).collect();
        Ok((total, paged))
    }

    fn items_for_source(state: &SurrealState, source_id: Uuid) -> Vec<PersistedMemoryItemRecord> {
        let mut items: Vec<_> = state
            .urns_by_source_id
            .get(&source_id)
            .into_iter()
            .flat_map(|urns| urns.iter())
            .filter_map(|urn| state.memory_by_urn.get(urn).cloned())
            .collect();
        items.sort_by_key(|item| item.sequence);
        items
    }

    fn public_indexing_status(search_available: bool) -> String {
        if search_available {
            "indexed".to_owned()
        } else {
            "deferred".to_owned()
        }
    }
}
