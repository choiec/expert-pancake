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
    let profile: Value =
        serde_json::from_str(include_str!("../../repo_tests/fixtures/perf/workload_profile.json"))
            .expect("workload profile must parse");
    let canonical_template: Value = serde_json::from_str(include_str!(
        "../../repo_tests/fixtures/perf/canonical_markdown_small.json"
    ))
    .expect("canonical fixture must parse");
    let badge_template: Value = serde_json::from_str(include_str!(
        "../../repo_tests/fixtures/perf/open_badges_small.json"
    ))
    .expect("badge fixture must parse");
    let clr_template: Value =
        serde_json::from_str(include_str!("../../repo_tests/fixtures/perf/clr_small.json"))
            .expect("clr fixture must parse");

    let canonical = run_registration_report(
        &app,
        &state,
        profile["benchmark_iterations"].as_u64().unwrap_or(10) as u32,
        |index| inflate_canonical_markdown(&canonical_template, index, &profile),
    )
    .await;
    let badge = run_registration_report(
        &app,
        &state,
        profile["benchmark_iterations"].as_u64().unwrap_or(10) as u32,
        |index| unique_standard_payload(&badge_template, index, "urn:badge:bench", "Bench Badge"),
    )
    .await;
    let clr = run_registration_report(
        &app,
        &state,
        profile["benchmark_iterations"].as_u64().unwrap_or(10) as u32,
        |index| {
            unique_standard_payload(
                &clr_template,
                index,
                "https://clr.example/credentials/bench",
                "Bench CLR",
            )
        },
    )
    .await;

    let retrieval_seed = post_json(
        &app,
        "/sources/register",
        inflate_canonical_markdown(&canonical_template, 50_000, &profile),
    )
    .await;
    let source_id = retrieval_seed["source_id"]
        .as_str()
        .expect("source id")
        .to_owned();
    let urn = retrieval_seed["memory_items"][0]["urn"]
        .as_str()
        .expect("urn")
        .to_owned();
    drain_worker(&state).await;

    seed_search_corpus(&app, &state, &canonical_template, &profile).await;

    let source_retrieval = run_get_report(
        &app,
        &format!("/sources/{source_id}"),
        profile["benchmark_iterations"].as_u64().unwrap_or(10) as u32,
    )
    .await;
    let memory_retrieval = run_get_report(
        &app,
        &format!("/memory-items/{urn}"),
        profile["benchmark_iterations"].as_u64().unwrap_or(10) as u32,
    )
    .await;
    let search = run_get_report(
        &app,
        "/search/memory-items?q=synthetic-corpus&document-type=markdown&limit=20&offset=0",
        profile["benchmark_iterations"].as_u64().unwrap_or(10) as u32,
    )
    .await;

    print_report("canonical_registration", &canonical);
    print_report("open_badges_registration", &badge);
    print_report("clr_registration", &clr);
    print_report("memory_item_retrieval", &memory_retrieval);
    print_report("source_retrieval_10k", &source_retrieval);
    print_report("search_projection", &search);
    println!("{}", state.request_metrics().render_prometheus());
}

#[derive(Debug)]
struct Report {
    total_requests: u64,
    failures: u64,
    elapsed: Duration,
    latencies: Vec<Duration>,
}

impl Report {
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

fn percentile_ms(samples: &[Duration], percentile: f64) -> u128 {
    let mut ordered = samples.iter().map(Duration::as_millis).collect::<Vec<_>>();
    ordered.sort_unstable();
    let index = ((ordered.len() as f64 - 1.0) * (percentile / 100.0)).ceil() as usize;
    ordered[index]
}

async fn run_registration_report<F>(
    app: &axum::Router,
    state: &AppState,
    iterations: u32,
    mut build_payload: F,
) -> Report
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
        if !response["source_id"].is_string() {
            failures += 1;
        }
        drain_worker(state).await;
    }

    Report {
        total_requests: iterations as u64,
        failures,
        elapsed: started.elapsed(),
        latencies,
    }
}

async fn run_get_report(app: &axum::Router, uri: &str, iterations: u32) -> Report {
    let started = Instant::now();
    let mut latencies = Vec::new();
    let mut failures = 0_u64;

    for _ in 0..iterations {
        let request_started = Instant::now();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("request must succeed");
        latencies.push(request_started.elapsed());
        if !response.status().is_success() {
            failures += 1;
        }
    }

    Report {
        total_requests: iterations as u64,
        failures,
        elapsed: started.elapsed(),
        latencies,
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
        assert!(response["source_id"].is_string());
        drain_worker(state).await;
    }
}

fn inflate_canonical_markdown(template: &Value, index: u32, profile: &Value) -> Value {
    let mut payload = template.clone();
    let target_bytes = profile["canonical_markdown_target_bytes"]
        .as_u64()
        .unwrap_or(98_304) as usize;
    payload["external-id"] = json!(format!("bench-markdown-{index}"));
    payload["title"] = json!(format!("Bench Markdown {index}"));

    let seed = payload["content"].as_str().expect("content must be string");
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
    payload["external-id"] = json!(format!("bench-corpus-{index}"));
    payload["title"] = json!(format!("Bench Corpus {index}"));
    payload["content"] = json!(format!(
        "# Corpus {index}\n\nsynthetic-corpus benchmark line {index}\n\n# Follow Up\n\nsynthetic-corpus benchmark tail {index}"
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

fn print_report(name: &str, report: &Report) {
    println!(
        "{name} p95_ms={} p99_ms={} throughput_rps={:.2} error_rate={:.4}",
        report.p95_ms(),
        report.p99_ms(),
        report.throughput_rps(),
        report.error_rate()
    );
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
