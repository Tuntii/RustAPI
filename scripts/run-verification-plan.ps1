# Sole verification entry point — in-repo RustAPI + RustAPI-Cloud mirror.
param(
    [string]$Scratch = "C:\Users\tunay\AppData\Local\Temp\grok-goal-21752d8f00a1\implementer"
)

$ErrorActionPreference = "Stop"
$rustapiRoot = Split-Path $PSScriptRoot -Parent
$cloudRoot = Join-Path $rustapiRoot "RustAPI-Cloud"

Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "listener*" -or $_.Name -like "port-listener*" } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Seconds 1
if (Test-Path $Scratch) {
    cmd /c "rmdir /s /q `"$Scratch`"" | Out-Null
}
New-Item -ItemType Directory -Force -Path $Scratch | Out-Null

Write-Host "==> [1] DB schema (docker-compose + psql; pg-embed fallback if engine unavailable)"
Push-Location $cloudRoot
$schemaPath = Join-Path $Scratch "db-schema.txt"
try {
    docker compose up -d 2>&1 | Out-File (Join-Path $Scratch "db-setup.log") -Encoding utf8
    Start-Sleep -Seconds 8
    Get-ChildItem (Join-Path $cloudRoot "migrations") -Filter "*.sql" | Sort-Object Name | ForEach-Object {
        Get-Content $_.FullName -Raw | docker compose exec -T db psql -U rustapi -d rustapi_cloud 2>&1 |
            Out-File (Join-Path $Scratch "db-setup.log") -Append -Encoding utf8
    }
    "-- docker compose + psql schema dump`n" | Out-File $schemaPath -Encoding utf8
    $schemaSql = @"
SELECT table_name, column_name, data_type
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name IN ('projects', 'deploys')
ORDER BY table_name, ordinal_position;
"@
    docker compose exec -T db psql -U rustapi -d rustapi_cloud -c $schemaSql 2>&1 |
        Out-File $schemaPath -Append -Encoding utf8
    $env:DATABASE_URL = "postgres://rustapi:rustapi@localhost:5432/rustapi_cloud"
    Write-Host "Docker Postgres ready."
}
catch {
    "DOCKER_ENGINE_UNAVAILABLE: $_`nFALLBACK: pg-embed schema appended by verify-deploy-e2e (step 3).`n" |
        Out-File $schemaPath -Encoding utf8
    Remove-Item Env:DATABASE_URL -ErrorAction SilentlyContinue
}

Write-Host "==> [2] cargo test (RustAPI-Cloud mirror)"
$cloudTestsLog = Join-Path $Scratch "cloud-tests.log"
cmd /c "cargo test -- --test-threads=1 --nocapture > `"$cloudTestsLog`" 2>&1"
if ($LASTEXITCODE -ne 0) { throw "cargo test failed (exit $LASTEXITCODE)" }

Write-Host "==> [3-4] verify-deploy-e2e (server boot + deploy + cargo run status 2x)"
$env:VERIFY_SCRATCH = $Scratch
$env:JWT_SECRET = "dev-secret-change-in-production"
if (-not $env:DATABASE_URL) {
    Write-Host "No DATABASE_URL - verify-deploy-e2e will start pg-embed"
}
cmd /c "cargo build --bin verify-deploy-e2e > NUL 2>&1"
if ($LASTEXITCODE -ne 0) { throw "verify-deploy-e2e build failed (exit $LASTEXITCODE)" }
$e2eBin = Join-Path $cloudRoot "target\debug\verify-deploy-e2e.exe"
& $e2eBin
if ($LASTEXITCODE -ne 0) { throw "verify-deploy-e2e failed (exit $LASTEXITCODE)" }

foreach ($artifact in @("deploy-flow.log", "cli-status.log", "deploy-state.json", "verify-e2e-console.log")) {
    if (-not (Test-Path (Join-Path $Scratch $artifact))) {
        throw "missing artifact $artifact"
    }
}
if (-not (Select-String -Path (Join-Path $Scratch "cli-status.log") -Pattern "cargo run -p cargo-rustapi" -Quiet)) {
    throw "cli-status.log missing cargo run invocations"
}
if (-not (Select-String -Path (Join-Path $Scratch "deploy-flow.log") -Pattern "DEPLOY_FLOW_OK" -Quiet)) {
    throw "deploy-flow.log missing DEPLOY_FLOW_OK"
}
if (-not (Select-String -Path (Join-Path $Scratch "deploy-flow.log") -Pattern ">>> REQUEST" -Quiet)) {
    throw "deploy-flow.log missing full HTTP transcripts"
}
if (-not (Select-String -Path (Join-Path $Scratch "verify-e2e-console.log") -Pattern "VERIFY_E2E_OK" -Quiet)) {
    throw "verify-e2e-console.log missing VERIFY_E2E_OK"
}

Write-Host "==> [5] cargo check"
$checksLog = Join-Path $Scratch "checks.log"
Pop-Location
Push-Location $rustapiRoot
cmd /c "cargo check -p cargo-rustapi --features cloud > `"$checksLog`" 2>&1"
$checkCli = $LASTEXITCODE
Pop-Location
Push-Location $cloudRoot
cmd /c "cargo check >> `"$checksLog`" 2>&1"
$checkCloud = $LASTEXITCODE
Pop-Location
if ($checkCli -ne 0 -or $checkCloud -ne 0) {
    throw "cargo check failed (cli=$checkCli cloud=$checkCloud)"
}

Write-Host "==> [6] changed-files manifest (git diff in RustAPI repo)"
$canonicalPath = Join-Path $rustapiRoot "CHANGED_FILES"
if (-not (Test-Path $canonicalPath)) {
    throw "missing CHANGED_FILES at repo root"
}
$canonicalLines = Get-Content $canonicalPath |
    Where-Object { $_ -and -not $_.StartsWith("#") } |
    ForEach-Object { $_.Trim() }

$requiredDrivers = @(
    "RustAPI-Cloud/src/bin/verify-deploy-e2e.rs"
    "RustAPI-Cloud/src/verify_db.rs"
    "scripts/run-verification-plan.ps1"
)
foreach ($driver in $requiredDrivers) {
    if ($canonicalLines -notcontains $driver) {
        throw "CHANGED_FILES missing required driver: $driver"
    }
}

$gitDiff = @(
    git -C $rustapiRoot diff --name-only
    git -C $rustapiRoot diff --name-only --cached
    git -C $rustapiRoot diff --name-only origin/main..HEAD
    git -C $rustapiRoot ls-files --others --exclude-standard
) | Where-Object { $_ } | Sort-Object -Unique

if ($gitDiff.Count -eq 0) {
    throw "git diff produced no files under RustAPI repo"
}

foreach ($entry in $canonicalLines) {
    if ($gitDiff -notcontains $entry) {
        throw "CHANGED_FILES entry not in git diff: $entry"
    }
    $diskPath = Join-Path $rustapiRoot $entry
    if (-not (Test-Path $diskPath)) {
        throw "CHANGED_FILES entry not found on disk: $entry"
    }
}

$head = git -C $rustapiRoot rev-parse --short HEAD
$manifest = Join-Path $Scratch "changed-files.txt"
$lines = @(
    "# Verification changed-files manifest"
    "# Generated: $(Get-Date -Format o)"
    "# Source: git diff (working tree + staged + origin/main..HEAD) in RustAPI repo"
    "# HEAD: $head"
    ""
    $gitDiff
)
$lines | Out-File $manifest -Encoding utf8
Copy-Item $manifest (Join-Path $Scratch "CHANGED_FILES.txt") -Force

$cloudCount = ($gitDiff | Where-Object { $_ -like "RustAPI-Cloud/*" }).Count
if ($cloudCount -eq 0) {
    throw "git diff missing RustAPI-Cloud/* paths"
}

Write-Host "==> Verification complete: $Scratch ($($gitDiff.Count) files, $cloudCount under RustAPI-Cloud/)"