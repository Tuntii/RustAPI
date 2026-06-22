# Prune stale local and remote branches for RustAPI.
# Usage:
#   .\.github\scripts\prune_stale_branches.ps1 -DryRun
#   .\.github\scripts\prune_stale_branches.ps1
#   .\.github\scripts\prune_stale_branches.ps1 -DeleteRemote

param(
    [switch]$DryRun,
    [switch]$DeleteRemote,
    [int]$StaleDays = 30
)

$ErrorActionPreference = "Stop"
$repoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
Set-Location $repoRoot

$protected = @("main", "master", "gh-pages", "Production-baseline")
$stalePatterns = @("^copilot/", "^copilot-", "^sentinel-", "^secure-")

function Test-StaleBranchName([string]$Name) {
    foreach ($pattern in $stalePatterns) {
        if ($Name -match $pattern) { return $true }
    }
    return $false
}

Write-Host "Scanning branches older than $StaleDays days..."
$cutoff = (Get-Date).AddDays(-$StaleDays)

$localBranches = git for-each-ref --format="%(refname:short)|%(committerdate:iso8601)" refs/heads/ |
    ForEach-Object {
        $parts = $_ -split '\|', 2
        [PSCustomObject]@{ Name = $parts[0]; Date = [datetime]$parts[1] }
    }

$candidates = $localBranches |
    Where-Object { $protected -notcontains $_.Name } |
    Where-Object { $_.Date -lt $cutoff -or (Test-StaleBranchName $_.Name) }

if (-not $candidates) {
    Write-Host "No stale branches found."
    exit 0
}

Write-Host ""
Write-Host "Stale branch candidates:"
$candidates | ForEach-Object { Write-Host "  $($_.Name) (last commit: $($_.Date.ToString('yyyy-MM-dd')))" }

if ($DryRun) {
    Write-Host ""
    Write-Host "Dry run - no branches deleted."
    exit 0
}

foreach ($branch in $candidates) {
    Write-Host "Deleting local branch: $($branch.Name)"
    git branch -D $branch.Name
}

if ($DeleteRemote) {
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        Write-Error "gh CLI not found. Install GitHub CLI or omit -DeleteRemote."
    }
    $repo = gh repo view --json nameWithOwner -q .nameWithOwner
    $remoteBranches = git for-each-ref --format="%(refname:short)" refs/remotes/origin/ |
        ForEach-Object { $_ -replace '^origin/', '' } |
        Where-Object { $_ -ne "HEAD" -and $protected -notcontains $_ } |
        Where-Object { Test-StaleBranchName $_ }

    foreach ($branch in $remoteBranches) {
        Write-Host "Deleting remote branch: origin/$branch"
        gh api -X DELETE "repos/$repo/git/refs/heads/$branch"
    }
}

Write-Host ""
Write-Host "Done. Enable GitHub stale branch archiving:"
Write-Host "  Repository Settings - Branches - Add branch ruleset (archive after 30 days)"