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

#[tokio::test]
async fn memory_ingest_latency_stays_within_slo_thresholds() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());
    let profile = load_profile();

    let canonical_template: Value =
        serde_json::from_str(include_str!("../fixtures/perf/canonical_markdown_small.json"))
            .expect("canonical fixture must parse");
    let badge_template: Value =
        serde_json::from_str(include_str!("../fixtures/perf/open_badges_small.json"))
            .expect("badge fixture must parse");
    let clr_template: Value = serde_json::from_str(include_str!("../fixtures/perf/clr_small.json"))
        .expect("clr fixture must parse");

    let canonical_registration = run_registration_scenario(
        &app,
        &state,
        profile["registration_iterations"].as_u64().unwrap_or(20) as u32,
        |index| inflate_canonical_markdown(&canonical_template, index, &profile),
    )
    .await;
    let badge_registration = run_registration_scenario(
        &app,
        &state,
        profile["registration_iterations"].as_u64().unwrap_or(20) as u32,
        |index| unique_standard_payload(&badge_template, index, "urn:badge:perf", "Performance Badge"),
    )
    .await;
    let clr_registration = run_registration_scenario(
        &app,
        &state,
        profile["registration_iterations"].as_u64().unwrap_or(20) as u32,
        |index| unique_standard_payload(
            &clr_template,
            index,
            "https://clr.example/credentials/perf",
            "Performance CLR",
        ),
    )
    .await;

    let retrieval_seed = post_json(
        &app,
        "/sources/register",
        inflate_canonical_markdown(&canonical_template, 10_000, &profile),
    )
    .await;
    assert_eq!(retrieval_seed.0, StatusCode::CREATED);
    let retrieval_source_id = retrieval_seed.1["source_id"].as_str().expect("source id");
    let retrieval_urn = retrieval_seed.1["memory_items"][0]["urn"].as_str().expect("urn");
    drain_worker(&state).await;

    let large_source_retrieval = run_get_scenario(
        &app,
        &format!("/sources/{retrieval_source_id}"),
        profile["retrieval_iterations"].as_u64().unwrap_or(20) as u32,
    )
    .await;
    let memory_item_retrieval = run_get_scenario(
        &app,
        &format!("/memory-items/{retrieval_urn}"),
        profile["retrieval_iterations"].as_u64().unwrap_or(20) as u32,
    )
    .await;

    seed_search_corpus(&app, &state, &canonical_template, &profile).await;
    let search = run_get_scenario(
        &app,
        "/search/memory-items?q=synthetic-corpus&document-type=markdown&limit=20&offset=0",
        profile["search_iterations"].as_u64().unwrap_or(20) as u32,
    )
    .await;

    assert_report(
        &canonical_registration,
        profile["registration_threshold_ms"].as_u64().unwrap_or(5_000) as u128,
        profile["registration_threshold_ms"].as_u64().unwrap_or(5_000) as u128,
    );
    assert_report(
        &badge_registration,
        profile["registration_threshold_ms"].as_u64().unwrap_or(5_000) as u128,
        profile["registration_threshold_ms"].as_u64().unwrap_or(5_000) as u128,
    );
    assert_report(
        &clr_registration,
        profile["registration_threshold_ms"].as_u64().unwrap_or(5_000) as u128,
        profile["registration_threshold_ms"].as_u64().unwrap_or(5_000) as u128,
    );
    assert_report(
        &memory_item_retrieval,
        profile["retrieval_threshold_ms"].as_u64().unwrap_or(200) as u128,
        profile["retrieval_threshold_ms"].as_u64().unwrap_or(200) as u128,
    );
    assert_report(
        &large_source_retrieval,
        profile["source_retrieval_threshold_ms"].as_u64().unwrap_or(200) as u128,
        profile["source_retrieval_threshold_ms"].as_u64().unwrap_or(200) as u128,
    );
    assert_report(
        &search,
        profile["search_threshold_ms"].as_u64().unwrap_or(500) as u128,
        profile["search_threshold_ms"].as_u64().unwrap_or(500) as u128,
    );

    let metrics = state.request_metrics().render_prometheus();

    assert_eq!(
        metric_count(
            &metrics,
            "/sources/register",
            201,
            Some("markdown"),
            Some("canonical")
        ),
        canonical_registration.total_requests
            + profile["search_corpus_sources"].as_u64().unwrap_or(250)
            + 1
    );
    assert_eq!(
        metric_count(&metrics, "/memory-items/{urn}", 200, Some("markdown"), None),
        memory_item_retrieval.total_requests
    );
    assert_eq!(
        metric_count(&metrics, "/sources/{source-id}", 200, Some("markdown"), None),
        large_source_retrieval.total_requests
    );
    assert_eq!(
        metric_count(&metrics, "/search/memory-items", 200, Some("markdown"), None),
        search.total_requests
    );
}

#[derive(Debug)]
struct ScenarioReport {
    name: &'static str,
    total_requests: u64,
    failures: u64,
    elapsed: Duration,
    latencies: Vec<Duration>,
}

impl ScenarioReport {
    fn p95_ms(&self) -> u128 {
        percentile_ms(&self.latencies, 95.0)
    }

    fn p99_ms(&self) -> u128 {
        percentile_ms(&self.latencies, 99.0)
    }

    fn throughput_rps(&self) -> f64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds == 0.0 {
            self.total_requests as f64
        } else {
            self.total_requests as f64 / seconds
        }
    }

    fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.failures as f64 / self.total_requests as f64
        }
    }
}

fn assert_report(report: &ScenarioReport, p95_max_ms: u128, p99_max_ms: u128) {
    let p95 = report.p95_ms();
    let p99 = report.p99_ms();
    assert!(p95 <= p95_max_ms, "p95 {p95}ms exceeded {p95_max_ms}ms");
    assert!(p99 <= p99_max_ms, "p99 {p99}ms exceeded {p99_max_ms}ms");
    assert_eq!(
        report.failures, 0,
        "{} error rate exceeded 0: {}",
        report.name,
        report.error_rate()
    );
    assert!(
        report.throughput_rps().is_finite() && report.throughput_rps() > 0.0,
        "{} throughput must be positive",
        report.name
    );
}

fn percentile_ms(samples: &[Duration], percentile: f64) -> u128 {
    let mut ordered = samples.iter().map(Duration::as_millis).collect::<Vec<_>>();
    ordered.sort_unstable();
    let index = ((ordered.len() as f64 - 1.0) * (percentile / 100.0)).ceil() as usize;
    ordered[index]
}

fn load_profile() -> Value {
    serde_json::from_str(include_str!("../fixtures/perf/workload_profile.json"))
        .expect("workload profile must parse")
}

async fn run_registration_scenario<F>(
    app: &axum::Router,
    state: &AppState,
    iterations: u32,
    mut build_payload: F,
) -> ScenarioReport
where
    F: FnMut(u32) -> Value,
{
    let started = Instant::now();
    let mut latencies = Vec::new();
    let mut failures = 0_u64;

    for index in 0..iterations {
        let payload = build_payload(index);
        let request_started = Instant::now();
        let response = post_json(app, "/sources/register", payload).await;
        latencies.push(request_started.elapsed());
        if !matches!(response.0, StatusCode::CREATED | StatusCode::OK) {
            failures += 1;
        }
        drain_worker(state).await;
    }

    ScenarioReport {
        name: "registration",
        total_requests: iterations as u64,
        failures,
        elapsed: started.elapsed(),
        latencies,
    }
}

async fn run_get_scenario(app: &axum::Router, uri: &str, iterations: u32) -> ScenarioReport {
    let started = Instant::now();
    let mut latencies = Vec::new();
    let mut failures = 0_u64;

    for _ in 0..iterations {
        let request_started = Instant::now();
        let response = get(app, uri).await;
        latencies.push(request_started.elapsed());
        if response.status() != StatusCode::OK {
            failures += 1;
        }
    }

    ScenarioReport {
        name: uri_to_name(uri),
        total_requests: iterations as u64,
        failures,
        elapsed: started.elapsed(),
        latencies,
    }
}

async fn seed_search_corpus(
    app: &axum::Router,
    state: &AppState,
    template: &Value,
    profile: &Value,
) {
    let sources = profile["search_corpus_sources"].as_u64().unwrap_or(250) as u32;

    for index in 0..sources {
        let response = post_json(
            app,
            "/sources/register",
            inflate_search_corpus_payload(template, index),
        )
        .await;
        assert!(matches!(response.0, StatusCode::CREATED | StatusCode::OK));
        drain_worker(state).await;
    }
}

async fn drain_worker(state: &AppState) {
    state
        .memory_ingest()
        .expect("memory ingest services configured")
        .index_memory_items_service()
        .process_next_job()
        .await
        .expect("worker run should succeed");
}

fn inflate_canonical_markdown(template: &Value, index: u32, profile: &Value) -> Value {
    let mut payload = template.clone();
    let target_bytes = profile["canonical_markdown_target_bytes"]
        .as_u64()
        .unwrap_or(98_304) as usize;
    payload["external-id"] =
        json!(format!("https://api.cherry-pick.net/cc/v1p3/example.edu:perf-markdown-{index}"));
    payload["title"] = json!(format!("Performance Markdown {index}"));

    let seed = payload["content"]
        .as_str()
        .expect("canonical fixture content should be string");
    let mut content = String::new();
    let mut section = 0_u32;
    while content.len() + seed.len() < target_bytes {
        content.push_str(&format!("# Section {section:05}\n\n{seed}\n\n"));
        section += 1;
    }
    payload["content"] = json!(content);
    payload
}

fn inflate_search_corpus_payload(template: &Value, index: u32) -> Value {
    let mut payload = template.clone();
    payload["external-id"] = json!(format!(
        "https://api.cherry-pick.net/cc/v1p3/example.edu:synthetic-corpus-{index}"
    ));
    payload["title"] = json!(format!("Synthetic Corpus {index}"));
    payload["content"] = json!(format!(
        "# Synthetic Corpus {index}\n\nsynthetic-corpus paragraph {index}\n\n# Retrieval\n\nsynthetic-corpus follow-up {index}"
    ));
    payload
}

fn unique_standard_payload(
    template: &Value,
    index: u32,
    id_prefix: &str,
    name_prefix: &str,
) -> Value {
    let mut payload = template.clone();
    payload["id"] = json!(format!("{id_prefix}-{index}"));
    payload["name"] = json!(format!("{name_prefix} {index}"));
    payload
}

fn metric_count(
    metrics: &str,
    route: &str,
    status_code: u16,
    document_type: Option<&str>,
    ingest_kind: Option<&str>,
) -> u64 {
    let document_type = document_type.unwrap_or("unknown");
    let ingest_kind = ingest_kind.unwrap_or("unknown");

    metrics
        .lines()
        .find(|line| {
            line.starts_with("http_request_latency_ms_count{")
                && line.contains(&format!("route=\"{route}\""))
                && line.contains(&format!("status_code=\"{status_code}\""))
                && line.contains(&format!("document_type=\"{document_type}\""))
                && line.contains(&format!("ingest_kind=\"{ingest_kind}\""))
        })
        .and_then(|line| line.rsplit_once(' '))
        .and_then(|(_, value)| value.parse::<u64>().ok())
        .unwrap_or_default()
}

fn uri_to_name(uri: &str) -> &'static str {
    if uri.starts_with("/memory-items/") {
        "memory-item-retrieval"
    } else if uri.starts_with("/sources/") {
        "source-retrieval"
    } else {
        "search"
    }
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
