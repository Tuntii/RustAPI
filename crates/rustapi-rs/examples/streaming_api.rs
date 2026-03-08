use rustapi_rs::prelude::*;
use std::convert::Infallible;

#[derive(Debug, Serialize, Schema)]
struct ProgressUpdate {
    step: u32,
    message: String,
}

async fn progress_feed() -> Sse<impl futures_util::Stream<Item = std::result::Result<SseEvent, Infallible>>> {
    let events = vec![
        Ok::<_, Infallible>(SseEvent::json_data(&ProgressUpdate {
            step: 1,
            message: "queued".to_string(),
        })
        .expect("json data should serialize")
        .event("progress")
        .id("1")),
        Ok::<_, Infallible>(SseEvent::json_data(&ProgressUpdate {
            step: 2,
            message: "processing".to_string(),
        })
        .expect("json data should serialize")
        .event("progress")
        .id("2")),
        Ok::<_, Infallible>(SseEvent::json_data(&ProgressUpdate {
            step: 3,
            message: "done".to_string(),
        })
        .expect("json data should serialize")
        .event("complete")
        .id("3")
        .retry(2_000)),
    ];

    sse_from_iter(events).keep_alive(KeepAlive::new())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting streaming example...");
    println!(" -> GET http://127.0.0.1:3000/events");

    RustApi::new()
        .route("/events", get(progress_feed))
        .run("127.0.0.1:3000")
        .await
}
