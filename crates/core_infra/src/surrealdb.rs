use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use core_shared::StartupError;
use core_shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use surrealdb::{
    Surreal,
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
};
use time::OffsetDateTime;
use tokio::time::timeout;
use uuid::Uuid;

use crate::setup::SurrealDbSettings;

const SOURCE_TABLE: &str = "memory_source";
const MEMORY_ITEM_TABLE: &str = "memory_item";
const INDEX_JOB_TABLE: &str = "memory_index_job";

#[derive(Debug)]
pub struct SurrealDbService {
    client: Surreal<Client>,
    settings: SurrealDbSettings,
}

#[derive(Debug, Clone)]
pub struct DependencyReport {
    pub is_ready: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSourceRecord {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: String,
    pub source_metadata: serde_json::Value,
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
    pub item_metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedIndexJobRecord {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub available_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedSourceBundle {
    pub source: PersistedSourceRecord,
    pub memory_items: Vec<PersistedMemoryItemRecord>,
    pub latest_job: Option<PersistedIndexJobRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRehydrationBundle {
    pub source: PersistedSourceRecord,
    pub memory_items: Vec<PersistedMemoryItemRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitRegistrationOutcome {
    Created(PersistedSourceBundle),
    Replay(PersistedSourceBundle),
}

#[derive(Debug, Clone)]
struct SurrealState {
    sources_by_id: HashMap<Uuid, PersistedSourceRecord>,
    source_ids_by_external_id: HashMap<String, Uuid>,
    memory_by_urn: HashMap<String, PersistedMemoryItemRecord>,
    memory_urns_by_source_id: HashMap<Uuid, Vec<String>>,
    jobs_by_source_id: HashMap<Uuid, Vec<PersistedIndexJobRecord>>,
    write_available: bool,
    search_available: bool,
    fail_next_commit: bool,
}

impl Default for SurrealState {
    fn default() -> Self {
        Self {
            sources_by_id: HashMap::new(),
            source_ids_by_external_id: HashMap::new(),
            memory_by_urn: HashMap::new(),
            memory_urns_by_source_id: HashMap::new(),
            jobs_by_source_id: HashMap::new(),
            write_available: true,
            search_available: true,
            fail_next_commit: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Deterministic in-memory authoritative store used by tests and contracts.
///
/// Production/runtime code should use `SurrealDbService` and the runtime
/// repositories in `mod_memory::infra` so transactional semantics are enforced
/// by the real SurrealDB backend.
pub struct InMemorySurrealDb {
    state: Arc<Mutex<SurrealState>>,
}

impl InMemorySurrealDb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_write_available(&self, available: bool) {
        let mut state = self.state.lock().expect("surreal state poisoned");
        state.write_available = available;
    }

    pub fn set_search_available(&self, available: bool) {
        let mut state = self.state.lock().expect("surreal state poisoned");
        state.search_available = available;
    }

    pub fn search_available(&self) -> bool {
        self.state
            .lock()
            .expect("surreal state poisoned")
            .search_available
    }

    pub fn readiness_probe(&self) -> AppResult<()> {
        let state = self.state.lock().expect("surreal state poisoned");
        if state.write_available {
            Ok(())
        } else {
            Err(AppError::storage_unavailable(
                "SurrealDB write path is unavailable",
            ))
        }
    }

    pub fn fail_next_commit(&self) {
        let mut state = self.state.lock().expect("surreal state poisoned");
        state.fail_next_commit = true;
    }

    pub fn lookup_source_by_external_id(&self, external_id: &str) -> Option<PersistedSourceBundle> {
        let state = self.state.lock().expect("surreal state poisoned");
        let source_id = state.source_ids_by_external_id.get(external_id)?;
        Self::bundle_from_state(&state, *source_id)
    }

    pub fn get_source_bundle(&self, source_id: Uuid) -> Option<PersistedSourceBundle> {
        let state = self.state.lock().expect("surreal state poisoned");
        Self::bundle_from_state(&state, source_id)
    }

    pub fn get_memory_item(
        &self,
        urn: &str,
    ) -> Option<(PersistedMemoryItemRecord, PersistedSourceRecord)> {
        let state = self.state.lock().expect("surreal state poisoned");
        let item = state.memory_by_urn.get(urn)?.clone();
        let source = state.sources_by_id.get(&item.source_id)?.clone();
        Some((item, source))
    }

    pub fn rehydrate_projection(&self, source_id: Uuid) -> Option<ProjectionRehydrationBundle> {
        let state = self.state.lock().expect("surreal state poisoned");
        let bundle = Self::bundle_from_state(&state, source_id)?;
        Some(ProjectionRehydrationBundle {
            source: bundle.source,
            memory_items: bundle.memory_items,
        })
    }

    pub fn latest_index_job(&self, source_id: Uuid) -> Option<PersistedIndexJobRecord> {
        self.state
            .lock()
            .expect("surreal state poisoned")
            .jobs_by_source_id
            .get(&source_id)
            .and_then(|jobs| jobs.last().cloned())
    }

    pub fn claim_next_index_job(&self, now: OffsetDateTime) -> Option<PersistedIndexJobRecord> {
        let mut state = self.state.lock().expect("surreal state poisoned");
        let mut selected: Option<(Uuid, usize, PersistedIndexJobRecord)> = None;

        for (source_id, jobs) in &state.jobs_by_source_id {
            for (index, job) in jobs.iter().enumerate() {
                let eligible = matches!(job.status.as_str(), "pending" | "retryable")
                    && job.available_at <= now;
                if !eligible {
                    continue;
                }

                let better = selected.as_ref().is_none_or(|(_, _, current)| {
                    job.available_at < current.available_at
                        || (job.available_at == current.available_at
                            && job.created_at < current.created_at)
                });

                if better {
                    selected = Some((*source_id, index, job.clone()));
                }
            }
        }

        let (source_id, index, mut job) = selected?;
        job.status = "processing".to_owned();
        job.updated_at = now;
        state.jobs_by_source_id.get_mut(&source_id)?[index] = job.clone();
        Some(job)
    }

    pub fn update_index_job(&self, job: PersistedIndexJobRecord) -> AppResult<()> {
        let mut state = self.state.lock().expect("surreal state poisoned");
        let jobs = state
            .jobs_by_source_id
            .get_mut(&job.source_id)
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "index job source '{}' was not found",
                    job.source_id
                ))
            })?;

        let Some(position) = jobs
            .iter()
            .position(|existing| existing.job_id == job.job_id)
        else {
            return Err(AppError::not_found(format!(
                "index job '{}' was not found",
                job.job_id
            )));
        };

        jobs[position] = job;
        jobs.sort_by_key(|record| record.created_at);
        Ok(())
    }

    pub fn commit_registration(
        &self,
        source: PersistedSourceRecord,
        memory_items: Vec<PersistedMemoryItemRecord>,
        job: PersistedIndexJobRecord,
    ) -> AppResult<CommitRegistrationOutcome> {
        let mut state = self.state.lock().expect("surreal state poisoned");
        if !state.write_available {
            return Err(AppError::storage_unavailable(
                "SurrealDB write path is unavailable",
            ));
        }
        if state.fail_next_commit {
            state.fail_next_commit = false;
            return Err(AppError::storage_unavailable(
                "simulated transactional failure",
            ));
        }

        if let Some(existing_id) = state
            .source_ids_by_external_id
            .get(&source.external_id)
            .copied()
        {
            let existing = Self::bundle_from_state(&state, existing_id)
                .ok_or_else(|| AppError::internal("missing bundle for replayed source"))?;
            let existing_hash = existing
                .source
                .source_metadata
                .pointer("/system/semantic_payload_hash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let requested_hash = source
                .source_metadata
                .pointer("/system/semantic_payload_hash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if existing_hash == requested_hash {
                return Ok(CommitRegistrationOutcome::Replay(existing));
            }
            return Err(AppError::conflict(format!(
                "external_id '{}' is already bound to a different semantic payload",
                source.external_id
            )));
        }

        let mut next_state = state.clone();
        Self::validate_memory_constraints(&next_state, &source, &memory_items)?;

        next_state
            .source_ids_by_external_id
            .insert(source.external_id.clone(), source.source_id);
        next_state
            .sources_by_id
            .insert(source.source_id, source.clone());
        next_state.memory_urns_by_source_id.insert(
            source.source_id,
            memory_items.iter().map(|item| item.urn.clone()).collect(),
        );
        for item in &memory_items {
            next_state
                .memory_by_urn
                .insert(item.urn.clone(), item.clone());
        }
        next_state
            .jobs_by_source_id
            .entry(source.source_id)
            .or_default()
            .push(job);

        let bundle = Self::bundle_from_state(&next_state, source.source_id)
            .ok_or_else(|| AppError::internal("failed to assemble committed bundle"))?;
        *state = next_state;
        Ok(CommitRegistrationOutcome::Created(bundle))
    }

    fn validate_memory_constraints(
        state: &SurrealState,
        source: &PersistedSourceRecord,
        memory_items: &[PersistedMemoryItemRecord],
    ) -> AppResult<()> {
        let mut seen_sequences = HashMap::new();
        let mut seen_urns = HashMap::new();
        for item in memory_items {
            if item.source_id != source.source_id {
                return Err(AppError::validation("memory item source_id mismatch"));
            }
            if item.start_offset > item.end_offset {
                return Err(AppError::validation("memory item offsets are invalid"));
            }
            if state.memory_by_urn.contains_key(&item.urn)
                || seen_urns.insert(item.urn.clone(), item.sequence).is_some()
            {
                return Err(AppError::conflict(format!(
                    "memory item URN '{}' must be unique",
                    item.urn
                )));
            }
            if seen_sequences
                .insert(item.sequence, item.urn.clone())
                .is_some()
            {
                return Err(AppError::conflict(format!(
                    "duplicate sequence {} for source {}",
                    item.sequence, source.source_id
                )));
            }
        }
        Ok(())
    }

    fn bundle_from_state(state: &SurrealState, source_id: Uuid) -> Option<PersistedSourceBundle> {
        let source = state.sources_by_id.get(&source_id)?.clone();
        let mut memory_items = state
            .memory_urns_by_source_id
            .get(&source_id)
            .into_iter()
            .flatten()
            .filter_map(|urn| state.memory_by_urn.get(urn).cloned())
            .collect::<Vec<_>>();
        memory_items.sort_by_key(|item| item.sequence);
        let latest_job = state
            .jobs_by_source_id
            .get(&source_id)
            .and_then(|jobs| jobs.last().cloned());
        Some(PersistedSourceBundle {
            source,
            memory_items,
            latest_job,
        })
    }
}

pub async fn bootstrap(settings: SurrealDbSettings) -> Result<SurrealDbService, StartupError> {
    let settings_for_connect = settings.clone();
    let client = timeout(settings.connect_timeout, async move {
        let client = Surreal::new::<Ws>(settings_for_connect.url.as_str())
            .await
            .map_err(|error| StartupError::InfraBootstrap {
                component: "surrealdb".to_string(),
                reason: error.to_string(),
            })?;

        client
            .signin(Root {
                username: &settings_for_connect.username,
                password: &settings_for_connect.password,
            })
            .await
            .map_err(|error| StartupError::InfraBootstrap {
                component: "surrealdb".to_string(),
                reason: error.to_string(),
            })?;

        client
            .use_ns(&settings_for_connect.namespace)
            .use_db(&settings_for_connect.database)
            .await
            .map_err(|error| StartupError::InfraBootstrap {
                component: "surrealdb".to_string(),
                reason: error.to_string(),
            })?;

        Ok::<_, StartupError>(client)
    })
    .await
    .map_err(|_| StartupError::InfraBootstrap {
        component: "surrealdb".to_string(),
        reason: format!(
            "bootstrap timed out after {} ms",
            settings.connect_timeout.as_millis()
        ),
    })??;

    let service = SurrealDbService { client, settings };
    service
        .ensure_memory_ingest_schema()
        .await
        .map_err(|error| StartupError::InfraBootstrap {
            component: "surrealdb".to_owned(),
            reason: error.message().to_owned(),
        })?;

    Ok(service)
}

impl SurrealDbService {
    pub fn client(&self) -> &Surreal<Client> {
        &self.client
    }

    pub fn readiness_timeout(&self) -> Duration {
        self.settings.readiness_timeout
    }

    pub async fn readiness(&self) -> DependencyReport {
        let readiness = timeout(self.settings.readiness_timeout, async {
            let record_id = format!("probe-{}", Uuid::new_v4());

            let _: Option<serde_json::Value> = self
                .client
                .create(("readiness_probe", record_id.as_str()))
                .content(json!({ "checked": true }))
                .await
                .map_err(|error| error.to_string())?;

            let _: Option<serde_json::Value> = self
                .client
                .delete(("readiness_probe", record_id.as_str()))
                .await
                .map_err(|error| error.to_string())?;

            Ok::<(), String>(())
        })
        .await;

        match readiness {
            Ok(Ok(())) => DependencyReport {
                is_ready: true,
                detail: None,
            },
            Ok(Err(error)) => DependencyReport {
                is_ready: false,
                detail: Some(error),
            },
            Err(_) => DependencyReport {
                is_ready: false,
                detail: Some(format!(
                    "readiness probe timed out after {} ms",
                    self.settings.readiness_timeout.as_millis()
                )),
            },
        }
    }

    pub async fn ensure_memory_ingest_schema(&self) -> AppResult<()> {
        let response = self
            .client
            .query(format!(
                "DEFINE TABLE IF NOT EXISTS {SOURCE_TABLE} SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS {MEMORY_ITEM_TABLE} SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS {INDEX_JOB_TABLE} SCHEMALESS;\n\
DEFINE INDEX IF NOT EXISTS memory_source_source_id ON TABLE {SOURCE_TABLE} COLUMNS source_id UNIQUE;\n\
DEFINE INDEX IF NOT EXISTS memory_source_external_id ON TABLE {SOURCE_TABLE} COLUMNS external_id UNIQUE;\n\
DEFINE INDEX IF NOT EXISTS memory_item_urn ON TABLE {MEMORY_ITEM_TABLE} COLUMNS urn UNIQUE;\n\
DEFINE INDEX IF NOT EXISTS memory_item_source_sequence ON TABLE {MEMORY_ITEM_TABLE} COLUMNS source_id, sequence UNIQUE;\n\
DEFINE INDEX IF NOT EXISTS memory_index_job_job_id ON TABLE {INDEX_JOB_TABLE} COLUMNS job_id UNIQUE;\n\
DEFINE INDEX IF NOT EXISTS memory_index_job_source_id_created_at ON TABLE {INDEX_JOB_TABLE} COLUMNS source_id, created_at;"
            ))
            .await
            .map_err(map_surreal_read_error)?;
        response.check().map_err(map_surreal_write_error)?;
        Ok(())
    }

    pub async fn find_source_by_external_id(
        &self,
        external_id: &str,
    ) -> AppResult<Option<PersistedSourceRecord>> {
        let mut response = self
            .client
            .query(format!(
                "SELECT * FROM {SOURCE_TABLE} WHERE external_id = $external_id LIMIT 1;"
            ))
            .bind(("external_id", external_id.to_owned()))
            .await
            .map_err(map_surreal_read_error)?;
        let rows: Vec<PersistedSourceRecord> = response.take(0).map_err(map_surreal_read_error)?;
        Ok(rows.into_iter().next())
    }

    pub async fn find_source_by_source_id(
        &self,
        source_id: Uuid,
    ) -> AppResult<Option<PersistedSourceRecord>> {
        let mut response = self
            .client
            .query(format!(
                "SELECT * FROM {SOURCE_TABLE} WHERE source_id = $source_id LIMIT 1;"
            ))
            .bind(("source_id", source_id))
            .await
            .map_err(map_surreal_read_error)?;
        let rows: Vec<PersistedSourceRecord> = response.take(0).map_err(map_surreal_read_error)?;
        Ok(rows.into_iter().next())
    }

    pub async fn find_memory_items_by_source_id(
        &self,
        source_id: Uuid,
    ) -> AppResult<Vec<PersistedMemoryItemRecord>> {
        let mut response = self
            .client
            .query(format!(
                "SELECT * FROM {MEMORY_ITEM_TABLE} WHERE source_id = $source_id ORDER BY sequence ASC;"
            ))
            .bind(("source_id", source_id))
            .await
            .map_err(map_surreal_read_error)?;
        response.take(0).map_err(map_surreal_read_error)
    }

    pub async fn find_latest_index_job_by_source_id(
        &self,
        source_id: Uuid,
    ) -> AppResult<Option<PersistedIndexJobRecord>> {
        let mut response = self
            .client
            .query(format!(
                "SELECT * FROM {INDEX_JOB_TABLE} WHERE source_id = $source_id ORDER BY created_at DESC LIMIT 1;"
            ))
            .bind(("source_id", source_id))
            .await
            .map_err(map_surreal_read_error)?;
        let rows: Vec<PersistedIndexJobRecord> =
            response.take(0).map_err(map_surreal_read_error)?;
        Ok(rows.into_iter().next())
    }

    pub async fn claim_next_index_job(
        &self,
        now: OffsetDateTime,
    ) -> AppResult<Option<PersistedIndexJobRecord>> {
        let mut response = self
            .client
            .query(format!(
                "SELECT * FROM {INDEX_JOB_TABLE} WHERE available_at <= $now AND (status = 'pending' OR status = 'retryable') ORDER BY available_at ASC, created_at ASC LIMIT 1;"
            ))
            .bind(("now", now))
            .await
            .map_err(map_surreal_read_error)?;
        let rows: Vec<PersistedIndexJobRecord> =
            response.take(0).map_err(map_surreal_read_error)?;
        let Some(mut job) = rows.into_iter().next() else {
            return Ok(None);
        };

        job.status = "processing".to_owned();
        job.updated_at = now;
        self.update_index_job(job.clone()).await?;

        Ok(Some(job))
    }

    pub async fn update_index_job(&self, job: PersistedIndexJobRecord) -> AppResult<()> {
        let response = self
            .client
            .query(format!(
                "UPDATE {INDEX_JOB_TABLE} SET status = $status, retry_count = $retry_count, last_error = $last_error, available_at = $available_at, updated_at = $updated_at WHERE job_id = $job_id;"
            ))
            .bind(("job_id", job.job_id))
            .bind(("status", job.status))
            .bind(("retry_count", job.retry_count))
            .bind(("last_error", job.last_error))
            .bind(("available_at", job.available_at))
            .bind(("updated_at", job.updated_at))
            .await
            .map_err(map_surreal_write_error)?;
        response.check().map_err(map_surreal_write_error)?;
        Ok(())
    }

    pub async fn rehydrate_projection(
        &self,
        source_id: Uuid,
    ) -> AppResult<Option<ProjectionRehydrationBundle>> {
        let Some(source) = self.find_source_by_source_id(source_id).await? else {
            return Ok(None);
        };
        let memory_items = self.find_memory_items_by_source_id(source_id).await?;
        Ok(Some(ProjectionRehydrationBundle {
            source,
            memory_items,
        }))
    }

    pub async fn find_memory_item_by_urn(
        &self,
        urn: &str,
    ) -> AppResult<Option<PersistedMemoryItemRecord>> {
        let mut response = self
            .client
            .query(format!(
                "SELECT * FROM {MEMORY_ITEM_TABLE} WHERE urn = $urn LIMIT 1;"
            ))
            .bind(("urn", urn.to_owned()))
            .await
            .map_err(map_surreal_read_error)?;
        let rows: Vec<PersistedMemoryItemRecord> =
            response.take(0).map_err(map_surreal_read_error)?;
        Ok(rows.into_iter().next())
    }

    pub async fn commit_authoritative_registration(
        &self,
        source: PersistedSourceRecord,
        memory_items: Vec<PersistedMemoryItemRecord>,
        job: PersistedIndexJobRecord,
    ) -> AppResult<()> {
        let mut query = format!("BEGIN TRANSACTION;\nCREATE {SOURCE_TABLE} CONTENT $source;\n");
        for index in 0..memory_items.len() {
            query.push_str(&format!(
                "CREATE {MEMORY_ITEM_TABLE} CONTENT $memory_item_{index};\n"
            ));
        }
        query.push_str(&format!(
            "CREATE {INDEX_JOB_TABLE} CONTENT $job;\nCOMMIT TRANSACTION;"
        ));

        let mut request = self.client.query(query).bind(("source", source));
        for (index, item) in memory_items.into_iter().enumerate() {
            let binding = format!("memory_item_{index}");
            request = request.bind((binding, item));
        }
        let response = request
            .bind(("job", job))
            .await
            .map_err(map_surreal_write_error)?;
        response.check().map_err(map_surreal_write_error)?;
        Ok(())
    }
}

fn map_surreal_read_error(error: impl std::fmt::Display) -> AppError {
    AppError::storage_unavailable(format!("SurrealDB read failed: {error}"))
}

fn map_surreal_write_error(error: impl std::fmt::Display) -> AppError {
    let message = error.to_string();
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("unique")
        || lowered.contains("duplicate")
        || lowered.contains("constraint")
        || lowered.contains("index")
    {
        return AppError::conflict(message);
    }
    AppError::storage_unavailable(format!("SurrealDB write failed: {message}"))
}
