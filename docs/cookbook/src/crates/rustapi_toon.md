# TOON (rustapi-extras `toon` feature)

**Lens**: "The Diplomat"
**Philosophy**: "Optimizing for Silicon Intelligence."

> The `rustapi-toon` crate has been merged into `rustapi-extras` behind the `toon` feature flag.
> All functionality remains identical; only the import path has changed.

Enable the feature in your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["protocol-toon"] }
```

## What is TOON?

**T**oken-**O**riented **O**bject **N**otation is a format designed to be consumed by Large Language Models (LLMs). It reduces token usage by stripping unnecessary syntax (braces, quotes) while maintaining semantic structure.

## Content Negotiation

The `LlmResponse<T>` type automatically negotiates the response format based on the `Accept` header.

```rust
async fn agent_data() -> LlmResponse<Data> {
    // Returns JSON for browsers
    // Returns TOON for AI Agents (using fewer tokens)
}
```

## Token Savings

TOON often reduces token count by 30-50% compared to JSON, saving significant costs and context window space when communicating with models like GPT-4 or Gemini.
