use uuid::{Uuid, uuid};

pub const SOURCE_ID_NAMESPACE: Uuid = uuid!("5f7a4f31-178b-57be-a92f-66c5c7d0f50f");

pub fn source_seed(canonical_id_version: &str, canonical_external_id: &str) -> String {
    format!("source|{canonical_id_version}|{canonical_external_id}")
}

pub fn deterministic_source_id(canonical_id_version: &str, canonical_external_id: &str) -> Uuid {
    let seed = source_seed(canonical_id_version, canonical_external_id);
    Uuid::new_v5(&SOURCE_ID_NAMESPACE, seed.as_bytes())
}

pub fn verify_source_id(
    canonical_id_version: &str,
    canonical_external_id: &str,
    source_id: Uuid,
) -> bool {
    deterministic_source_id(canonical_id_version, canonical_external_id) == source_id
}