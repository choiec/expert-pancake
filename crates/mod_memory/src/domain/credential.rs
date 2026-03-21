use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialFamily {
    OpenBadgesV3,
    ClrV2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardCredential {
    pub credential_id: String,
    pub family: CredentialFamily,
    pub credential: Value,
    pub semantic_payload_hash: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationStatus {
    Created,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialSearchProjection {
    pub credential_id: String,
    pub family: CredentialFamily,
    pub name: Option<Value>,
    pub issuer: Option<Value>,
    pub issuer_id: Option<String>,
    pub credential_type: Value,
    pub valid_from: Option<String>,
    pub preview: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialSearchHit {
    pub credential: Value,
    pub family: CredentialFamily,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialSearchResponse {
    pub items: Vec<CredentialSearchHit>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchCredentialsQuery {
    pub q: Option<String>,
    pub family: Option<CredentialFamily>,
    pub issuer_id: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialIndexJobStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialIndexJob {
    pub job_id: Uuid,
    pub credential_id: String,
    pub status: CredentialIndexJobStatus,
    pub retry_count: u32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl StandardCredential {
    pub fn new(credential_id: String, family: CredentialFamily, credential: Value) -> Self {
        let now = OffsetDateTime::now_utc();
        let normalized = normalize_json(&credential);
        let semantic_payload_hash = semantic_payload_hash(&normalized);

        Self {
            credential_id,
            family,
            credential: normalized,
            semantic_payload_hash,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn to_projection(&self) -> CredentialSearchProjection {
        let name = self.credential.get("name").cloned();
        let issuer = self.credential.get("issuer").cloned();
        let issuer_id = issuer.as_ref().and_then(extract_issuer_id);
        let preview = build_preview(name.as_ref(), issuer.as_ref());

        CredentialSearchProjection {
            credential_id: self.credential_id.clone(),
            family: self.family,
            name,
            issuer,
            issuer_id,
            credential_type: self
                .credential
                .get("type")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
            valid_from: self
                .credential
                .get("validFrom")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            preview,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn response_document(&self) -> Value {
        self.credential.clone()
    }
}

impl CredentialSearchProjection {
    pub fn to_hit(&self, score: Option<f64>) -> CredentialSearchHit {
        let mut credential = Map::new();
        credential.insert("id".to_string(), Value::String(self.credential_id.clone()));
        credential.insert("type".to_string(), self.credential_type.clone());
        if let Some(name) = &self.name {
            credential.insert("name".to_string(), name.clone());
        }
        if let Some(issuer) = &self.issuer {
            credential.insert("issuer".to_string(), issuer.clone());
        }
        if let Some(valid_from) = &self.valid_from {
            credential.insert("validFrom".to_string(), Value::String(valid_from.clone()));
        }

        CredentialSearchHit {
            credential: Value::Object(credential),
            family: self.family,
            score,
            preview: self.preview.clone(),
        }
    }
}

pub fn normalize_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sorted = object.iter().collect::<Vec<_>>();
            sorted.sort_by(|left, right| left.0.cmp(right.0));

            let mut normalized = Map::new();
            for (key, value) in sorted {
                normalized.insert(key.clone(), normalize_json(value));
            }

            Value::Object(normalized)
        }
        Value::Array(items) => Value::Array(items.iter().map(normalize_json).collect()),
        _ => value.clone(),
    }
}

pub fn semantic_payload_hash(value: &Value) -> String {
    let canonical = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    format!("{:x}", hasher.finalize())
}

pub fn extract_type_tokens(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

pub fn extract_stringish(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Object(object) => object
            .values()
            .find_map(Value::as_str)
            .map(ToOwned::to_owned),
        _ => None,
    }
}

pub fn extract_issuer_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Object(object) => object
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        _ => None,
    }
}

fn build_preview(name: Option<&Value>, issuer: Option<&Value>) -> Option<String> {
    let name = name.and_then(extract_stringish);
    let issuer = issuer.and_then(extract_stringish);

    match (name, issuer) {
        (Some(name), Some(issuer)) => Some(format!("{name} · {issuer}")),
        (Some(name), None) => Some(name),
        (None, Some(issuer)) => Some(issuer),
        (None, None) => None,
    }
}

pub fn conflict_details(existing: &StandardCredential, incoming: &StandardCredential) -> Value {
    json!({
        "credential_id": existing.credential_id,
        "existing_hash": existing.semantic_payload_hash,
        "incoming_hash": incoming.semantic_payload_hash,
    })
}
