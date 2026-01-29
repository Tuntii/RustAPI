# RustAPI Benchmarking Suite

param(
    [int]$Requests = 100000,
    [int]$Concurrency = 50,
    [switch]$SkipActix = $false,
    [switch]$Internal = $false,  # Run internal cargo bench
    [switch]$Quick = $false      # Quick smoke test mode
)

$ErrorActionPreference = "Continue"

Write-Host "===================================================================" -ForegroundColor Cyan
Write-Host "       Running RustAPI Performance Benchmarking" -ForegroundColor Yellow
Write-Host "===================================================================" -ForegroundColor Cyan

if ($Quick) {
    $Requests = 1000
    $Concurrency = 10
    Write-Host "Running in Quick Mode (1000 reqs, 10 conn)" -ForegroundColor Magenta
}

# ---------------------------------------------------------
# 1. Internal Benchmarks (Criterion)
# ---------------------------------------------------------
if ($Internal -or $Quick) {
    Write-Host "`n[1/2] Running Internal Micro-benchmarks (cargo bench)..." -ForegroundColor Yellow
    # If quick, we might want to filter or run fewer, but cargo bench is usually fast enough or hard to param
    cargo bench --workspace
} else {
    Write-Host "`n[1/2] Skipping Internal Benchmarks (use -Internal to run)" -ForegroundColor DarkGray
}

# ---------------------------------------------------------
# 2. End-to-End API Benchmarks (Hey)
# ---------------------------------------------------------
Write-Host "`n[2/2] Running E2E API Benchmarks (hey)..." -ForegroundColor Yellow

# Check if hey is installed
if (-not (Get-Command "hey" -ErrorAction SilentlyContinue)) {
    $goHey = Join-Path $HOME "go\bin\hey.exe"
    if (Test-Path $goHey) {
        function Run-Hey { & $goHey @args }
    } else {
        Write-Host "X 'hey' is not installed! Skipping E2E benchmarks." -ForegroundColor Red
        Write-Host "Install with: go install github.com/rakyll/hey@latest"
        exit 0 # Don't fail entire script if only hey is missing, unless it's strictly required
    }
} else {
    function Run-Hey { & hey @args }
}

# Build servers
Write-Host "Building servers in release mode..." -ForegroundColor Yellow
cargo build --release -p bench-server 2>&1 | Out-Null
if (-not $SkipActix) {
    cargo build --release -p actix-bench-server 2>&1 | Out-Null
}
Write-Host "Build complete!" -ForegroundColor Green

$results = @{}

function Run-Benchmark {
    param ([string]$Name, [string]$Framework, [string]$Url, [string]$Method = "GET", [string]$Body = $null)

    Write-Host "  Testing: $Name" -ForegroundColor White
    $heyArgs = @("-n", $Requests, "-c", $Concurrency)
    if ($Method -eq "POST" -and $Body) {
        $heyArgs += @("-m", "POST", "-H", "Content-Type: application/json", "-d", $Body)
    }
    $heyArgs += $Url

    $output = Run-Hey @heyArgs 2>&1 | Out-String

    $rps = 0; $avgLatency = 0
    if ($output -match "Requests/sec:\s+([\d.]+)") { $rps = $Matches[1] }
    if ($output -match "Average:\s+([\d.]+)\s+secs") { $avgLatency = $Matches[1] }

    if ($rps -gt 0) {
        $key = "$Framework|$Name"
        $results[$key] = @{ Framework = $Framework; Endpoint = $Name; RPS = [double]$rps; AvgLatency = [double]$avgLatency * 1000 }
        Write-Host "    -> $rps req/s, avg: $([math]::Round([double]$avgLatency * 1000, 2))ms" -ForegroundColor Gray
    }
}

function Test-Framework {
    param ([string]$Name, [string]$Port)
    Write-Host "`nTesting $Name on port $Port" -ForegroundColor Cyan

    $retries = 10
    while ($retries -gt 0) {
        try { $null = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/" -TimeoutSec 1 -ErrorAction Stop -UseBasicParsing; break }
        catch { Start-Sleep -Milliseconds 500; $retries-- }
    }
    if ($retries -eq 0) { Write-Host "X Server not responding on port $Port" -ForegroundColor Red; return }

    Run-Benchmark -Name "Plain Text" -Framework $Name -Url "http://127.0.0.1:$Port/"
    Run-Benchmark -Name "JSON Hello" -Framework $Name -Url "http://127.0.0.1:$Port/json"

    if ($Name -eq "RustAPI") {
        Run-Benchmark -Name "POST JSON" -Framework $Name -Url "http://127.0.0.1:$Port/create-user" -Method "POST" -Body '{"name":"Test User","email":"test@example.com"}'
    } else {
        Run-Benchmark -Name "POST JSON" -Framework $Name -Url "http://127.0.0.1:$Port/users" -Method "POST" -Body '{"name":"Test User","email":"test@example.com"}'
    }
}

# Start RustAPI
$rustApiProcess = Start-Process -FilePath ".\target\release\bench-server.exe" -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
try { Test-Framework -Name "RustAPI" -Port "8080" } finally { Stop-Process -Id $rustApiProcess.Id -Force -ErrorAction SilentlyContinue }

# Start Actix
if (-not $SkipActix) {
    $actixProcess = Start-Process -FilePath ".\target\release\actix-bench-server.exe" -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 2
    try { Test-Framework -Name "Actix-web" -Port "8081" } finally { Stop-Process -Id $actixProcess.Id -Force -ErrorAction SilentlyContinue }
}

Write-Host "`nBenchmarks Complete." -ForegroundColor Green
