use mod_memory::domain::credential::{CredentialFamily, StandardCredential};
use mod_memory::infra::repo::CredentialRepository;
use serde_json::json;

use core_infra::surrealdb::SurrealCredentialStore;

fn sample_credential(id: &str, name: &str) -> StandardCredential {
    StandardCredential::new(
        id.to_string(),
        CredentialFamily::OpenBadgesV3,
        json!({
            "@context": [
                "https://www.w3.org/ns/credentials/v2",
                "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"
            ],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": id,
            "name": name,
            "issuer": {"id": "https://issuer.example.org"},
            "credentialSubject": {
                "id": "did:example:learner:001",
                "type": "AchievementSubject",
                "achievement": {
                    "id": "https://issuer.example.org/achievements/rust-badge",
                    "type": "Achievement",
                    "name": name
                }
            },
            "proof": {"type": "DataIntegrityProof"},
            "validFrom": "2026-01-01T00:00:00Z"
        }),
    )
}

#[tokio::test]
async fn authoritative_store_preserves_schema_exact_records_and_uniqueness() {
    let store = SurrealCredentialStore::new();
    let credential = sample_credential("urn:example:badge:store", "Store Badge");

    let created = store.register(credential.clone()).await.expect("created");
    assert_eq!(created.credential.credential, credential.credential);

    let replay = store.register(credential.clone()).await.expect("replayed");
    assert_eq!(replay.credential.credential, credential.credential);

    let conflicting = sample_credential("urn:example:badge:store", "Changed Badge");
    let error = store.register(conflicting).await.expect_err("conflict");
    assert_eq!(error.status(), http::StatusCode::CONFLICT);

    let loaded = store
        .get("urn:example:badge:store")
        .await
        .expect("load succeeds")
        .expect("record exists");
    assert_eq!(loaded.credential, credential.credential);
    assert_eq!(store.job_count().await, 1);
}
