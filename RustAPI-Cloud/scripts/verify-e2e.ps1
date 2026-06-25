# End-to-end deploy verification (requires Docker Desktop + libpq)
param(
    [string]$JwtSecret = "dev-secret-change-in-production",
    [string]$CloudUrl = "http://127.0.0.1:3000"
)

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

Write-Host "==> Starting Postgres..."
docker compose -f (Join-Path $root "docker-compose.yml") up -d
Start-Sleep -Seconds 5

Write-Host "==> Applying migrations..."
& (Join-Path $root "scripts\apply-migrations.ps1")

Write-Host "==> Building port-listener fixture..."
cargo build --release --manifest-path (Join-Path $root "fixtures\port-listener\Cargo.toml")

Write-Host "==> Starting cloud server..."
$env:DATABASE_URL = "postgres://rustapi:rustapi@localhost:5432/rustapi_cloud"
$env:JWT_SECRET = $JwtSecret
$env:GITHUB_CLIENT_ID = "test"
$env:GITHUB_CLIENT_SECRET = "test"
$env:GITHUB_REDIRECT_URI = "http://localhost:3000/auth/callback"
$env:STORAGE_ROOT = Join-Path $root "storage"
$server = Start-Process -FilePath "cargo" -ArgumentList "run" -WorkingDirectory $root -PassThru -NoNewWindow
Start-Sleep -Seconds 8

try {
    Write-Host "==> Minting test JWT and deploying..."
    # Verification continues via cargo test -- --ignored when DB is up
    cargo test -- --ignored
    if ($LASTEXITCODE -ne 0) { throw "Integration tests failed" }
    Write-Host "==> E2E verification passed."
}
finally {
    if ($server -and -not $server.HasExited) {
        Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
    }
}