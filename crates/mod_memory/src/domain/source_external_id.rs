use serde_json::{Map, Value};

use core_shared::{AppError, AppResult};

use crate::domain::normalization::{normalize_object_id, normalize_source_domain};

const CANONICAL_NAMESPACE_PREFIX: &str = "https://api.cherry-pick.net/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSourceExternalId {
    standard: String,
    version: String,
    source_domain: String,
    object_id_raw: String,
    object_id_normalized: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectStandardProfile {
    OpenBadgesAchievementCredential,
    ClrCredential,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectStandardCanonicalization {
    pub external_id: CanonicalSourceExternalId,
    pub original_standard_id: String,
    pub title: String,
    pub profile: DirectStandardProfile,
}

impl CanonicalSourceExternalId {
    pub fn parse_canonical_uri(uri: &str) -> AppResult<Self> {
        let remainder = uri
            .strip_prefix(CANONICAL_NAMESPACE_PREFIX)
            .ok_or_else(|| AppError::validation("external_id must use the canonical namespace"))?;

        let mut parts = remainder.splitn(3, '/');
        let standard = parts
            .next()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::validation("external_id is missing the standard segment"))?;
        let version = parts
            .next()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::validation("external_id is missing the version segment"))?;
        let tail = parts
            .next()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::validation("external_id is missing the identity segment"))?;

        ensure_supported_standard_version(standard, version)?;

        let Some(separator) = tail.find(':') else {
            return Err(AppError::validation(
                "external_id must include source_domain and object_id",
            ));
        };
        let source_domain = &tail[..separator];
        let object_id_normalized = &tail[separator + 1..];
        if source_domain.is_empty() || object_id_normalized.is_empty() {
            return Err(AppError::validation(
                "external_id must include non-empty source_domain and object_id",
            ));
        }

        let normalized_domain = normalize_source_domain(source_domain)?;
        if normalized_domain != source_domain {
            return Err(AppError::validation(
                "external_id source_domain is not in canonical normalized form",
            ));
        }

        let raw_object_id = percent_decode_utf8(object_id_normalized)?;
        let normalized_object_id = normalize_object_id(&raw_object_id)?;
        if normalized_object_id != object_id_normalized {
            return Err(AppError::validation(
                "external_id object_id is not in canonical normalized form",
            ));
        }

        Ok(Self {
            standard: standard.to_owned(),
            version: version.to_owned(),
            source_domain: normalized_domain,
            object_id_raw: raw_object_id.trim().to_owned(),
            object_id_normalized: normalized_object_id,
        })
    }

    pub fn from_components(
        standard: &str,
        version: &str,
        source_domain: &str,
        object_id_raw: &str,
    ) -> AppResult<Self> {
        ensure_supported_standard_version(standard, version)?;
        Ok(Self {
            standard: standard.to_owned(),
            version: version.to_owned(),
            source_domain: normalize_source_domain(source_domain)?,
            object_id_raw: object_id_raw.trim().to_owned(),
            object_id_normalized: normalize_object_id(object_id_raw)?,
        })
    }

    pub fn canonical_uri(&self) -> String {
        format!(
            "{CANONICAL_NAMESPACE_PREFIX}{}/{}/{}:{}",
            self.standard, self.version, self.source_domain, self.object_id_normalized
        )
    }

    pub fn standard(&self) -> &str {
        &self.standard
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn source_domain(&self) -> &str {
        &self.source_domain
    }

    pub fn object_id_raw(&self) -> &str {
        &self.object_id_raw
    }
}

pub fn canonicalize_direct_standard_payload(
    value: &Value,
) -> AppResult<DirectStandardCanonicalization> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::validation("request body must be a JSON object"))?;
    let types = string_or_string_array(object, "type")?;
    let original_standard_id = required_trimmed_string(object, "id")?;
    let title = required_trimmed_string(object, "name")?;

    let profile = if has_type(&types, &["OpenBadgeCredential", "AchievementCredential"]) {
        DirectStandardProfile::OpenBadgesAchievementCredential
    } else if has_type(&types, &["ClrCredential", "CLRCredential"])
        || object.get("credentialSubject").is_some()
    {
        DirectStandardProfile::ClrCredential
    } else {
        return Err(AppError::validation(
            "supported-standard payload could not be mapped to a supported standard",
        )
        .with_error_code("INVALID_STANDARD_PAYLOAD"));
    };

    let source_domain = trusted_domain_for_profile(object, profile)?;
    let (standard, version) = match profile {
        DirectStandardProfile::OpenBadgesAchievementCredential => ("ob", "v2p0"),
        DirectStandardProfile::ClrCredential => ("clr", "v2p0"),
    };

    Ok(DirectStandardCanonicalization {
        external_id: CanonicalSourceExternalId::from_components(
            standard,
            version,
            &source_domain,
            &original_standard_id,
        )?,
        original_standard_id,
        title,
        profile,
    })
}

fn trusted_domain_for_profile(
    object: &Map<String, Value>,
    profile: DirectStandardProfile,
) -> AppResult<String> {
    let candidates = match profile {
        DirectStandardProfile::OpenBadgesAchievementCredential => &["issuer.id"][..],
        DirectStandardProfile::ClrCredential => &["publisher.id", "issuer.id"][..],
    };

    for path in candidates {
        if let Some(value) = nested_string(object, path)? {
            return normalize_source_domain(&value).map_err(|_| {
                AppError::validation("trusted direct-standard source_domain could not be derived")
                    .with_error_code("DIRECT_STANDARD_REJECTED_UNTRUSTED_DOMAIN")
            });
        }
    }

    Err(
        AppError::validation("trusted direct-standard source_domain could not be derived")
            .with_error_code("DIRECT_STANDARD_REJECTED_UNTRUSTED_DOMAIN"),
    )
}

fn nested_string(object: &Map<String, Value>, path: &str) -> AppResult<Option<String>> {
    let mut current = Value::Object(object.clone());
    for segment in path.split('.') {
        let Some(next) = current.get(segment) else {
            return Ok(None);
        };
        current = next.clone();
    }

    match current {
        Value::String(value) => Ok(Some(value)),
        Value::Null => Ok(None),
        _ => Err(AppError::validation(format!("{path} must be a string"))),
    }
}

fn string_or_string_array(object: &Map<String, Value>, field: &str) -> AppResult<Vec<String>> {
    let value = object
        .get(field)
        .ok_or_else(|| AppError::validation(format!("{field} is required")))?;
    match value {
        Value::String(single) => Ok(vec![single.clone()]),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::String(item) => Ok(item.clone()),
                _ => Err(AppError::validation(format!(
                    "{field} entries must be strings"
                ))),
            })
            .collect(),
        _ => Err(AppError::validation(format!(
            "{field} must be a string or array of strings"
        ))),
    }
}

fn required_trimmed_string(object: &Map<String, Value>, field: &str) -> AppResult<String> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::validation(format!("{field} must be a string")))?;
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(AppError::validation(format!("{field} must not be empty"))
            .with_error_code("INVALID_STANDARD_PAYLOAD"));
    }
    Ok(trimmed)
}

fn has_type(types: &[String], markers: &[&str]) -> bool {
    types
        .iter()
        .any(|value| markers.iter().any(|marker| value == marker))
}

fn ensure_supported_standard_version(standard: &str, version: &str) -> AppResult<()> {
    let supported = matches!(
        (standard, version),
        ("cc", "v1p3") | ("case", "v1p0") | ("qti", "v3p0") | ("ob", "v2p0") | ("clr", "v2p0")
    );
    if supported {
        Ok(())
    } else {
        Err(AppError::validation(
            "external_id must use a supported canonical standard/version token",
        ))
    }
}

fn percent_decode_utf8(input: &str) -> AppResult<String> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(AppError::validation(
                    "external_id contains an invalid percent escape",
                ));
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).map_err(|_| {
                AppError::validation("external_id contains an invalid percent escape")
            })?;
            let value = u8::from_str_radix(hex, 16).map_err(|_| {
                AppError::validation("external_id contains an invalid percent escape")
            })?;
            output.push(value);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(output)
        .map_err(|_| AppError::validation("external_id object_id is not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{DirectStandardProfile, canonicalize_direct_standard_payload};

    #[test]
    fn classifies_open_badges_payloads() {
        let payload = json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": " urn:badge:1 ",
            "issuer": {
                "id": "https://issuer.example.org"
            },
            "name": " Rust Badge "
        });

        let standard = canonicalize_direct_standard_payload(&payload)
            .expect("open badges payload should canonicalize");

        assert_eq!(
            standard.profile,
            DirectStandardProfile::OpenBadgesAchievementCredential
        );
        assert_eq!(
            standard.external_id.canonical_uri(),
            "https://api.cherry-pick.net/ob/v2p0/issuer.example.org:urn%3Abadge%3A1"
        );
        assert_eq!(standard.original_standard_id, "urn:badge:1");
        assert_eq!(standard.title, "Rust Badge");
    }

    #[test]
    fn rejects_shape_valid_but_unmappable_standard_payloads() {
        let payload = json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential"],
            "id": "urn:example:1",
            "name": "Example"
        });

        let error = canonicalize_direct_standard_payload(&payload)
            .expect_err("unsupported standard must fail");

        assert_eq!(error.error_code(), "INVALID_STANDARD_PAYLOAD");
    }
}
