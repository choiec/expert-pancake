use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use core_infra::surrealdb::{
    InMemorySurrealDb, PersistedAuthoritativeState, PersistedIndexJobRecord,
    PersistedMemoryItemRecord, PersistedSourceRecord,
};
use core_shared::{AppError, AppResult, DefaultIdGenerator, IdGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::normalization::{normalized_json_hash_from_str, raw_body_hash_from_str};
use crate::domain::source::{CANONICAL_ID_VERSION, IngestKind};
use crate::domain::source_external_id::{
    CanonicalSourceExternalId, canonicalize_direct_standard_payload,
};
use crate::domain::source_identity::{
    deterministic_source_id, source_seed, verify_source_id,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationClassification {
    Migratable,
    Consolidate,
    Unmigratable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependentReferenceCounts {
    pub memory_item: usize,
    pub memory_index_job: usize,
    pub search_projection: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationRowReport {
    pub legacy_source_id: Uuid,
    pub legacy_external_id: String,
    pub candidate_canonical_external_id: String,
    pub candidate_source_seed: String,
    pub candidate_source_id: Uuid,
    pub classification: MigrationClassification,
    pub decision_reason: String,
    pub legacy_resolution_path: String,
    pub canonical_id_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_standard_id: Option<String>,
    pub semantic_payload_hash: String,
    pub raw_body_hash_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body_hash: Option<String>,
    pub dependent_reference_counts: DependentReferenceCounts,
    pub planned_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationSummary {
    pub total_rows: usize,
    pub migratable_rows: usize,
    pub consolidation_groups: usize,
    pub unmigratable_rows: usize,
    pub conflict_groups: usize,
    pub reference_gap_rows: usize,
    pub stop_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationDryRunReport {
    pub run_id: Uuid,
    pub migration_phase: String,
    pub summary: MigrationSummary,
    pub rows: Vec<MigrationRowReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationVerificationReport {
    pub migration_phase: String,
    pub verified: bool,
    pub canonical_row_count: usize,
    pub remap_count: usize,
}

pub struct InMemorySourceMigration {
    db: Arc<InMemorySurrealDb>,
}

impl InMemorySourceMigration {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }

    pub fn snapshot(&self) -> PersistedAuthoritativeState {
        self.db.export_state()
    }

    pub fn rollback(&self, snapshot: PersistedAuthoritativeState) {
        let mut restored = snapshot;
        restored.migration_phase = "rollback".to_owned();
        self.db.replace_state(restored.clone());
        let mut steady_state = restored;
        steady_state.migration_phase = "steady_state".to_owned();
        self.db.replace_state(steady_state);
        tracing::warn!(
            migration_phase = "rollback",
            decision_reason = "MIGRATION_ROLLED_BACK",
            "source migration rolled back via full snapshot restore"
        );
    }

    pub fn dry_run(&self) -> AppResult<MigrationDryRunReport> {
        let snapshot = self.db.export_state();
        let rows = build_rows(&snapshot)?;
        let summary = summarize_rows(&rows, snapshot.snapshot_ready);
        tracing::info!(
            migration_phase = "dry_run",
            decision_reason = if summary.stop_required {
                "MIGRATION_ABORTED_STOP_CONDITION"
            } else {
                "MIGRATION_VERIFIED"
            },
            total_rows = summary.total_rows,
            unmigratable_rows = summary.unmigratable_rows,
            conflict_groups = summary.conflict_groups,
            "source migration dry-run completed"
        );
        Ok(MigrationDryRunReport {
            run_id: Uuid::new_v4(),
            migration_phase: "dry_run".to_owned(),
            summary,
            rows,
        })
    }

    pub fn execute(&self) -> AppResult<MigrationDryRunReport> {
        let snapshot = self.db.export_state();
        if !snapshot.snapshot_ready {
            return Err(AppError::storage_unavailable(
                "snapshot or backup gate failed before migration execution",
            )
            .with_error_code("MIGRATION_BACKUP_GATE_FAILED"));
        }

        let report = self.dry_run()?;
        if report.summary.stop_required {
            tracing::warn!(
                migration_phase = "rewrite",
                decision_reason = "MIGRATION_ABORTED_STOP_CONDITION",
                "source migration aborted by stop conditions"
            );
            return Err(AppError::conflict(
                "source migration dry-run reported stop conditions",
            )
            .with_error_code("MIGRATION_STOP_CONDITION"));
        }

        let next_state = rewrite_state(snapshot, &report.rows)?;
        self.db.replace_state(next_state);
        tracing::info!(
            migration_phase = "verification",
            decision_reason = "MIGRATION_VERIFIED",
            "source migration rewrite completed"
        );
        Ok(report)
    }

    pub fn verify(&self, report: &MigrationDryRunReport) -> AppResult<MigrationVerificationReport> {
        let snapshot = self.db.export_state();
        let source_ids = snapshot
            .sources
            .iter()
            .map(|source| source.source_id)
            .collect::<BTreeSet<_>>();

        for source in &snapshot.sources {
            CanonicalSourceExternalId::parse_canonical_uri(&source.external_id)?;
            if source
                .source_metadata
                .pointer("/system/canonical_id_version")
                .and_then(Value::as_str)
                != Some(CANONICAL_ID_VERSION)
            {
                return Err(AppError::conflict(
                    "source migration verification found a non-v1 canonical row",
                )
                .with_error_code("MIGRATION_VERIFICATION_FAILED"));
            }
            if source
                .source_metadata
                .pointer("/system/canonical_payload_hash")
                .is_some()
            {
                return Err(AppError::conflict(
                    "source migration verification found a legacy canonical_payload_hash alias",
                )
                .with_error_code("MIGRATION_VERIFICATION_FAILED"));
            }
        }

        for item in &snapshot.memory_items {
            if !source_ids.contains(&item.source_id) {
                return Err(AppError::conflict(
                    "source migration verification found an unre-written memory_item reference",
                )
                .with_error_code("MIGRATION_VERIFICATION_FAILED"));
            }
        }

        for job in &snapshot.index_jobs {
            if !source_ids.contains(&job.source_id) {
                return Err(AppError::conflict(
                    "source migration verification found an unre-written memory_index_job reference",
                )
                .with_error_code("MIGRATION_VERIFICATION_FAILED"));
            }
        }

        let expected_count = report
            .rows
            .iter()
            .filter(|row| row.classification != MigrationClassification::Unmigratable)
            .map(|row| row.candidate_source_id)
            .collect::<BTreeSet<_>>()
            .len();

        if snapshot.sources.len() != expected_count {
            return Err(AppError::conflict(
                "source migration verification found a canonical row-count mismatch",
            )
            .with_error_code("MIGRATION_VERIFICATION_FAILED"));
        }

        for row in &report.rows {
            let expected_seed = source_seed(&row.canonical_id_version, &row.candidate_canonical_external_id);
            if row.candidate_source_seed != expected_seed
                || !verify_source_id(
                    &row.canonical_id_version,
                    &row.candidate_canonical_external_id,
                    row.candidate_source_id,
                )
            {
                return Err(AppError::conflict(
                    "source migration verification found a non-reproducible candidate_source_id",
                )
                .with_error_code("MIGRATION_VERIFICATION_FAILED"));
            }
        }

        tracing::info!(
            migration_phase = snapshot.migration_phase,
            decision_reason = "MIGRATION_VERIFIED",
            canonical_row_count = snapshot.sources.len(),
            remap_count = snapshot.source_id_remaps.len(),
            "source migration verification passed"
        );

        Ok(MigrationVerificationReport {
            migration_phase: snapshot.migration_phase,
            verified: true,
            canonical_row_count: snapshot.sources.len(),
            remap_count: snapshot.source_id_remaps.len(),
        })
    }

    pub fn complete_cutover(&self) {
        let mut snapshot = self.db.export_state();
        snapshot.migration_phase = "steady_state".to_owned();
        snapshot.source_id_remaps.clear();
        self.db.replace_state(snapshot);
    }
}

fn build_rows(snapshot: &PersistedAuthoritativeState) -> AppResult<Vec<MigrationRowReport>> {
    let grouped_memory = snapshot
        .memory_items
        .iter()
        .fold(BTreeMap::<Uuid, Vec<PersistedMemoryItemRecord>>::new(), |mut acc, item| {
            acc.entry(item.source_id).or_default().push(item.clone());
            acc
        });
    let grouped_jobs = snapshot
        .index_jobs
        .iter()
        .fold(BTreeMap::<Uuid, Vec<PersistedIndexJobRecord>>::new(), |mut acc, job| {
            acc.entry(job.source_id).or_default().push(job.clone());
            acc
        });

    let mut rows = snapshot
        .sources
        .iter()
        .map(|source| {
            let memory_items = grouped_memory
                .get(&source.source_id)
                .cloned()
                .unwrap_or_default();
            let jobs = grouped_jobs.get(&source.source_id).cloned().unwrap_or_default();
            build_row(source, &memory_items, &jobs)
        })
        .collect::<AppResult<Vec<_>>>()?;

    let mut canonical_groups = BTreeMap::<String, Vec<usize>>::new();
    for (index, row) in rows.iter().enumerate() {
        if !row.candidate_canonical_external_id.is_empty() {
            canonical_groups
                .entry(row.candidate_canonical_external_id.clone())
                .or_default()
                .push(index);
        }
    }

    for indices in canonical_groups.values() {
        if indices.len() <= 1 {
            continue;
        }

        let hashes = indices
            .iter()
            .map(|index| rows[*index].semantic_payload_hash.clone())
            .collect::<BTreeSet<_>>();
        if hashes.len() == 1 {
            for index in indices {
                rows[*index].classification = MigrationClassification::Consolidate;
                rows[*index].decision_reason = "LEGACY_ROW_CONSOLIDATE_MATCH".to_owned();
                rows[*index].legacy_resolution_path = "shadow_duplicate".to_owned();
                rows[*index].planned_action = "consolidate".to_owned();
            }
        } else {
            for index in indices {
                rows[*index].classification = MigrationClassification::Unmigratable;
                rows[*index].decision_reason =
                    "LEGACY_ROW_UNMIGRATABLE_DUPLICATE_CONFLICT".to_owned();
                rows[*index].planned_action = "abort".to_owned();
            }
        }
    }

    rows.sort_by_key(|row| row.legacy_source_id);
    Ok(rows)
}

fn build_row(
    source: &PersistedSourceRecord,
    memory_items: &[PersistedMemoryItemRecord],
    jobs: &[PersistedIndexJobRecord],
) -> AppResult<MigrationRowReport> {
    let derived = derive_candidate_external_id(source, memory_items);
    let candidate_canonical_external_id = derived
        .as_ref()
        .map(|(external_id, _)| external_id.clone())
        .unwrap_or_default();
    let original_standard_id = derived.as_ref().and_then(|(_, original_standard_id)| {
        original_standard_id.clone().or_else(|| {
            source
                .source_metadata
                .pointer("/system/original_standard_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
    });

    let semantic_payload_hash = derive_semantic_payload_hash(source, memory_items)?;
    let raw_body_hash = derive_raw_body_hash(source, memory_items);
    let candidate_source_seed = if candidate_canonical_external_id.is_empty() {
        String::new()
    } else {
        source_seed(CANONICAL_ID_VERSION, &candidate_canonical_external_id)
    };
    let candidate_source_id = if candidate_canonical_external_id.is_empty() {
        source.source_id
    } else {
        deterministic_source_id(CANONICAL_ID_VERSION, &candidate_canonical_external_id)
    };

    let mut row = MigrationRowReport {
        legacy_source_id: source.source_id,
        legacy_external_id: source.external_id.clone(),
        candidate_canonical_external_id,
        candidate_source_seed,
        candidate_source_id,
        classification: MigrationClassification::Migratable,
        decision_reason: "LEGACY_ROW_MIGRATABLE".to_owned(),
        legacy_resolution_path: "legacy_only".to_owned(),
        canonical_id_version: CANONICAL_ID_VERSION.to_owned(),
        original_standard_id,
        semantic_payload_hash,
        raw_body_hash_present: raw_body_hash.is_some(),
        raw_body_hash,
        dependent_reference_counts: DependentReferenceCounts {
            memory_item: memory_items.len(),
            memory_index_job: jobs.len(),
            search_projection: 0,
        },
        planned_action: "rewrite".to_owned(),
    };

    if row.candidate_canonical_external_id.is_empty() {
        row.classification = MigrationClassification::Unmigratable;
        row.decision_reason = "LEGACY_ROW_UNMIGRATABLE_MISSING_CANONICAL_COMPONENT".to_owned();
        row.planned_action = "abort".to_owned();
    }

    Ok(row)
}

fn derive_candidate_external_id(
    source: &PersistedSourceRecord,
    memory_items: &[PersistedMemoryItemRecord],
) -> Option<(String, Option<String>)> {
    if let Ok(canonical) = CanonicalSourceExternalId::parse_canonical_uri(&source.external_id) {
        return Some((canonical.canonical_uri(), source.original_standard_id().map(ToOwned::to_owned)));
    }

    if source.document_type == "json" {
        let raw_body = memory_items.first()?.content.clone();
        let payload: Value = serde_json::from_str(&raw_body).ok()?;
        let canonical = canonicalize_direct_standard_payload(&payload).ok()?;
        return Some((
            canonical.external_id.canonical_uri(),
            Some(canonical.original_standard_id),
        ));
    }

    None
}

trait PersistedSourceRecordExt {
    fn original_standard_id(&self) -> Option<&str>;
}

impl PersistedSourceRecordExt for PersistedSourceRecord {
    fn original_standard_id(&self) -> Option<&str> {
        self.source_metadata
            .pointer("/system/original_standard_id")
            .and_then(Value::as_str)
    }
}

fn derive_semantic_payload_hash(
    source: &PersistedSourceRecord,
    memory_items: &[PersistedMemoryItemRecord],
) -> AppResult<String> {
    if let Some(value) = source
        .source_metadata
        .pointer("/system/semantic_payload_hash")
        .and_then(Value::as_str)
    {
        return Ok(value.to_owned());
    }
    if let Some(value) = source
        .source_metadata
        .pointer("/system/canonical_payload_hash")
        .and_then(Value::as_str)
    {
        return Ok(value.to_owned());
    }
    if source.document_type == "json" {
        if let Some(raw_body) = memory_items.first().map(|item| item.content.as_str()) {
            return normalized_json_hash_from_str(raw_body);
        }
    }
    Err(AppError::conflict(
        "legacy row is missing semantic payload material for migration",
    )
    .with_error_code("MIGRATION_STOP_CONDITION"))
}

fn derive_raw_body_hash(
    source: &PersistedSourceRecord,
    memory_items: &[PersistedMemoryItemRecord],
) -> Option<String> {
    source
        .source_metadata
        .pointer("/system/raw_body_hash")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            if source.document_type == "json" {
                memory_items
                    .first()
                    .map(|item| raw_body_hash_from_str(&item.content))
            } else {
                None
            }
        })
}

fn summarize_rows(rows: &[MigrationRowReport], snapshot_ready: bool) -> MigrationSummary {
    let migratable_rows = rows
        .iter()
        .filter(|row| row.classification == MigrationClassification::Migratable)
        .count();
    let consolidation_groups = rows
        .iter()
        .filter(|row| row.classification == MigrationClassification::Consolidate)
        .map(|row| row.candidate_canonical_external_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let unmigratable_rows = rows
        .iter()
        .filter(|row| row.classification == MigrationClassification::Unmigratable)
        .count();
    let conflict_groups = rows
        .iter()
        .filter(|row| row.decision_reason == "LEGACY_ROW_UNMIGRATABLE_DUPLICATE_CONFLICT")
        .map(|row| row.candidate_canonical_external_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let reference_gap_rows = rows
        .iter()
        .filter(|row| row.dependent_reference_counts.memory_item == 0)
        .count();

    MigrationSummary {
        total_rows: rows.len(),
        migratable_rows,
        consolidation_groups,
        unmigratable_rows,
        conflict_groups,
        reference_gap_rows,
        stop_required: !snapshot_ready || unmigratable_rows > 0 || conflict_groups > 0,
    }
}

fn rewrite_state(
    snapshot: PersistedAuthoritativeState,
    rows: &[MigrationRowReport],
) -> AppResult<PersistedAuthoritativeState> {
    let mut grouped_rows = BTreeMap::<Uuid, &MigrationRowReport>::new();
    for row in rows {
        grouped_rows.insert(row.legacy_source_id, row);
    }

    let source_by_id = snapshot
        .sources
        .iter()
        .map(|source| (source.source_id, source.clone()))
        .collect::<HashMap<_, _>>();
    let items_by_source = snapshot
        .memory_items
        .iter()
        .fold(BTreeMap::<Uuid, Vec<PersistedMemoryItemRecord>>::new(), |mut acc, item| {
            acc.entry(item.source_id).or_default().push(item.clone());
            acc
        });
    let jobs_by_source = snapshot
        .index_jobs
        .iter()
        .fold(BTreeMap::<Uuid, Vec<PersistedIndexJobRecord>>::new(), |mut acc, job| {
            acc.entry(job.source_id).or_default().push(job.clone());
            acc
        });

    let mut source_groups = BTreeMap::<Uuid, Vec<&MigrationRowReport>>::new();
    for row in rows {
        let group_key = row.candidate_source_id;
        source_groups.entry(group_key).or_default().push(row);
    }

    let urn_generator = DefaultIdGenerator;
    let mut new_sources = Vec::new();
    let mut new_items = Vec::new();
    let mut new_jobs = Vec::new();
    let mut remaps = HashMap::new();

    for (target_source_id, group_rows) in source_groups {
        let survivor = group_rows
            .iter()
            .filter_map(|row| source_by_id.get(&row.legacy_source_id))
            .min_by_key(|source| (source.created_at, source.source_id))
            .ok_or_else(|| AppError::internal("migration survivor selection failed"))?;
        let report_row = group_rows[0];

        let mut metadata = match &survivor.source_metadata {
            Value::Object(map) => map.clone(),
            _ => Map::new(),
        };
        metadata.remove("system");
        metadata.insert(
            "system".to_owned(),
            json!({
                "canonical_id_version": CANONICAL_ID_VERSION,
                "ingest_kind": if report_row.original_standard_id.is_some() {
                    IngestKind::DirectStandard
                } else {
                    IngestKind::Canonical
                },
                "semantic_payload_hash": report_row.semantic_payload_hash,
                "original_standard_id": report_row.original_standard_id,
                "raw_body_hash": report_row.raw_body_hash,
            }),
        );

        new_sources.push(PersistedSourceRecord {
            source_id: target_source_id,
            external_id: report_row.candidate_canonical_external_id.clone(),
            title: survivor.title.clone(),
            summary: survivor.summary.clone(),
            document_type: survivor.document_type.clone(),
            source_metadata: Value::Object(metadata),
            created_at: survivor.created_at,
            updated_at: survivor.updated_at,
        });

        for row in &group_rows {
            if row.legacy_source_id != target_source_id {
                remaps.insert(row.legacy_source_id, target_source_id);
            }
            for item in items_by_source.get(&row.legacy_source_id).into_iter().flatten() {
                let urn = urn_generator
                    .memory_item_urn(
                        target_source_id,
                        item.sequence,
                        item.start_offset,
                        item.end_offset,
                        &item.content_hash,
                    )
                    .to_string();
                if new_items.iter().any(|existing: &PersistedMemoryItemRecord| existing.urn == urn) {
                    continue;
                }
                new_items.push(PersistedMemoryItemRecord {
                    urn,
                    source_id: target_source_id,
                    sequence: item.sequence,
                    unit_type: item.unit_type.clone(),
                    start_offset: item.start_offset,
                    end_offset: item.end_offset,
                    version: item.version.clone(),
                    content: item.content.clone(),
                    content_hash: item.content_hash.clone(),
                    item_metadata: item.item_metadata.clone(),
                    created_at: item.created_at,
                    updated_at: item.updated_at,
                });
            }
            for job in jobs_by_source.get(&row.legacy_source_id).into_iter().flatten() {
                new_jobs.push(PersistedIndexJobRecord {
                    source_id: target_source_id,
                    ..job.clone()
                });
            }
        }
    }

    Ok(PersistedAuthoritativeState {
        sources: new_sources,
        memory_items: new_items,
        index_jobs: new_jobs,
        source_id_remaps: remaps,
        migration_phase: "verification".to_owned(),
        snapshot_ready: snapshot.snapshot_ready,
    })
}