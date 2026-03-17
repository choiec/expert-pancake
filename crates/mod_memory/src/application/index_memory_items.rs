use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use core_shared::{AppError, AppResult, ErrorKind};

use crate::application::register_source::ClockPort;
use crate::infra::indexer::{IndexingJob, OutboxStatus, ProjectionIndexPort};
use crate::infra::meili_indexer::backoff_available_at;
use crate::infra::repo::IndexingOutboxRepository;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub poll_interval: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            poll_interval: Duration::from_millis(250),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexingIterationResult {
    pub source_id: uuid::Uuid,
    pub job_id: uuid::Uuid,
    pub status: OutboxStatus,
    pub retry_count: u32,
}

pub struct IndexMemoryItemsService {
    outbox_repo: Arc<dyn IndexingOutboxRepository>,
    projection_index: Arc<dyn ProjectionIndexPort>,
    clock: Arc<dyn ClockPort>,
    retry_policy: RetryPolicy,
}

impl IndexMemoryItemsService {
    pub fn new(
        outbox_repo: Arc<dyn IndexingOutboxRepository>,
        projection_index: Arc<dyn ProjectionIndexPort>,
        clock: Arc<dyn ClockPort>,
        retry_policy: RetryPolicy,
    ) -> Self {
        Self {
            outbox_repo,
            projection_index,
            clock,
            retry_policy,
        }
    }

    pub async fn process_next_job(&self) -> AppResult<Option<IndexingIterationResult>> {
        let now = self.clock.now();
        let Some(mut job) = self.outbox_repo.claim_next_job(now).await? else {
            return Ok(None);
        };

        let result = async {
            self.projection_index.ensure_index().await?;
            let projection_inputs = self
                .outbox_repo
                .rehydrate_projection_inputs(job.source_id)
                .await?;
            if projection_inputs.is_empty() {
                return Err(AppError::internal(format!(
                    "source '{}' produced no projection inputs",
                    job.source_id
                )));
            }

            self.projection_index.upsert(&projection_inputs).await
        }
        .await;

        let now = self.clock.now();
        match result {
            Ok(()) => {
                job.status = OutboxStatus::Completed;
                job.last_error = None;
                job.updated_at = now;
                job.available_at = now;
                self.outbox_repo.update_job(&job).await?;
                Ok(Some(IndexingIterationResult {
                    source_id: job.source_id,
                    job_id: job.job_id,
                    status: job.status,
                    retry_count: job.retry_count,
                }))
            }
            Err(error) => {
                self.transition_failure(&mut job, now, &error).await?;
                Ok(Some(IndexingIterationResult {
                    source_id: job.source_id,
                    job_id: job.job_id,
                    status: job.status,
                    retry_count: job.retry_count,
                }))
            }
        }
    }

    pub async fn run_forever(self: Arc<Self>) {
        loop {
            match self.process_next_job().await {
                Ok(Some(result)) => {
                    info!(
                        source_id = %result.source_id,
                        job_id = %result.job_id,
                        status = result.status.as_str(),
                        retry_count = result.retry_count,
                        "processed memory indexing job"
                    );
                }
                Ok(None) => {
                    tokio::time::sleep(self.retry_policy.poll_interval).await;
                }
                Err(error) => {
                    warn!(message = %error.message(), "memory indexing worker iteration failed");
                    tokio::time::sleep(self.retry_policy.poll_interval).await;
                }
            }
        }
    }

    async fn transition_failure(
        &self,
        job: &mut IndexingJob,
        now: time::OffsetDateTime,
        error: &AppError,
    ) -> AppResult<()> {
        let next_retry_count = job.retry_count + 1;
        job.retry_count = next_retry_count;
        job.last_error = Some(error.message().to_owned());
        job.updated_at = now;
        if next_retry_count > self.retry_policy.max_retries
            || error.kind() == ErrorKind::NotFound
            || error.kind() == ErrorKind::Internal
        {
            job.status = OutboxStatus::DeadLetter;
            job.available_at = now;
        } else {
            job.status = OutboxStatus::Retryable;
            job.available_at =
                backoff_available_at(now, next_retry_count, self.retry_policy.retry_delay);
        }
        self.outbox_repo.update_job(job).await
    }
}
