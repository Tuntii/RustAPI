# Recipe: Performance Tuning

When the API feels slow, don't guess—profile and benchmark.

## 1. Run the Suite
Use the integrated benchmark tool.

```powershell
.\benches\run_benchmarks.ps1
```

This runs:
1.  **Micro-benchmarks** (internal `cargo bench` via Criterion).
2.  **Macro-benchmarks** (external `hey` HTTP load test).

## 2. Interpret the Data
- **High Latency, Low CPU**: You are IO-bound. Check database queries (indexes?) or external API calls.
- **High Latency, High CPU**: You are CPU-bound.
    - Are you doing heavy JSON serialization?
    - Are you cloning Strings unnecessarily?
    - Are you blocking the async runtime?

## 3. Common Optimizations
- **Allocations**: Use `cow` (Clone-on-Write) or `&str` reference passing.
- **JSON**: Ensure `serde_json` is not re-parsing the same data.
- **Database**: Use connection pooling correctly (already configured in Core).

## 4. Verify
After making a change, run the benchmark script again.
- Did `Requests/sec` go up?
- Did `Average Latency` go down?
