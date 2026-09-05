[CmdletBinding()]
param(
    # Optional: force a SemVer bump in Cargo.toml (e.g. -Version 0.2.1).
    # Default: use [workspace.package].version as-is (semantic release / release-plz owns bumps).
    [string]$Version
)

$ErrorActionPreference = "Stop"

# Ensure we are running from the project root
$ProjectRoot = Resolve-Path "$PSScriptRoot/.."
Set-Location $ProjectRoot

# Define crates in dependency order
$Crates = @(
    @{ Name = "rustapi-macros"; Path = "crates/rustapi-macros" },
    @{ Name = "rustapi-validate"; Path = "crates/rustapi-validate" },
    @{ Name = "rustapi-openapi"; Path = "crates/rustapi-openapi" },
    @{ Name = "rustapi-core"; Path = "crates/rustapi-core" },
    @{ Name = "rustapi-testing"; Path = "crates/rustapi-testing" },
    @{ Name = "rustapi-extras"; Path = "crates/rustapi-extras" },
    @{ Name = "rustapi-toon"; Path = "crates/rustapi-toon" },
    @{ Name = "rustapi-ws"; Path = "crates/rustapi-ws" },
    @{ Name = "rustapi-view"; Path = "crates/rustapi-view" },
    @{ Name = "rustapi-grpc"; Path = "crates/rustapi-grpc" },
    @{ Name = "rustapi-mcp"; Path = "crates/rustapi-mcp" },
    @{ Name = "rustapi-rs"; Path = "crates/rustapi-rs" },
    @{ Name = "cargo-rustapi"; Path = "crates/cargo-rustapi" }
)

# Read workspace version once
$WorkspaceContent = Get-Content "Cargo.toml" -Raw

# --- VERSIONING LOGIC (SemVer; no commit-count) ---
if ($PSBoundParameters.ContainsKey('Version') -and $Version) {
    $NewVersion = $Version.Trim()
    if ($NewVersion -notmatch '^\d+\.\d+\.\d+') {
        throw "Version must look like SemVer (e.g. 0.2.1), got: $NewVersion"
    }
    Write-Host "Using explicit version from parameter: $NewVersion" -ForegroundColor Green

    # Update [workspace.package] version only (first occurrence after that header is enough via multiline)
    if ($WorkspaceContent -match '(?ms)(\[workspace\.package\][^\[]*?version\s*=\s*")[^"]*(")') {
        $WorkspaceContent = $WorkspaceContent -replace '(?ms)(\[workspace\.package\][^\[]*?version\s*=\s*")[^"]*(")', "`${1}$NewVersion`${2}"
    } else {
        throw "Could not locate [workspace.package] version in Cargo.toml"
    }
    # Align path-dep versions under [workspace.dependencies]
    $WorkspaceContent = $WorkspaceContent -replace '(path = "crates/[^"]+", version = ")[^"]*(")', "`${1}$NewVersion`${2}"
    Set-Content "Cargo.toml" -Value $WorkspaceContent -NoNewline
    Write-Host "Updated workspace package + path-dep versions to $NewVersion" -ForegroundColor Yellow
} else {
    if ($WorkspaceContent -match '\[workspace\.package\][\s\S]*?version\s*=\s*"(.*?)"') {
        $NewVersion = $matches[1]
        Write-Host "Using workspace SemVer from Cargo.toml: $NewVersion" -ForegroundColor Green
        Write-Host "(Bumps are owned by conventional commits + release-plz; pass -Version only for a manual override.)" -ForegroundColor DarkGray
    } else {
        throw "Could not read [workspace.package].version from Cargo.toml"
    }
}
# -------------------------------------

$WorkspaceVersion = $null
if ($WorkspaceContent -match '\[workspace\.package\][\s\S]*?version\s*=\s*"(.*?)"') {
    $WorkspaceVersion = $matches[1]
}

function Get-LocalVersion {
    param ([string]$Path)
    $Content = Get-Content "$Path/Cargo.toml" -Raw
    
    # Check for specific version
    if ($Content -match '(?m)^version\s*=\s*"(.*?)"') {
        return $matches[1]
    }
    
    # Check for workspace inheritance
    if ($Content -match 'version\.workspace\s*=\s*true') {
        if ($WorkspaceVersion) {
            return $WorkspaceVersion
        }
        throw "Crate uses workspace version but could not find version in root Cargo.toml"
    }
    
    throw "Could not find version in $Path/Cargo.toml"
}

function Get-RemoteVersion {
    param ([string]$Name)
    try {
        $Url = "https://crates.io/api/v1/crates/$Name"
        $Response = Invoke-RestMethod -Uri $Url -Method Get -ErrorAction Stop
        return $Response.crate.max_version
    } catch {
        if ($_.Exception.Response.StatusCode -eq 404) {
            return $null
        }
        Write-Warning "Failed to check crates.io for $Name : $($_.Exception.Message)"
        return $null
    }
}

Write-Host "Starting Smart Publish Process..." -ForegroundColor Magenta

foreach ($Crate in $Crates) {
    $Name = $Crate.Name
    $Path = $Crate.Path

    Write-Host "Checking $Name..." -NoNewline

    $LocalVer = Get-LocalVersion -Path $Path
    $RemoteVer = Get-RemoteVersion -Name $Name

    Write-Host " Local: $LocalVer | Remote: $RemoteVer" -ForegroundColor DarkGray

    $LocalStr = [string]$LocalVer.Trim()
    $RemoteStr = if ($RemoteVer) { [string]$RemoteVer.Trim() } else { "" }

    $NeedsPublish = $false
    
    if ([string]::IsNullOrEmpty($RemoteStr)) {
        Write-Host " [NEW]" -ForegroundColor Cyan
        $NeedsPublish = $true
    }
    elseif ($LocalStr -ne $RemoteStr) {
        Write-Host " [UPDATE] $RemoteStr -> $LocalStr" -ForegroundColor Green
        $NeedsPublish = $true
    }
    else {
        Write-Host " [SKIP] Version matches" -ForegroundColor Yellow
    }
    
    if ($NeedsPublish) {
        Write-Host "   Publishing $LocalStr..." -ForegroundColor Cyan
        
        # --- CIRCULAR DEPENDENCY HACK ---
        # rustapi-core depends on rustapi-testing (dev), which depends on rustapi-core.
        # rustapi-mcp has a dev-dep on rustapi-rs (for testing the protocol-mcp feature).
        # We strip these dev-dependencies temporarily so publish succeeds before the reverse dep is on the index.
        $BackupContent = $null
        if ($Name -eq "rustapi-core") {
            try {
                $ManifestPath = "$Path/Cargo.toml"
                $RawManifest = Get-Content $ManifestPath -Raw
                $BackupContent = $RawManifest
                
                # Regex to remove rustapi-testing from lines inside [dev-dependencies]
                $ModifiedManifest = $RawManifest -replace '(?m)^rustapi-testing\s*=\s*\{.*?\}\s*$', '# STRIPPED FOR PUBLISH'
                Set-Content $ManifestPath -Value $ModifiedManifest
                Write-Host "   [HACK] Temporarily stripped rustapi-testing from $Name manifest" -ForegroundColor Yellow
            } catch {
                Write-Warning "Failed to modify manifest for circular dependency hack: $_"
                $BackupContent = $null
            }
        }
        elseif ($Name -eq "rustapi-mcp") {
            try {
                $ManifestPath = "$Path/Cargo.toml"
                $RawManifest = Get-Content $ManifestPath -Raw
                if (-not $BackupContent) { $BackupContent = $RawManifest }
                
                # Strip the dev-dep back to rustapi-rs (which will be published after mcp)
                $ModifiedManifest = $RawManifest -replace '(?m)^rustapi-rs\s*=\s*\{.*?\}\s*$', '# STRIPPED FOR PUBLISH (dev-dep on later crate)'
                Set-Content $ManifestPath -Value $ModifiedManifest
                Write-Host "   [HACK] Temporarily stripped rustapi-rs dev-dep from $Name manifest" -ForegroundColor Yellow
            } catch {
                Write-Warning "Failed to modify manifest for circular dependency hack: $_"
                if (-not $BackupContent) { $BackupContent = $null }
            }
        }
        # --------------------------------

        try {
            cargo publish -p $Name --allow-dirty --no-verify
            if ($LASTEXITCODE -ne 0) { 
                throw "Cargo publish failed"
            }
        } finally {
            # Restore manifest
            if ($BackupContent) {
                Set-Content "$Path/Cargo.toml" -Value $BackupContent
                Write-Host "   [HACK] Restored manifest for $Name" -ForegroundColor Yellow
            }
        }
        
        Write-Host "   Waiting 5s for propagation..." -ForegroundColor DarkGray
        Start-Sleep -Seconds 5
    }
}

Write-Host "Smart Publish Completed!" -ForegroundColor Magenta
