use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use core_infra::surrealdb::{
    CommitRegistrationOutcome, InMemorySurrealDb, PersistedMemoryItemRecord, PersistedSourceRecord,
    PersistedStandardCredentialRecord, SearchProjectionRecord,
};
use core_shared::{AppError, AppResult, DefaultIdGenerator, IdGenerator};
use jsonschema::Validator;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    memory_item::MemoryItem,
    normalization::{NormalizationInput, normalize_source, normalized_json_hash, sha256_hex},
    source::{
        CANONICAL_ID_VERSION, CanonicalSource, DocumentType, StandardFamily,
        canonical_external_id_for_standard, derive_source_id, is_canonical_external_id,
    },
};

const OPEN_BADGES_ACHIEVEMENT_CREDENTIAL_SCHEMA: &str = include_str!(
    "../../../specs/001-memory-ingest/contracts/1edtech/ob_v3p0_achievementcredential_schema.json"
);
const CLR_CREDENTIAL_SCHEMA: &str = include_str!(
    "../../../specs/001-memory-ingest/contracts/1edtech/clr_v2p0_clrcredential_schema.json"
);

static OPEN_BADGES_VALIDATOR: OnceLock<Validator> = OnceLock::new();
static CLR_VALIDATOR: OnceLock<Validator> = OnceLock::new();

#[derive(Clone)]
pub struct MemoryModule {
    db: Arc<InMemorySurrealDb>,
    id_generator: Arc<dyn IdGenerator>,
    normalization_timeout: Duration,
}

impl std::fmt::Debug for MemoryModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryModule").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSourcePayload {
    pub title: String,
    pub summary: Option<String>,
    pub external_id: String,
    pub document_type: String,
    pub content: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSourceOutcome {
    pub created: bool,
    pub source: SourceView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceView {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: String,
    pub created_at: OffsetDateTime,
    pub indexing_status: String,
    pub source_metadata: Value,
    pub memory_items: Vec<MemoryItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItemView {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub item_metadata: Value,
    pub source_metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub source_id: Option<Uuid>,
    pub document_type: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitView {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub items: Vec<SearchProjectionRecord>,
}

#[derive(Debug, Clone)]
struct ParsedRegistration {
    canonical: CanonicalSource,
    standard_credential: Option<PersistedStandardCredentialRecord>,
}

impl MemoryModule {
    pub fn fixture(db: Arc<InMemorySurrealDb>, normalization_timeout: Duration) -> Self {
        Self {
            db,
            id_generator: Arc::new(DefaultIdGenerator),
            normalization_timeout,
        }
    }

    pub fn register_source(&self, raw_body: &str) -> AppResult<RegisterSourceOutcome> {
        let started_at = OffsetDateTime::now_utc();
        let parsed = parse_register_payload(raw_body)?;
        let source = parsed.canonical;
        let source_id = derive_source_id(&source.external_id);
        let normalized = normalize_source(
            &NormalizationInput {
                source_id,
                external_id: &source.external_id,
                title: &source.title,
                summary: source.summary.as_deref(),
                document_type: source.document_type,
                authoritative_content: &source.content,
                source_metadata: &source.metadata,
                created_at: started_at,
            },
            self.id_generator.as_ref(),
        )?;

        let timeout = self.normalization_timeout;
        if timeout.as_secs() == 0 {
            return Err(AppError::timeout("normalization timed out"));
        }

        let persisted_source = PersistedSourceRecord {
            source_id,
            external_id: source.external_id.clone(),
            title: source.title,
            summary: source.summary,
            document_type: source.document_type.as_str().to_owned(),
            source_metadata: source.metadata,
            canonical_hash: source.canonical_hash,
            created_at: started_at,
            updated_at: started_at,
        };
        let persisted_items = normalized
            .into_iter()
            .map(into_persisted_memory_item)
            .collect();

        let committed = self.db.commit_registration(
            persisted_source,
            persisted_items,
            parsed.standard_credential,
        )?;
        match committed {
            CommitRegistrationOutcome::Created(bundle) => Ok(RegisterSourceOutcome {
                created: true,
                source: bundle_to_source_view(bundle),
            }),
            CommitRegistrationOutcome::Replay(bundle) => Ok(RegisterSourceOutcome {
                created: false,
                source: bundle_to_source_view(bundle),
            }),
        }
    }

    pub fn get_source(&self, source_id: Uuid) -> AppResult<SourceView> {
        let bundle = self
            .db
            .get_source_bundle(source_id)
            .ok_or_else(|| AppError::not_found("source was not found"))?;
        Ok(bundle_to_source_view(bundle))
    }

    pub fn get_memory_item(&self, urn: &str) -> AppResult<MemoryItemView> {
        let (item, source, _) = self
            .db
            .get_memory_item(urn)
            .ok_or_else(|| AppError::not_found("memory item was not found"))?;
        Ok(memory_item_view(item, &source.source_metadata))
    }

    pub fn search_memory_items(&self, query: SearchQuery) -> AppResult<SearchHitView> {
        let (total, items) = self.db.search(
            query.q.as_deref(),
            query.source_id,
            query.document_type.as_deref(),
            query.limit,
            query.offset,
        )?;
        Ok(SearchHitView {
            total,
            limit: query.limit,
            offset: query.offset,
            items,
        })
    }
}

fn parse_register_payload(raw_body: &str) -> AppResult<ParsedRegistration> {
    let value: Value = serde_json::from_str(raw_body)
        .map_err(|_| AppError::validation("request body must be valid json"))?;

    if value.get("external-id").is_some()
        || value.get("document-type").is_some()
        || value.get("content").is_some()
    {
        let title = required_string(&value, "title")?;
        let external_id = required_string(&value, "external-id")?;
        let document_type = match required_string(&value, "document-type")?.as_str() {
            "text" => DocumentType::Text,
            "markdown" => DocumentType::Markdown,
            other => {
                return Err(AppError::validation(format!(
                    "unsupported document-type '{other}'"
                )));
            }
        };
        let content = required_string(&value, "content")?;
        let summary = optional_string(&value, "summary");
        if !is_canonical_external_id(&external_id) {
            return Err(AppError::validation(
                "canonical/manual ingest requires a canonical project-owned external-id",
            ));
        }
        let user_metadata = metadata_object(&value)?;
        let canonical_hash = sha256_hex(&canonical_json_shape(&json!({
            "title": &title,
            "summary": &summary,
            "external-id": &external_id,
            "document-type": document_type.as_str(),
            "content": &content,
            "metadata": &user_metadata,
        })));
        return Ok(ParsedRegistration {
            canonical: CanonicalSource {
                title,
                summary,
                external_id,
                document_type,
                content,
                metadata: attach_system_metadata(user_metadata, "canonical", &canonical_hash, None),
                canonical_hash,
            },
            standard_credential: None,
        });
    }

    let family = validate_and_classify_standard_payload(&value)?;
    validate_standard_credential_for_certification(&value, family)?;
    let title = standard_title(&value, family)?;
    let original_standard_id = required_string(&value, "id")?;
    let source_domain = trusted_source_domain(&value, &original_standard_id)?;
    let external_id =
        canonical_external_id_for_standard(family, &source_domain, &original_standard_id);
    let canonical_hash = normalized_json_hash(raw_body)?;

    Ok(ParsedRegistration {
        canonical: CanonicalSource {
            title: title.clone(),
            summary: None,
            external_id,
            document_type: DocumentType::Json,
            content: raw_body.to_owned(),
            metadata: attach_system_metadata(
                json!({}),
                "direct_standard",
                &canonical_hash,
                Some(original_standard_id.clone()),
            ),
            canonical_hash: canonical_hash.clone(),
        },
        standard_credential: Some(build_standard_credential_record(family, &value)?),
    })
}

fn validate_and_classify_standard_payload(value: &Value) -> AppResult<StandardFamily> {
    let open_badges_errors = schema_errors(open_badges_validator(), value);
    let clr_errors = schema_errors(clr_validator(), value);

    match (open_badges_errors.is_empty(), clr_errors.is_empty()) {
        (true, false) => Ok(StandardFamily::OpenBadges),
        (false, true) => Ok(StandardFamily::Clr),
        (true, true) => {
            if value_contains_token(value.get("type"), "ClrCredential") {
                Ok(StandardFamily::Clr)
            } else if value_contains_token(value.get("type"), "AchievementCredential")
                || value_contains_token(value.get("type"), "OpenBadgeCredential")
            {
                Ok(StandardFamily::OpenBadges)
            } else {
                Err(AppError::validation(
                    "supported-standard payload is shape-valid but unmappable",
                )
                .with_error_code("INVALID_STANDARD_PAYLOAD"))
            }
        }
        (false, false) => Err(AppError::validation(
            "supported-standard payload failed pinned 1EdTech envelope validation",
        )
        .with_details(json!({
            "open_badges": open_badges_errors,
            "clr": clr_errors,
        }))),
    }
}

fn validate_standard_credential_for_certification(
    value: &Value,
    family: StandardFamily,
) -> AppResult<()> {
    let contexts = string_tokens(value.get("@context"));
    let missing_contexts = required_contexts_for(family)
        .into_iter()
        .filter(|context| !contexts.iter().any(|candidate| candidate == context))
        .collect::<Vec<_>>();
    if !missing_contexts.is_empty() {
        return Err(AppError::validation(
            "supported-standard payload is missing required certification contexts",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD")
        .with_details(json!({ "missing_contexts": missing_contexts })));
    }

    let types = string_tokens(value.get("type"));
    let missing_types = required_types_for(family)
        .into_iter()
        .filter(|kind| !types.iter().any(|candidate| candidate == kind))
        .collect::<Vec<_>>();
    if !missing_types.is_empty() {
        return Err(AppError::validation(
            "supported-standard payload is missing required certification types",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD")
        .with_details(json!({ "missing_types": missing_types })));
    }

    let issuer_id = issuer_id(value)?;
    if extract_host(&issuer_id).is_none() {
        return Err(AppError::validation(
            "supported-standard payload issuer id must be a URI with a host",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD"));
    }

    let credential_schema = json_array(value.get("credentialSchema"));
    if credential_schema.is_empty() {
        return Err(AppError::validation(
            "supported-standard payload must include credentialSchema for certification-level ingest",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD"));
    }

    let expected_schema_id = match family {
        StandardFamily::OpenBadges => {
            "https://purl.imsglobal.org/spec/ob/v3p0/schema/json/ob_v3p0_achievementcredential_schema.json"
        }
        StandardFamily::Clr => {
            "https://purl.imsglobal.org/spec/clr/v2p0/schema/json/clr_v2p0_clrcredential_schema.json"
        }
    };
    let credential_schema_ok = credential_schema.iter().any(|entry| {
        entry.get("id").and_then(Value::as_str) == Some(expected_schema_id)
            && entry.get("type").and_then(Value::as_str) == Some("1EdTechJsonSchemaValidator2019")
    });
    if !credential_schema_ok {
        return Err(AppError::validation(
            "supported-standard payload credentialSchema must pin the official 1EdTech validator",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD")
        .with_details(json!({
            "expected_schema_id": expected_schema_id,
            "expected_schema_type": "1EdTechJsonSchemaValidator2019",
        })));
    }

    let proofs = proof_entries(value)?;
    let supported_proof_types = [
        "DataIntegrityProof",
        "Ed25519Signature2018",
        "Ed25519Signature2020",
        "JsonWebSignature2020",
    ];
    for proof in &proofs {
        let proof_type = required_object_string(proof, "type")?;
        if !supported_proof_types.contains(&proof_type.as_str()) {
            return Err(AppError::validation(
                "supported-standard payload proof type is not supported",
            )
            .with_error_code("INVALID_STANDARD_PAYLOAD")
            .with_details(json!({
                "supported_proof_types": supported_proof_types,
                "actual": proof_type,
            })));
        }

        let proof_purpose = required_object_string(proof, "proofPurpose")?;
        if proof_purpose != "assertionMethod" {
            return Err(AppError::validation(
                "supported-standard payload proofPurpose must be assertionMethod",
            )
            .with_error_code("INVALID_STANDARD_PAYLOAD"));
        }

        let verification_method = required_object_string(proof, "verificationMethod")?;
        if verification_method.trim().is_empty() {
            return Err(AppError::validation(
                "supported-standard payload proof verificationMethod must be non-empty",
            )
            .with_error_code("INVALID_STANDARD_PAYLOAD"));
        }

        let _created = required_object_string(proof, "created")?;
        let has_jws = proof
            .get("jws")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|value| value.split('.').count() == 3);
        let has_proof_value = proof
            .get("proofValue")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        if !has_jws && !has_proof_value {
            return Err(AppError::validation(
                "supported-standard payload proof must include a compact jws or proofValue",
            )
            .with_error_code("INVALID_STANDARD_PAYLOAD"));
        }
    }

    Ok(())
}

fn build_standard_credential_record(
    family: StandardFamily,
    value: &Value,
) -> AppResult<PersistedStandardCredentialRecord> {
    let filtered = canonicalize_json_value(&filter_schema_fields(value, family));
    if filtered.is_object() {
        Ok(filtered)
    } else {
        Err(AppError::internal(
            "supported-standard payload must remain a json object after schema filtering",
        ))
    }
}

fn standard_title(value: &Value, family: StandardFamily) -> AppResult<String> {
    if let Some(name) = localized_string(value.get("name")) {
        return Ok(name);
    }

    if family == StandardFamily::OpenBadges {
        if let Some(name) = localized_string(value.pointer("/credentialSubject/achievement/name")) {
            return Ok(name);
        }
    }

    Err(
        AppError::validation("supported-standard payload is shape-valid but unmappable")
            .with_error_code("INVALID_STANDARD_PAYLOAD"),
    )
}

fn required_contexts_for(family: StandardFamily) -> Vec<&'static str> {
    let mut contexts = vec!["https://www.w3.org/ns/credentials/v2"];
    match family {
        StandardFamily::OpenBadges => {
            contexts.push("https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json");
        }
        StandardFamily::Clr => {
            contexts.push("https://purl.imsglobal.org/spec/clr/v2p0/context-2.0.1.json");
        }
    }
    contexts
}

fn required_types_for(family: StandardFamily) -> Vec<&'static str> {
    let mut types = vec!["VerifiableCredential"];
    match family {
        StandardFamily::OpenBadges => types.push("AchievementCredential"),
        StandardFamily::Clr => types.push("ClrCredential"),
    }
    types
}

fn localized_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        }
        Value::Object(map) => {
            let mut entries = map
                .iter()
                .filter_map(|(locale, value)| value.as_str().map(|text| (locale, text.trim())))
                .filter(|(_, text)| !text.is_empty())
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            entries.first().map(|(_, text)| (*text).to_owned())
        }
        _ => None,
    }
}

fn value_contains_token(value: Option<&Value>, expected: &str) -> bool {
    match value {
        Some(Value::String(token)) => token == expected,
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some(expected)),
        _ => false,
    }
}

fn string_tokens(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(token)) => vec![token.trim().to_owned()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn json_array(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(values)) => values.clone(),
        Some(other) => vec![other.clone()],
        None => Vec::new(),
    }
}

fn proof_entries(value: &Value) -> AppResult<Vec<Value>> {
    let proofs = json_array(value.get("proof"));
    if proofs.is_empty() {
        return Err(AppError::validation(
            "supported-standard payload must include proof for certification-level ingest",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD"));
    }
    if proofs.iter().any(|proof| !proof.is_object()) {
        return Err(AppError::validation(
            "supported-standard payload proof must be an object or array of objects",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD"));
    }
    Ok(proofs)
}

fn issuer_id(value: &Value) -> AppResult<String> {
    value
        .get("issuer")
        .and_then(|issuer| issuer.get("id").or(Some(issuer)))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::validation("supported-standard payload issuer must contain a string id")
                .with_error_code("INVALID_STANDARD_PAYLOAD")
        })
}

fn required_object_string(value: &Value, key: &str) -> AppResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::validation(format!(
                "supported-standard payload object field '{key}' is required"
            ))
            .with_error_code("INVALID_STANDARD_PAYLOAD")
        })
}

fn schema_errors(validator: &Validator, value: &Value) -> Vec<String> {
    validator
        .iter_errors(value)
        .take(5)
        .map(|error| error.to_string())
        .collect()
}

fn open_badges_validator() -> &'static Validator {
    OPEN_BADGES_VALIDATOR.get_or_init(|| {
        compile_validator(
            OPEN_BADGES_ACHIEVEMENT_CREDENTIAL_SCHEMA,
            "Open Badges 3.0 AchievementCredential",
        )
    })
}

fn clr_validator() -> &'static Validator {
    CLR_VALIDATOR.get_or_init(|| compile_validator(CLR_CREDENTIAL_SCHEMA, "CLR 2.0 ClrCredential"))
}

fn compile_validator(schema_text: &str, schema_name: &str) -> Validator {
    let schema: Value = serde_json::from_str(schema_text)
        .unwrap_or_else(|error| panic!("failed to parse pinned {schema_name} schema: {error}"));
    jsonschema::validator_for(&schema)
        .unwrap_or_else(|error| panic!("failed to compile pinned {schema_name} schema: {error}"))
}

fn trusted_source_domain(value: &Value, original_standard_id: &str) -> AppResult<String> {
    if let Some(issuer_id) = value
        .get("issuer")
        .and_then(|issuer| issuer.get("id").or(Some(issuer)))
        .and_then(Value::as_str)
        .and_then(extract_host)
    {
        return Ok(issuer_id);
    }

    extract_host(original_standard_id).ok_or_else(|| {
        AppError::validation("supported-standard payload is shape-valid but unmappable")
            .with_error_code("INVALID_STANDARD_PAYLOAD")
    })
}

fn required_string(value: &Value, key: &str) -> AppResult<String> {
    let field = value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::validation(format!("'{key}' is required")))?;
    Ok(field.to_owned())
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn metadata_object(value: &Value) -> AppResult<Value> {
    match value.get("metadata") {
        Some(Value::Object(_)) => Ok(value.get("metadata").cloned().unwrap_or_else(|| json!({}))),
        Some(_) => Err(AppError::validation("'metadata' must be an object")),
        None => Ok(json!({})),
    }
}

fn attach_system_metadata(
    metadata: Value,
    ingest_kind: &str,
    semantic_payload_hash: &str,
    original_standard_id: Option<String>,
) -> Value {
    let mut metadata = match metadata {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    metadata.insert(
        "system".to_owned(),
        json!({
            "canonical_id_version": CANONICAL_ID_VERSION,
            "ingest_kind": ingest_kind,
            "semantic_payload_hash": semantic_payload_hash,
            "original_standard_id": original_standard_id,
        }),
    );

    Value::Object(metadata)
}

fn extract_host(value: &str) -> Option<String> {
    value.parse::<http::Uri>().ok()?.host().map(str::to_owned)
}

fn canonical_json_shape(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        Value::Array(values) => {
            let inner = values
                .iter()
                .map(canonical_json_shape)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{inner}]")
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let inner = entries
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key).unwrap_or_default();
                    format!("{key}:{}", canonical_json_shape(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json_value).collect()),
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let mut normalized = serde_json::Map::new();
            for (key, value) in entries {
                normalized.insert(key.clone(), canonicalize_json_value(value));
            }
            Value::Object(normalized)
        }
        other => other.clone(),
    }
}

fn filter_schema_fields(value: &Value, family: StandardFamily) -> Value {
    let allowed_keys: &[&str] = match family {
        StandardFamily::OpenBadges => &[
            "@context",
            "awardedDate",
            "credentialSchema",
            "credentialStatus",
            "credentialSubject",
            "description",
            "endorsement",
            "endorsementJwt",
            "evidence",
            "id",
            "image",
            "issuer",
            "name",
            "proof",
            "refreshService",
            "termsOfUse",
            "type",
            "validFrom",
            "validUntil",
        ],
        StandardFamily::Clr => &[
            "@context",
            "awardedDate",
            "credentialSchema",
            "credentialStatus",
            "credentialSubject",
            "description",
            "endorsement",
            "endorsementJwt",
            "evidence",
            "id",
            "image",
            "issuer",
            "name",
            "partial",
            "proof",
            "refreshService",
            "termsOfUse",
            "type",
            "validFrom",
            "validUntil",
        ],
    };

    let mut filtered = serde_json::Map::new();
    if let Value::Object(map) = value {
        for key in allowed_keys {
            if let Some(entry) = map.get(*key) {
                filtered.insert((*key).to_owned(), entry.clone());
            }
        }
    }
    Value::Object(filtered)
}

fn into_persisted_memory_item(item: MemoryItem) -> PersistedMemoryItemRecord {
    PersistedMemoryItemRecord {
        urn: item.urn,
        source_id: item.source_id,
        sequence: item.sequence,
        unit_type: item.unit_type.as_str().to_owned(),
        start_offset: item.start_offset,
        end_offset: item.end_offset,
        version: item.version,
        content: item.content,
        content_hash: item.content_hash,
        item_metadata: item.item_metadata,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

fn bundle_to_source_view(bundle: core_infra::surrealdb::PersistedSourceBundle) -> SourceView {
    SourceView {
        source_id: bundle.source.source_id,
        external_id: bundle.source.external_id.clone(),
        title: bundle.source.title.clone(),
        summary: bundle.source.summary.clone(),
        document_type: bundle.source.document_type.clone(),
        created_at: bundle.source.created_at,
        indexing_status: bundle.indexing_status,
        source_metadata: bundle.source.source_metadata.clone(),
        memory_items: bundle
            .memory_items
            .into_iter()
            .map(|item| memory_item_view(item, &bundle.source.source_metadata))
            .collect(),
    }
}

fn memory_item_view(item: PersistedMemoryItemRecord, source_metadata: &Value) -> MemoryItemView {
    MemoryItemView {
        urn: item.urn,
        source_id: item.source_id,
        sequence: item.sequence,
        content: item.content,
        created_at: item.created_at,
        updated_at: item.updated_at,
        item_metadata: item.item_metadata,
        source_metadata: source_metadata.clone(),
    }
}
