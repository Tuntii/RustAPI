use crate::{CompletionRequest, CompletionResponse, LlmError, LlmRouter};
use serde::de::DeserializeOwned;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// StructuredOutput — schema-first, guaranteed-valid decoding
// ---------------------------------------------------------------------------

/// Configuration for structured output extraction from LLM responses.
///
/// `StructuredOutput` wraps a [`LlmRouter`] and ensures every response is
/// deserialized into a concrete Rust type `T`. On parse failure it can
/// optionally retry with a corrective prompt.
///
/// # Example
/// ```rust,no_run
/// use rustapi_llm::{StructuredOutput, LlmRouter, CompletionRequest};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Sentiment { score: f32, label: String }
///
/// # async fn example(router: LlmRouter) {
/// let extractor = StructuredOutput::<Sentiment>::new(&router)
///     .with_max_retries(2);
///
/// let result = extractor
///     .extract(CompletionRequest::simple("Analyze: 'I love Rust!'"))
///     .await
///     .unwrap();
///
/// println!("score={}, label={}", result.score, result.label);
/// # }
/// ```
pub struct StructuredOutput<'r, T> {
    router: &'r LlmRouter,
    max_retries: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<'r, T> StructuredOutput<'r, T>
where
    T: DeserializeOwned + 'static,
{
    /// Create a structured output extractor backed by the given router.
    pub fn new(router: &'r LlmRouter) -> Self {
        Self {
            router,
            max_retries: 1,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the maximum number of retries on parse failure (default: 1).
    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Send the request and parse the response into `T`.
    ///
    /// If the first response doesn't parse, a corrective retry is attempted
    /// up to `max_retries` times, appending the parse error as context for
    /// the model.
    pub async fn extract(&self, request: CompletionRequest) -> Result<T, LlmError> {
        let mut current_request = request;
        let mut last_parse_error: Option<String> = None;

        for attempt in 0..=self.max_retries {
            // On retry, append corrective context
            if attempt > 0 {
                if let Some(ref parse_err) = last_parse_error {
                    let correction = format!(
                        "Your previous response could not be parsed as valid JSON. \
                         Error: {}. Please respond with ONLY valid JSON matching the \
                         requested schema, no markdown fences or extra text.",
                        parse_err
                    );
                    current_request
                        .messages
                        .push(crate::Message::user(correction));
                    debug!(attempt, "Retrying structured output extraction");
                }
            }

            let response = self.router.complete(current_request.clone()).await?;
            match Self::try_parse(&response) {
                Ok(parsed) => return Ok(parsed),
                Err(e) => {
                    warn!(attempt, error = %e, "Structured output parse failed");
                    // Append the assistant response so the model sees its own output
                    current_request
                        .messages
                        .push(crate::Message::assistant(&response.content));
                    last_parse_error = Some(e);
                }
            }
        }

        Err(LlmError::structured_output(format!(
            "Failed to parse structured output after {} attempts: {}",
            self.max_retries + 1,
            last_parse_error.unwrap_or_default()
        )))
    }

    /// Try to parse a completion response as `T`.
    ///
    /// Handles common LLM response quirks:
    /// - Strips markdown code fences (```json ... ```)
    /// - Trims whitespace
    fn try_parse(response: &CompletionResponse) -> Result<T, String> {
        let content = response.content.trim();

        // Strip markdown code fences
        let json_str = if content.starts_with("```") {
            let stripped = content
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            stripped
        } else {
            content
        };

        serde_json::from_str::<T>(json_str).map_err(|e| e.to_string())
    }
}

/// Convenience method on `LlmRouter` for structured output.
impl LlmRouter {
    /// Create a [`StructuredOutput`] extractor bound to this router.
    pub fn structured<T: DeserializeOwned + 'static>(&self) -> StructuredOutput<'_, T> {
        StructuredOutput::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockProvider, FinishReason, TokenUsage};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestOutput {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_structured_output_success() {
        let mock = MockProvider::new("test");
        mock.enqueue_response(CompletionResponse {
            model: "m".to_string(),
            content: r#"{"name": "hello", "value": 42}"#.to_string(),
            tool_calls: Vec::new(),
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Stop,
            provider: "test".to_string(),
        });

        let router = LlmRouter::builder().provider(mock).build();
        let result: TestOutput = router
            .structured()
            .extract(CompletionRequest::simple("test"))
            .await
            .unwrap();

        assert_eq!(result.name, "hello");
        assert_eq!(result.value, 42);
    }

    #[tokio::test]
    async fn test_structured_output_strips_code_fences() {
        let mock = MockProvider::new("test");
        mock.enqueue_response(CompletionResponse {
            model: "m".to_string(),
            content: "```json\n{\"name\": \"fenced\", \"value\": 1}\n```".to_string(),
            tool_calls: Vec::new(),
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Stop,
            provider: "test".to_string(),
        });

        let router = LlmRouter::builder().provider(mock).build();
        let result: TestOutput = router
            .structured()
            .extract(CompletionRequest::simple("test"))
            .await
            .unwrap();

        assert_eq!(result.name, "fenced");
    }

    #[tokio::test]
    async fn test_structured_output_retry_on_parse_failure() {
        let mock = MockProvider::new("test");
        // First response: invalid JSON
        mock.enqueue_response(CompletionResponse {
            model: "m".to_string(),
            content: "Sure! Here's the output: {invalid}".to_string(),
            tool_calls: Vec::new(),
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Stop,
            provider: "test".to_string(),
        });
        // Second response (after correction): valid JSON
        mock.enqueue_response(CompletionResponse {
            model: "m".to_string(),
            content: r#"{"name": "retry", "value": 99}"#.to_string(),
            tool_calls: Vec::new(),
            usage: TokenUsage::default(),
            finish_reason: FinishReason::Stop,
            provider: "test".to_string(),
        });

        let router = LlmRouter::builder().provider(mock).build();
        let result: TestOutput = router
            .structured()
            .with_max_retries(1)
            .extract(CompletionRequest::simple("test"))
            .await
            .unwrap();

        assert_eq!(result.name, "retry");
        assert_eq!(result.value, 99);
    }

    #[tokio::test]
    async fn test_structured_output_all_retries_exhausted() {
        let mock = MockProvider::new("test")
            .with_default_content("definitely not json");

        let router = LlmRouter::builder().provider(mock).build();
        let result = router
            .structured::<TestOutput>()
            .with_max_retries(1)
            .extract(CompletionRequest::simple("test"))
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LlmError::StructuredOutputError { .. }));
    }
}
