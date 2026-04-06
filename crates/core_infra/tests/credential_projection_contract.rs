use serde_json::json;

use core_infra::build_infra_bundle;

#[tokio::test]
async fn projection_sync_is_non_authoritative_and_recoverable() {
    let bundle = build_infra_bundle();
    bundle.handles.search_store.set_ready(false).await;

    let outcome = bundle
        .module
        .register_service
        .register(json!({
            "@context": [
                "https://www.w3.org/ns/credentials/v2",
                "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"
            ],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": "urn:example:badge:projection",
            "name": "Projection Badge",
            "issuer": {"id": "https://issuer.example.org"},
            "credentialSubject": {
                "id": "did:example:learner:001",
                "type": "AchievementSubject",
                "achievement": {
                    "id": "https://issuer.example.org/achievements/projection",
                    "type": "Achievement",
                    "name": "Projection Badge"
                }
            },
            "proof": {"type": "DataIntegrityProof"},
            "validFrom": "2026-01-01T00:00:00Z"
        }))
        .await
        .expect("authoritative registration succeeds");

    assert_eq!(
        outcome.credential.credential_id,
        "urn:example:badge:projection"
    );
    assert_eq!(bundle.handles.search_store.projection_count().await, 0);
    assert_eq!(bundle.handles.authoritative_store.job_count().await, 1);

    bundle.handles.search_store.set_ready(true).await;
    bundle
        .projection_sync
        .sync_pending()
        .await
        .expect("pending projection sync succeeds");

    assert_eq!(bundle.handles.search_store.projection_count().await, 1);
}
