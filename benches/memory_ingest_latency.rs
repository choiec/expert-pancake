use std::sync::Arc;
use std::time::{Duration, Instant};

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, header},
};
use core_infra::surrealdb::InMemorySurrealDb;
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::main]
async fn main() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    let mut registration_latencies = Vec::new();
    let mut search_latencies = Vec::new();

    for index in 0..10_u32 {
        let started = Instant::now();
        let response = post_json(
            &app,
            "/sources/register",
            json!({
                "title": format!("Bench source {index}"),
                "external-id": format!("bench-source-{index}"),
                "document-type": "markdown",
                "content": format!("# Intro\n\nbenchmark content {index}")
            }),
        )
        .await;
        registration_latencies.push(started.elapsed());
        assert!(response["source_id"].is_string());
        state
            .memory_ingest()
            .expect("memory ingest services configured")
            .index_memory_items_service()
            .process_next_job()
            .await
            .expect("worker run should succeed")
            .expect("job should exist");
    }

    for _ in 0..10 {
        let started = Instant::now();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/search/memory-items?q=benchmark&limit=10&offset=0")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("request must succeed");
        assert!(response.status().is_success());
        search_latencies.push(started.elapsed());
    }

    println!(
        "registration_ms p95={} p99={}",
        percentile_ms(&registration_latencies, 95.0),
        percentile_ms(&registration_latencies, 99.0)
    );
    println!(
        "search_ms p95={} p99={}",
        percentile_ms(&search_latencies, 95.0),
        percentile_ms(&search_latencies, 99.0)
    );
    println!("{}", state.request_metrics().render_prometheus());
}

fn percentile_ms(samples: &[Duration], percentile: f64) -> u128 {
    let mut ordered = samples.iter().map(Duration::as_millis).collect::<Vec<_>>();
    ordered.sort_unstable();
    let index = ((ordered.len() as f64 - 1.0) * (percentile / 100.0)).ceil() as usize;
    ordered[index]
}

async fn post_json(app: &axum::Router, uri: &str, payload: Value) -> Value {
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
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    serde_json::from_slice(&body).expect("response must be valid json")
}
