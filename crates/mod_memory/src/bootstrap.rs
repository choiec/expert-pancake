use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use core_shared::ApiError;
use jsonschema::{Validator, validator_for};
use serde_json::{Value, json};
use tracing::warn;

use crate::domain::credential::{
    CredentialFamily, CredentialSearchResponse, SearchCredentialsQuery, StandardCredential,
    extract_type_tokens,
};
use crate::infra::repo::{CredentialRepository, CredentialRepositoryResult, SearchRepository};

#[async_trait]
pub trait DependencyProbe: Send + Sync {
    async fn is_ready(&self) -> bool;
}

#[async_trait]
pub trait ProjectionSync: Send + Sync {
    async fn sync_pending(&self) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct RegisterCredentialService {
    validator: Arc<CredentialValidator>,
    repository: Arc<dyn CredentialRepository>,
    projection_sync: Arc<dyn ProjectionSync>,
}

#[derive(Clone)]
pub struct GetCredentialService {
    repository: Arc<dyn CredentialRepository>,
}

#[derive(Clone)]
pub struct SearchCredentialsService {
    search_repository: Arc<dyn SearchRepository>,
}

#[derive(Clone)]
pub struct CredentialModule {
    pub register_service: RegisterCredentialService,
    pub get_service: GetCredentialService,
    pub search_service: SearchCredentialsService,
    pub authoritative_probe: Arc<dyn DependencyProbe>,
    pub search_probe: Arc<dyn DependencyProbe>,
}

#[derive(Debug)]
struct CredentialValidator {
    ob_schema: Validator,
    clr_schema: Validator,
}

impl CredentialModule {
    pub fn new(
        repository: Arc<dyn CredentialRepository>,
        projection_repository: Arc<dyn SearchRepository>,
        projection_sync: Arc<dyn ProjectionSync>,
        authoritative_probe: Arc<dyn DependencyProbe>,
        search_probe: Arc<dyn DependencyProbe>,
    ) -> Self {
        let validator = Arc::new(CredentialValidator::new());

        Self {
            register_service: RegisterCredentialService {
                validator,
                repository: repository.clone(),
                projection_sync,
            },
            get_service: GetCredentialService {
                repository: repository.clone(),
            },
            search_service: SearchCredentialsService {
                search_repository: projection_repository,
            },
            authoritative_probe,
            search_probe,
        }
    }
}

impl RegisterCredentialService {
    pub async fn register(&self, payload: Value) -> Result<CredentialRepositoryResult, ApiError> {
        let credential = self.validator.validate(payload)?;
        let outcome = self.repository.register(credential).await?;

        if let Err(error) = self.projection_sync.sync_pending().await {
            warn!(
                error_code = error.error_code(),
                message = error.message(),
                "projection sync degraded after authoritative write"
            );
        }

        Ok(outcome)
    }
}

impl GetCredentialService {
    pub async fn get(&self, credential_id: &str) -> Result<StandardCredential, ApiError> {
        self.repository
            .get(credential_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Credential was not found"))
    }
}

impl SearchCredentialsService {
    pub async fn search(
        &self,
        query: SearchCredentialsQuery,
    ) -> Result<CredentialSearchResponse, ApiError> {
        self.search_repository.search(&query).await
    }
}

impl CredentialValidator {
    fn new() -> Self {
        Self {
            ob_schema: validator_for(&open_badges_schema()).expect("Open Badges schema compiles"),
            clr_schema: validator_for(&clr_schema()).expect("CLR schema compiles"),
        }
    }

    fn validate(&self, payload: Value) -> Result<StandardCredential, ApiError> {
        let object = payload.as_object().ok_or_else(|| {
            ApiError::invalid_input("Credential payload must be a JSON object", None)
        })?;

        let family = classify_family(object)?;
        enforce_allowed_top_level_keys(object, family)?;

        match family {
            CredentialFamily::OpenBadgesV3 => validate_against_schema(&self.ob_schema, &payload)?,
            CredentialFamily::ClrV2 => validate_against_schema(&self.clr_schema, &payload)?,
        }

        let credential_id = object
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ApiError::invalid_input("Credential id must be a string", None))?
            .to_string();

        if credential_id.is_empty() {
            return Err(ApiError::invalid_input(
                "Credential id must not be empty",
                None,
            ));
        }

        Ok(StandardCredential::new(credential_id, family, payload))
    }
}

fn classify_family(object: &serde_json::Map<String, Value>) -> Result<CredentialFamily, ApiError> {
    let type_value = object.get("type").ok_or_else(|| {
        ApiError::invalid_input(
            "Credential type is required for family classification",
            None,
        )
    })?;
    let tokens = extract_type_tokens(type_value);
    let has_open_badges = tokens
        .iter()
        .any(|token| token == "OpenBadgeCredential" || token == "AchievementCredential");
    let has_clr = tokens.iter().any(|token| token == "ClrCredential");

    match (has_open_badges, has_clr) {
        (true, false) => Ok(CredentialFamily::OpenBadgesV3),
        (false, true) => Ok(CredentialFamily::ClrV2),
        _ => Err(ApiError::invalid_input(
            "Credential payload could not be classified to exactly one supported family",
            Some(json!({
                "supported_families": ["open_badges_v3", "clr_v2"],
                "type": type_value,
            })),
        )),
    }
}

fn enforce_allowed_top_level_keys(
    object: &serde_json::Map<String, Value>,
    family: CredentialFamily,
) -> Result<(), ApiError> {
    let allowed = allowed_keys_for_family(family);
    let unsupported = object
        .keys()
        .filter(|key| !allowed.contains(key.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(ApiError::invalid_input(
            "Credential payload includes unsupported top-level fields",
            Some(json!({ "unsupported_fields": unsupported })),
        ))
    }
}

fn allowed_keys_for_family(family: CredentialFamily) -> BTreeSet<&'static str> {
    let mut allowed = BTreeSet::from([
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
    ]);

    if family == CredentialFamily::ClrV2 {
        allowed.insert("partial");
    }

    allowed
}

fn validate_against_schema(schema: &Validator, payload: &Value) -> Result<(), ApiError> {
    let errors = schema
        .iter_errors(payload)
        .map(|error| Value::String(error.to_string()))
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::invalid_input(
            "Credential payload does not satisfy the pinned family schema",
            Some(Value::Array(errors)),
        ))
    }
}

fn open_badges_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["@context", "type", "id", "issuer", "credentialSubject", "proof", "validFrom"],
        "properties": standard_credential_properties()
    })
}

fn clr_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["@context", "type", "id", "issuer", "name", "credentialSubject", "proof", "validFrom"],
        "properties": standard_credential_properties()
    })
}

fn standard_credential_properties() -> serde_json::Map<String, Value> {
    serde_json::Map::from_iter([
        ("@context".to_string(), flexible_string_or_array_schema()),
        ("awardedDate".to_string(), json!({"type": "string"})),
        (
            "credentialSchema".to_string(),
            object_or_array_of_objects_schema(),
        ),
        (
            "credentialStatus".to_string(),
            object_or_array_of_objects_schema(),
        ),
        ("credentialSubject".to_string(), json!({"type": "object"})),
        ("description".to_string(), string_or_object_schema()),
        (
            "endorsement".to_string(),
            object_or_array_of_objects_schema(),
        ),
        (
            "endorsementJwt".to_string(),
            string_or_array_of_strings_schema(),
        ),
        ("evidence".to_string(), object_or_array_of_objects_schema()),
        ("id".to_string(), json!({"type": "string"})),
        ("image".to_string(), string_or_object_schema()),
        ("issuer".to_string(), string_or_object_schema()),
        ("name".to_string(), string_or_object_schema()),
        ("partial".to_string(), json!({"type": "boolean"})),
        ("proof".to_string(), object_or_array_of_objects_schema()),
        (
            "refreshService".to_string(),
            object_or_array_of_objects_schema(),
        ),
        (
            "termsOfUse".to_string(),
            object_or_array_of_objects_schema(),
        ),
        ("type".to_string(), flexible_string_or_array_schema()),
        ("validFrom".to_string(), json!({"type": "string"})),
        ("validUntil".to_string(), json!({"type": "string"})),
    ])
}

fn flexible_string_or_array_schema() -> Value {
    json!({
        "oneOf": [
            {"type": "string"},
            {"type": "array", "items": {}}
        ]
    })
}

fn string_or_object_schema() -> Value {
    json!({
        "oneOf": [
            {"type": "string"},
            {"type": "object"}
        ]
    })
}

fn object_or_array_of_objects_schema() -> Value {
    json!({
        "oneOf": [
            {"type": "object"},
            {"type": "array", "items": {"type": "object"}}
        ]
    })
}

fn string_or_array_of_strings_schema() -> Value {
    json!({
        "oneOf": [
            {"type": "string"},
            {"type": "array", "items": {"type": "string"}}
        ]
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::CredentialValidator;
    use crate::domain::credential::CredentialFamily;

    #[test]
    fn classifies_open_badges() {
        let validator = CredentialValidator::new();
        let credential = validator
            .validate(json!({
                "@context": [
                    "https://www.w3.org/ns/credentials/v2",
                    "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"
                ],
                "type": ["VerifiableCredential", "OpenBadgeCredential"],
                "id": "urn:example:badge:001",
                "issuer": { "id": "https://issuer.example.org" },
                "credentialSubject": { "achievement": { "name": "Rust" } },
                "proof": { "type": "DataIntegrityProof" },
                "validFrom": "2026-01-01T00:00:00Z"
            }))
            .expect("valid credential");

        assert_eq!(credential.family, CredentialFamily::OpenBadgesV3);
    }
}
