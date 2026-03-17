use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use core_infra::InfrastructureServices;
use core_shared::StartupError;
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
    infrastructure: Option<InfrastructureServices>,
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
pub struct MetricsLabels {
    pub document_type: Option<String>,
    pub ingest_kind: Option<String>,
}

impl AppState {
    pub async fn bootstrap(config: AppConfig) -> Result<Self, StartupError> {
        let infrastructure =
            core_infra::setup::bootstrap_infrastructure(&config.infrastructure).await?;

        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                infrastructure: Some(infrastructure),
                metrics: RequestMetrics::new(),
                probe_mode: ProbeMode::Live,
            }),
        })
    }

    pub fn for_test(config: AppConfig, probe_snapshot: ProbeSnapshot) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config,
                infrastructure: None,
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

    pub async fn readiness(&self) -> ProbeSnapshot {
        match &self.inner.probe_mode {
            ProbeMode::Fixed(snapshot) => *snapshot,
            ProbeMode::Live => {
                let infrastructure = self
                    .inner
                    .infrastructure
                    .as_ref()
                    .expect("live mode requires infrastructure services");

                let database = infrastructure.surrealdb.readiness().await;
                let search = infrastructure.meilisearch.readiness().await;

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
                "method=\"{}\",route=\"{}\",status_code=\"{}\",document_type=\"{}\",ingest_kind=\"{}\"",
                key.method,
                key.route,
                key.status_code,
                key.document_type.as_deref().unwrap_or("unknown"),
                key.ingest_kind.as_deref().unwrap_or("unknown"),
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
