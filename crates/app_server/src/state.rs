use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use core_infra::surrealdb::InMemorySurrealDb;
use core_infra::{MeilisearchService, SurrealDbService};
use core_shared::StartupError;
use mod_memory::MemoryModule;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

const LATENCY_BUCKETS_MS: [u64; 6] = [50, 100, 200, 500, 1_000, 5_000];

#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(Debug)]
struct AppStateInner {
    config: AppConfig,
    surrealdb: Option<Arc<SurrealDbService>>,
    meilisearch: Option<Arc<MeilisearchService>>,
    memory_ingest: Option<MemoryModule>,
    metrics: RequestMetrics,
    probe_mode: ProbeMode,
}

#[derive(Debug, Clone)]
enum ProbeMode {
    Live,
    Fixed(ProbeSnapshot),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeStatus {
    Ready,
    Degraded,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeSnapshot {
    pub service: ProbeStatus,
    pub database: ProbeStatus,
    pub search: ProbeStatus,
}

#[derive(Debug)]
pub struct RequestMetrics {
    buckets_ms: &'static [u64],
    inner: Mutex<BTreeMap<MetricKey, HistogramSeries>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MetricKey {
    pub method: String,
    pub route: String,
    pub status_code: u16,
    pub document_type: Option<String>,
    pub ingest_kind: Option<String>,
    pub decision_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct HistogramSeries {
    buckets: Vec<BucketCount>,
    count: u64,
    sum_ms: u128,
}

#[derive(Debug, Clone)]
struct BucketCount {
    upper_bound_ms: u64,
    count: u64,
}

#[derive(Debug, Clone, Default)]
/// Optional per-response metric labels consumed by the latency middleware.
///
/// Future handlers can attach bounded labels before returning a response:
/// `MetricsLabels::new().with_document_type("markdown").with_ingest_kind("canonical")`
/// and then insert them into `response.extensions_mut()` or call
/// `MetricsLabels::insert_response_extension`.
pub struct MetricsLabels {
    pub document_type: Option<String>,
    pub ingest_kind: Option<String>,
    pub decision_reason: Option<String>,
}

impl AppState {
    pub async fn bootstrap(config: AppConfig) -> Result<Self, StartupError> {
        let infrastructure =
            core_infra::setup::bootstrap_infrastructure(&config.infrastructure).await?;
        let surrealdb = Arc::new(infrastructure.surrealdb);
        let meilisearch = Arc::new(infrastructure.meilisearch);
        let search_enabled = config.infrastructure.meilisearch.enabled;
        let memory_ingest = MemoryModule::runtime(
            surrealdb.clone(),
            meilisearch.clone(),
            search_enabled,
            config.timeouts.normalization_timeout,
        );

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                surrealdb: Some(surrealdb),
                meilisearch: Some(meilisearch),
                memory_ingest: Some(memory_ingest),
                metrics: RequestMetrics::new(),
                probe_mode: ProbeMode::Live,
            }),
        })
    }

    pub fn for_test(config: AppConfig, probe_snapshot: ProbeSnapshot) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config,
                surrealdb: None,
                meilisearch: None,
                memory_ingest: None,
                metrics: RequestMetrics::new(),
                probe_mode: ProbeMode::Fixed(probe_snapshot),
            }),
        }
    }

    pub fn for_memory_ingest_test(
        config: AppConfig,
        probe_snapshot: ProbeSnapshot,
        db: Arc<InMemorySurrealDb>,
    ) -> Self {
        Self::for_memory_ingest_test_with_projection(config, probe_snapshot, db, true)
    }

    pub fn for_memory_ingest_test_with_projection(
        config: AppConfig,
        probe_snapshot: ProbeSnapshot,
        db: Arc<InMemorySurrealDb>,
        projection_available: bool,
    ) -> Self {
        let memory_ingest = MemoryModule::fixture(
            db,
            projection_available,
            config.timeouts.normalization_timeout,
        );

        Self {
            inner: Arc::new(AppStateInner {
                config,
                surrealdb: None,
                meilisearch: None,
                memory_ingest: Some(memory_ingest),
                metrics: RequestMetrics::new(),
                probe_mode: ProbeMode::Fixed(probe_snapshot),
            }),
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    pub fn max_request_body_bytes(&self) -> usize {
        self.inner.config.limits.max_request_body_bytes
    }

    pub fn normalization_timeout(&self) -> Duration {
        self.inner.config.timeouts.normalization_timeout
    }

    pub fn request_metrics(&self) -> &RequestMetrics {
        &self.inner.metrics
    }

    pub fn spawn_background_tasks(&self) {
        if let Some(memory_ingest) = &self.inner.memory_ingest {
            let memory_ingest = memory_ingest.clone();
            tokio::spawn(async move {
                memory_ingest.index_memory_items().await;
            });
        }
    }

    pub fn memory_ingest(&self) -> Option<&MemoryModule> {
        self.inner.memory_ingest.as_ref()
    }

    pub async fn readiness(&self) -> ProbeSnapshot {
        match &self.inner.probe_mode {
            ProbeMode::Fixed(snapshot) => *snapshot,
            ProbeMode::Live => {
                let database = self
                    .inner
                    .surrealdb
                    .as_ref()
                    .expect("live mode requires surrealdb")
                    .readiness()
                    .await;
                let search = self
                    .inner
                    .meilisearch
                    .as_ref()
                    .expect("live mode requires meilisearch")
                    .readiness()
                    .await;

                ProbeSnapshot {
                    service: ProbeStatus::Ready,
                    database: if database.is_ready {
                        ProbeStatus::Ready
                    } else {
                        ProbeStatus::Down
                    },
                    search: if search.is_ready {
                        ProbeStatus::Ready
                    } else {
                        ProbeStatus::Degraded
                    },
                }
            }
        }
    }
}

impl ProbeSnapshot {
    pub const fn ready() -> Self {
        Self {
            service: ProbeStatus::Ready,
            database: ProbeStatus::Ready,
            search: ProbeStatus::Ready,
        }
    }

    pub const fn new(service: ProbeStatus, database: ProbeStatus, search: ProbeStatus) -> Self {
        Self {
            service,
            database,
            search,
        }
    }
}

impl MetricsLabels {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_document_type(mut self, document_type: impl Into<String>) -> Self {
        self.document_type = Some(document_type.into());
        self
    }

    pub fn with_ingest_kind(mut self, ingest_kind: impl Into<String>) -> Self {
        self.ingest_kind = Some(ingest_kind.into());
        self
    }

    pub fn with_decision_reason(mut self, decision_reason: impl Into<String>) -> Self {
        self.decision_reason = Some(decision_reason.into());
        self
    }

    pub fn insert_response_extension(self, response: &mut axum::response::Response) {
        response.extensions_mut().insert(self);
    }
}

impl RequestMetrics {
    fn new() -> Self {
        Self {
            buckets_ms: &LATENCY_BUCKETS_MS,
            inner: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn buckets_ms(&self) -> &'static [u64] {
        self.buckets_ms
    }

    pub fn record(&self, key: MetricKey, duration: Duration) {
        let duration_ms = duration.as_millis();
        let mut guard = self.inner.lock().expect("request metrics mutex poisoned");
        let series = guard
            .entry(key)
            .or_insert_with(|| HistogramSeries::new(self.buckets_ms));

        series.record(duration_ms);
    }

    pub fn render_prometheus(&self) -> String {
        let guard = self.inner.lock().expect("request metrics mutex poisoned");
        let mut output = String::from(
            "# HELP http_request_latency_ms HTTP request latency histogram in milliseconds\n",
        );
        output.push_str("# TYPE http_request_latency_ms histogram\n");

        for (key, series) in guard.iter() {
            let labels = format!(
                "method=\"{}\",route=\"{}\",status_code=\"{}\",document_type=\"{}\",ingest_kind=\"{}\",decision_reason=\"{}\"",
                key.method,
                key.route,
                key.status_code,
                key.document_type.as_deref().unwrap_or("unknown"),
                key.ingest_kind.as_deref().unwrap_or("unknown"),
                key.decision_reason.as_deref().unwrap_or("unknown"),
            );

            for bucket in &series.buckets {
                output.push_str(&format!(
                    "http_request_latency_ms_bucket{{{labels},le=\"{}\"}} {}\n",
                    bucket.upper_bound_ms, bucket.count,
                ));
            }

            output.push_str(&format!(
                "http_request_latency_ms_bucket{{{labels},le=\"+Inf\"}} {}\n",
                series.count,
            ));
            output.push_str(&format!(
                "http_request_latency_ms_sum{{{labels}}} {}\n",
                series.sum_ms,
            ));
            output.push_str(&format!(
                "http_request_latency_ms_count{{{labels}}} {}\n",
                series.count,
            ));
        }

        output
    }
}

impl HistogramSeries {
    fn new(buckets_ms: &[u64]) -> Self {
        Self {
            buckets: buckets_ms
                .iter()
                .map(|upper_bound_ms| BucketCount {
                    upper_bound_ms: *upper_bound_ms,
                    count: 0,
                })
                .collect(),
            count: 0,
            sum_ms: 0,
        }
    }

    fn record(&mut self, duration_ms: u128) {
        self.count += 1;
        self.sum_ms += duration_ms;

        for bucket in &mut self.buckets {
            if duration_ms <= u128::from(bucket.upper_bound_ms) {
                bucket.count += 1;
            }
        }
    }
}
