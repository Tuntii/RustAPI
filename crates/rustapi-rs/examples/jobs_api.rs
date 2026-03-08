#[cfg(not(any(feature = "extras-jobs", feature = "jobs")))]
fn main() {
    eprintln!(
        "Run this example with jobs support enabled:\n  cargo run -p rustapi-rs --example jobs_api --features extras-jobs"
    );
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
use async_trait::async_trait;
#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
use rustapi_rs::extras::jobs::{InMemoryBackend, Job, JobContext, JobQueue};
#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
use rustapi_rs::prelude::*;
#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
use std::sync::{atomic::{AtomicU64, Ordering}, Arc};

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[derive(Clone)]
struct AppState {
    processed_jobs: Arc<AtomicU64>,
    queue: JobQueue,
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[derive(Debug, Clone, Deserialize, Serialize, Schema)]
struct EmailJobData {
    to: String,
    subject: String,
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[derive(Clone)]
struct SendEmailJob {
    processed_jobs: Arc<AtomicU64>,
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[async_trait]
impl Job for SendEmailJob {
    const NAME: &'static str = "send_email";
    type Data = EmailJobData;

    async fn execute(
        &self,
        ctx: JobContext,
        data: Self::Data,
    ) -> std::result::Result<(), rustapi_rs::extras::jobs::JobError> {
        println!(
            "[job:{} attempt:{}] sending '{}' to {}",
            ctx.job_id, ctx.attempt, data.subject, data.to
        );
        self.processed_jobs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[derive(Debug, Serialize, Schema)]
struct EnqueueResponse {
    job_id: String,
    queued: bool,
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[derive(Debug, Serialize, Schema)]
struct WorkerResponse {
    processed: bool,
    total_processed: u64,
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
async fn enqueue_email(
    State(state): State<AppState>,
    Json(payload): Json<EmailJobData>,
) -> Created<EnqueueResponse> {
    let job_id = state
        .queue
        .enqueue::<SendEmailJob>(payload)
        .await
        .expect("enqueue should succeed for the in-memory backend");

    Created(EnqueueResponse {
        job_id,
        queued: true,
    })
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
async fn process_next(State(state): State<AppState>) -> Json<WorkerResponse> {
    let processed = state
        .queue
        .process_one()
        .await
        .expect("processing should succeed for the in-memory backend");

    Json(WorkerResponse {
        processed,
        total_processed: state.processed_jobs.load(Ordering::SeqCst),
    })
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
async fn queue_stats(State(state): State<AppState>) -> Json<WorkerResponse> {
    Json(WorkerResponse {
        processed: false,
        total_processed: state.processed_jobs.load(Ordering::SeqCst),
    })
}

#[cfg(any(feature = "extras-jobs", feature = "jobs"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting jobs example...");
    println!(" -> POST http://127.0.0.1:3000/jobs/email");
    println!(" -> POST http://127.0.0.1:3000/jobs/process-next");
    println!(" -> GET  http://127.0.0.1:3000/jobs/stats");

    let processed_jobs = Arc::new(AtomicU64::new(0));
    let queue = JobQueue::new(InMemoryBackend::new());
    queue
        .register_job(SendEmailJob {
            processed_jobs: processed_jobs.clone(),
        })
        .await;

    RustApi::new()
        .state(AppState {
            processed_jobs,
            queue,
        })
        .route("/jobs/email", post(enqueue_email))
        .route("/jobs/process-next", post(process_next))
        .route("/jobs/stats", get(queue_stats))
        .run("127.0.0.1:3000")
        .await
}
