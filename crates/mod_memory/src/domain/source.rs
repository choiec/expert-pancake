use serde::{Deserialize, Serialize};
use uuid::{Uuid, uuid};

pub const CANONICAL_EXTERNAL_ID_PREFIX: &str = "https://api.cherry-pick.net/";
pub const CANONICAL_ID_VERSION: &str = "v1";

const SOURCE_ID_NAMESPACE: Uuid = uuid!("718d566c-b4f7-5705-83c8-f4fd6a0e2e1d");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    Text,
    Markdown,
    Json,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFamily {
    OpenBadges,
    Clr,
}

impl StandardFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenBadges => "openbadges",
            Self::Clr => "clr",
        }
    }

    pub fn default_version(&self) -> &'static str {
        match self {
            Self::OpenBadges => "v3p0",
            Self::Clr => "v2p0",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSource {
    pub title: String,
    pub summary: Option<String>,
    pub external_id: String,
    pub document_type: DocumentType,
    pub content: String,
    pub metadata: serde_json::Value,
    pub canonical_hash: String,
}

pub fn derive_source_id(external_id: &str) -> Uuid {
    Uuid::new_v5(&SOURCE_ID_NAMESPACE, external_id.as_bytes())
}

pub fn is_canonical_external_id(external_id: &str) -> bool {
    let remainder = match external_id.strip_prefix(CANONICAL_EXTERNAL_ID_PREFIX) {
        Some(value) => value,
        None => return false,
    };

    let mut segments = remainder.splitn(3, '/');
    let Some(standard) = segments.next() else {
        return false;
    };
    let Some(version) = segments.next() else {
        return false;
    };
    let Some(tail) = segments.next() else {
        return false;
    };

    is_lower_alnum(standard)
        && is_canonical_version(version)
        && tail.split_once(':').is_some_and(|(domain, object_id)| {
            !domain.trim().is_empty() && !object_id.trim().is_empty()
        })
}

pub fn canonical_external_id_for_standard(
    family: StandardFamily,
    source_domain: &str,
    original_standard_id: &str,
) -> String {
    format!(
        "{}/{}/{source_domain}:{}",
        format!("{CANONICAL_EXTERNAL_ID_PREFIX}{}", family.as_str()),
        family.default_version(),
        percent_encode(original_standard_id.trim()),
    )
}

fn is_lower_alnum(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

fn is_canonical_version(value: &str) -> bool {
    let Some(version) = value.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<_> = version.split('p').collect();
    parts.len() >= 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(nibble_to_hex(byte >> 4));
            encoded.push(nibble_to_hex(byte & 0x0f));
        }
    }
    encoded
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'A' + (value - 10)),
        _ => '0',
    }
}
