# RustAPI vs Actix-web Benchmark Script
# 
# Requires: hey (install with: go install github.com/rakyll/hey@latest)
# 
# Usage: .\run_benchmarks.ps1

param(
    [int]$Requests = 100000,
    [int]$Concurrency = 50,
    [switch]$SkipActix = $false
)

$ErrorActionPreference = "Continue"

Write-Host ""
Write-Host "===================================================================" -ForegroundColor Cyan
Write-Host "       Running RustAPI Performance Benchmark" -ForegroundColor Yellow
Write-Host "===================================================================" -ForegroundColor Cyan
Write-Host ""

# Check if hey is installed
if (-not (Get-Command "hey" -ErrorAction SilentlyContinue)) {
    $goHey = Join-Path $HOME "go\bin\hey.exe"
    if (Test-Path $goHey) {
        function Run-Hey { & $goHey @args }
    } else {
        Write-Host "X 'hey' is not installed!" -ForegroundColor Red
        Write-Host ""
        Write-Host "Install hey with:" -ForegroundColor Yellow
        Write-Host "  go install github.com/rakyll/hey@latest" -ForegroundColor White
        Write-Host ""
        Write-Host "Or download from: https://github.com/rakyll/hey/releases" -ForegroundColor White
        exit 1
    }
} else {
    function Run-Hey { & hey @args }
}

# Build servers in release mode
Write-Host "Building servers in release mode..." -ForegroundColor Yellow
cargo build --release -p bench-server 2>&1 | Out-Null
if (-not $SkipActix) {
    cargo build --release -p actix-bench-server 2>&1 | Out-Null
}
Write-Host "Build complete!" -ForegroundColor Green
Write-Host ""

# Results storage
$results = @{}

function Run-Benchmark {
    param (
        [string]$Name,
        [string]$Framework,
        [string]$Url,
        [string]$Method = "GET",
        [string]$Body = $null
    )
    
    Write-Host "  Testing: $Name" -ForegroundColor White
    
    $heyArgs = @("-n", $Requests, "-c", $Concurrency)
    
    if ($Method -eq "POST" -and $Body) {
        $heyArgs += @("-m", "POST", "-H", "Content-Type: application/json", "-d", $Body)
    }
    
    $heyArgs += $Url
    
    $output = Run-Hey @heyArgs 2>&1 | Out-String
    
    # Parse results using -match for simplicity and avoid regex type issues
    $rps = 0
    if ($output -match "Requests/sec:\s+([\d.]+)") {
        $rps = $Matches[1]
    }
    
    $avgLatency = 0
    if ($output -match "Average:\s+([\d.]+)\s+secs") {
        $avgLatency = $Matches[1]
    }
    
    if ($rps -gt 0) {
        $key = "$Framework|$Name"
        $results[$key] = @{
            Framework = $Framework
            Endpoint = $Name
            RPS = [double]$rps
            AvgLatency = [double]$avgLatency * 1000  # Convert to ms
        }
        Write-Host "    -> $rps req/s, avg: $([math]::Round([double]$avgLatency * 1000, 2))ms" -ForegroundColor Gray
    }
}

function Test-Framework {
    param (
        [string]$Name,
        [string]$Port
    )
    
    Write-Host ""
    Write-Host "Testing $Name on port $Port" -ForegroundColor Cyan
    Write-Host "---------------------------------------------" -ForegroundColor DarkGray
    
    # Wait for server to be ready
    $retries = 10
    while ($retries -gt 0) {
        try {
            $null = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/" -TimeoutSec 1 -ErrorAction Stop -UseBasicParsing
            break
        } catch {
            Start-Sleep -Milliseconds 500
            $retries--
        }
    }
    
    if ($retries -eq 0) {
        Write-Host "X Server not responding on port $Port" -ForegroundColor Red
        return
    }
    
    # Run benchmarks
    Run-Benchmark -Name "Plain Text" -Framework $Name -Url "http://127.0.0.1:$Port/"
    Run-Benchmark -Name "JSON Hello" -Framework $Name -Url "http://127.0.0.1:$Port/json"
    Run-Benchmark -Name "Path Param" -Framework $Name -Url "http://127.0.0.1:$Port/users/123"
    
    if ($Name -eq "RustAPI") {
        Run-Benchmark -Name "List Users" -Framework $Name -Url "http://127.0.0.1:$Port/users-list"
        Run-Benchmark -Name "POST JSON" -Framework $Name -Url "http://127.0.0.1:$Port/create-user" -Method "POST" -Body '{"name":"Test User","email":"test@example.com"}'
    } else {
        Run-Benchmark -Name "Two Params" -Framework $Name -Url "http://127.0.0.1:$Port/users/123/posts/456"
        Run-Benchmark -Name "List Users" -Framework $Name -Url "http://127.0.0.1:$Port/users"
        Run-Benchmark -Name "POST JSON" -Framework $Name -Url "http://127.0.0.1:$Port/users" -Method "POST" -Body '{"name":"Test User","email":"test@example.com"}'
    }
}

# Start RustAPI server
Write-Host "Starting RustAPI server..." -ForegroundColor Yellow
$rustApiProcess = Start-Process -FilePath ".\target\release\bench-server.exe" -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2

try {
    Test-Framework -Name "RustAPI" -Port "8080"
} finally {
    # Stop RustAPI server
    Stop-Process -Id $rustApiProcess.Id -Force -ErrorAction SilentlyContinue
}

if (-not $SkipActix) {
    # Start Actix-web server
    Write-Host ""
    Write-Host "Starting Actix-web server..." -ForegroundColor Yellow
    $actixProcess = Start-Process -FilePath ".\target\release\actix-bench-server.exe" -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 2
    
    try {
        Test-Framework -Name "Actix-web" -Port "8081"
    } finally {
        # Stop Actix-web server
        Stop-Process -Id $actixProcess.Id -Force -ErrorAction SilentlyContinue
    }
}

# Print results table
Write-Host ""
Write-Host ""
Write-Host "===================================================================" -ForegroundColor Cyan
Write-Host "                         RESULTS SUMMARY" -ForegroundColor Yellow
Write-Host "===================================================================" -ForegroundColor Cyan
Write-Host ""

$endpoints = @("Plain Text", "JSON Hello", "Path Param", "Two Params", "List Users", "POST JSON")

Write-Host ("{0,-15} {1,-15} {2,-15} {3,-10}" -f "Endpoint", "RustAPI", "Actix-web", "Ratio") -ForegroundColor White
Write-Host "-----------------------------------------------------------------" -ForegroundColor DarkGray

foreach ($endpoint in $endpoints) {
    $rustKey = "RustAPI|$endpoint"
    $actixKey = "Actix-web|$endpoint"
    
    $rustRPS = if ($results.ContainsKey($rustKey)) { $results[$rustKey].RPS } else { 0 }
    $actixRPS = if ($results.ContainsKey($actixKey)) { $results[$actixKey].RPS } else { 0 }
    
    $ratio = if ($actixRPS -gt 0) { [math]::Round($rustRPS / $actixRPS * 100, 1) } else { "N/A" }
    $ratioStr = if ("$ratio" -ne "N/A") { "$ratio%" } else { "N/A" }
    
    $rustStr = "$([math]::Round($rustRPS)) req/s"
    $actixStr = if ($actixRPS -gt 0) { "$([math]::Round($actixRPS)) req/s" } else { "N/A" }
    
    Write-Host ("{0,-15} {1,-15} {2,-15} {3,-10}" -f $endpoint, $rustStr, $actixStr, $ratioStr)
}

Write-Host ""
Write-Host "===================================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Configuration: $Requests requests, $Concurrency concurrent connections" -ForegroundColor Gray
Write-Host ""
