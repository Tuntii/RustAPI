# Wrapper: sole verification entry point from the RustAPI workspace.
# Orchestration + artifact emission live in RustAPI-Cloud/scripts/run-verification-plan.ps1.
# Canonical dual-repo goal patch list: RustAPI/CHANGED_FILES (copied to {SCRATCH}/CHANGED_FILES.txt).
param(
    [string]$Scratch = "C:\Users\tunay\AppData\Local\Temp\grok-goal-21752d8f00a1\implementer"
)

$repoRoot = Split-Path $PSScriptRoot -Parent
$canonical = Join-Path $repoRoot "CHANGED_FILES"
if (-not (Test-Path $canonical)) {
    Write-Error "Missing canonical goal manifest: $canonical"
    exit 1
}

$cloudScript = Join-Path $repoRoot "..\RustAPI-Cloud\scripts\run-verification-plan.ps1"
Write-Host "Canonical CHANGED_FILES: $canonical"
Write-Host "Cloud verification script: $cloudScript"
& $cloudScript -Scratch $Scratch
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }