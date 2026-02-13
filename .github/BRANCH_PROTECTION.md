# Branch Protection Setup

Configure branch protection for `main` so public API checks are required.

## Required Status Checks

Add these checks as required:

- `Public API / Snapshot Drift`
- `Public API / Label Gate`

## GitHub UI Path

1. Repository `Settings`
2. `Branches`
3. Edit rule for `main`
4. Enable `Require status checks to pass before merging`
5. Add the two checks above

## API Automation (optional)

Use a fine-grained token with `Administration: Read and write` for the repository.

```powershell
$owner = "Tuntii"
$repo = "RustAPI"
$token = $env:GITHUB_TOKEN

$body = @{
  required_status_checks = @{
    strict = $true
    contexts = @(
      "Public API / Snapshot Drift",
      "Public API / Label Gate"
    )
  }
  enforce_admins = $true
  required_pull_request_reviews = @{
    required_approving_review_count = 1
  }
  restrictions = $null
} | ConvertTo-Json -Depth 10

Invoke-RestMethod `
  -Method Put `
  -Uri "https://api.github.com/repos/$owner/$repo/branches/main/protection" `
  -Headers @{
    Authorization = "Bearer $token"
    Accept = "application/vnd.github+json"
    "X-GitHub-Api-Version" = "2022-11-28"
  } `
  -Body $body `
  -ContentType "application/json"
```
