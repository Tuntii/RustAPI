param(
    [string]$DatabaseUrl = $env:DATABASE_URL
)

if (-not $DatabaseUrl) {
    $DatabaseUrl = "postgres://rustapi:rustapi@localhost:5432/rustapi_cloud"
}

$root = Split-Path $PSScriptRoot -Parent
$migrationsDir = Join-Path $root "migrations"
$files = Get-ChildItem -Path $migrationsDir -Filter "*.sql" | Sort-Object Name

foreach ($file in $files) {
    Write-Host "Applying $($file.Name)..."
    $sql = Get-Content $file.FullName -Raw
    docker compose -f (Join-Path $root "docker-compose.yml") exec -T db psql -U rustapi -d rustapi_cloud -c $sql
    if ($LASTEXITCODE -ne 0) {
        throw "Migration failed: $($file.Name)"
    }
}

Write-Host "Migrations applied."