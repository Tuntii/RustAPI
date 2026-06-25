# Wrapper: sole verification entry point from the RustAPI workspace.
param(
    [string]$Scratch = "C:\Users\tunay\AppData\Local\Temp\grok-goal-21752d8f00a1\implementer"
)

$cloudScript = Join-Path (Split-Path $PSScriptRoot -Parent) "..\RustAPI-Cloud\scripts\run-verification-plan.ps1"
& $cloudScript -Scratch $Scratch
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }