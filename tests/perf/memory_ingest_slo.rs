use std::sync::Arc;
use std::time::{Duration, Instant};

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use core_infra::surrealdb::InMemorySurrealDb;
use serde_json::{Value, json};
use tower::ServiceExt;

const REGISTRATION_P95_MS: u128 = 5_000;
const REGISTRATION_P99_MS: u128 = 5_000;
const RETRIEVAL_P95_MS: u128 = 200;
const RETRIEVAL_P99_MS: u128 = 200;
const SEARCH_P95_MS: u128 = 500;
const SEARCH_P99_MS: u128 = 500;

#[tokio::test]
async fn memory_ingest_latency_stays_within_slo_thresholds() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    let mut registration_latencies = Vec::new();
    let mut retrieval_latencies = Vec::new();
    let mut search_latencies = Vec::new();

    let badge_fixture: Value =
        serde_json::from_str(include_str!("../fixtures/perf/open_badges_small.json"))
            .expect("badge fixture must parse");
    let clr_fixture: Value = serde_json::from_str(include_str!("../fixtures/perf/clr_small.json"))
        .expect("clr fixture must parse");

    let mut first_source_id = None;
    let mut first_urn = None;

    for index in 0..20_u32 {
        let payload = if index % 3 == 0 {
            json!({
                "title": format!("Perf markdown {index}"),
                "external-id": format!("perf-markdown-{index}"),
                "document-type": "markdown",
                "content": format!("# Intro\n\nbenchmark body {index} with searchable content")
            })
        } else if index % 3 == 1 {
            let mut payload = badge_fixture.clone();
            payload["id"] = json!(format!("urn:badge:perf-{index}"));
            payload["name"] = json!(format!("Performance Badge {index}"));
            payload
        } else {
            let mut payload = clr_fixture.clone();
            payload["id"] = json!(format!("https://clr.example/credentials/perf-{index}"));
            payload["name"] = json!(format!("Performance CLR {index}"));
            payload
        };

        let started = Instant::now();
        let response = post_json(&app, "/sources/register", payload).await;
        registration_latencies.push(started.elapsed());
        assert!(matches!(response.0, StatusCode::CREATED | StatusCode::OK));

        state
            .memory_ingest()
            .expect("memory ingest services configured")
            .index_memory_items_service()
            .process_next_job()
            .await
            .expect("worker run should succeed")
            .expect("job should exist");

        if first_source_id.is_none() {
            first_source_id = response.1["source_id"].as_str().map(str::to_owned);
            first_urn = response.1["memory_items"][0]["urn"]
                .as_str()
                .map(str::to_owned);
        }
    }

    let source_id = first_source_id.expect("first source id present");
    let urn = first_urn.expect("first urn present");

    for _ in 0..30 {
        let started = Instant::now();
        let response = get(&app, &format!("/memory-items/{urn}")).await;
        retrieval_latencies.push(started.elapsed());
        assert_eq!(response.status(), StatusCode::OK);

        let started = Instant::now();
        let response = get(&app, &format!("/sources/{source_id}")).await;
        retrieval_latencies.push(started.elapsed());
        assert_eq!(response.status(), StatusCode::OK);
    }

    for _ in 0..30 {
        let started = Instant::now();
        let response = get(
            &app,
            "/search/memory-items?q=Performance&document-type=json&limit=10&offset=0",
        )
        .await;
        search_latencies.push(started.elapsed());
        assert_eq!(response.status(), StatusCode::OK);
    }

    assert_duration_thresholds(
        &registration_latencies,
        REGISTRATION_P95_MS,
        REGISTRATION_P99_MS,
    );
    assert_duration_thresholds(&retrieval_latencies, RETRIEVAL_P95_MS, RETRIEVAL_P99_MS);
    assert_duration_thresholds(&search_latencies, SEARCH_P95_MS, SEARCH_P99_MS);

    let metrics = state.request_metrics().render_prometheus();
    assert!(metrics.contains("route=\"/sources/register\""));
    assert!(metrics.contains("route=\"/memory-items/{urn}\""));
    assert!(metrics.contains("route=\"/sources/{source-id}\""));
    assert!(metrics.contains("route=\"/search/memory-items\""));
}

fn assert_duration_thresholds(samples: &[Duration], p95_max_ms: u128, p99_max_ms: u128) {
    let p95 = percentile_ms(samples, 95.0);
    let p99 = percentile_ms(samples, 99.0);
    assert!(p95 <= p95_max_ms, "p95 {p95}ms exceeded {p95_max_ms}ms");
    assert!(p99 <= p99_max_ms, "p99 {p99}ms exceeded {p99_max_ms}ms");
}

fn percentile_ms(samples: &[Duration], percentile: f64) -> u128 {
    let mut ordered = samples.iter().map(Duration::as_millis).collect::<Vec<_>>();
    ordered.sort_unstable();
    let index = ((ordered.len() as f64 - 1.0) * (percentile / 100.0)).ceil() as usize;
    ordered[index]
}

async fn post_json(app: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    (
        status,
        serde_json::from_slice(&body).expect("response must be valid json"),
    )
}

async fn get(app: &axum::Router, uri: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed")
}
