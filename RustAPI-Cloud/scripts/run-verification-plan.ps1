# Deprecated: use RustAPI/scripts/run-verification-plan.ps1 (in-repo orchestrator).
& (Join-Path $PSScriptRoot "..\..\scripts\run-verification-plan.ps1") @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }