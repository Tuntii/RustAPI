# Cloud verification moved to the RustAPI-Cloud repository.
param(
    [string]$CloudRepo = $env:RUSTAPI_CLOUD_REPO,
    [string]$Scratch = (Join-Path $env:TEMP "rustapi-cloud-verify")
)

$ErrorActionPreference = "Stop"
if (-not $CloudRepo) {
    $CloudRepo = Join-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) "RustAPI-Cloud"
}
$script = Join-Path $CloudRepo "scripts\run-verification-plan.ps1"
if (-not (Test-Path $script)) {
    throw "RustAPI-Cloud not found at $CloudRepo. Clone https://github.com/Tuntii/RustAPI-Cloud or set RUSTAPI_CLOUD_REPO."
}
$env:RUSTAPI_REPO = Split-Path $PSScriptRoot -Parent
& $script -Scratch $Scratch -RustApiRepo $env:RUSTAPI_REPO
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }