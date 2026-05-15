[CmdletBinding()]
param(
    [string]$RawInput
)

$ErrorActionPreference = 'Stop'

function Get-PropertyValue {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string[]]$Names
    )

    foreach ($name in $Names) {
        if ($null -ne $Object -and $Object.PSObject.Properties[$name]) {
            return $Object.$name
        }
    }

    return $null
}

function Add-Reminder {
    param(
        [System.Collections.Generic.List[string]]$List,
        [string]$Message
    )

    if (-not [string]::IsNullOrWhiteSpace($Message) -and -not $List.Contains($Message)) {
        [void]$List.Add($Message)
    }
}

$raw = if ($PSBoundParameters.ContainsKey('RawInput')) {
    $RawInput
} else {
    [Console]::In.ReadToEnd()
}

if ([string]::IsNullOrWhiteSpace($raw)) {
    return
}

try {
    $payload = $raw | ConvertFrom-Json
} catch {
    return
}

$eventName = Get-PropertyValue -Object $payload -Names @('hookEventName', 'eventName')
if (-not $eventName) {
    $hookSpecific = Get-PropertyValue -Object $payload -Names @('hookSpecificInput', 'hookSpecificData')
    if ($hookSpecific) {
        $eventName = Get-PropertyValue -Object $hookSpecific -Names @('hookEventName', 'eventName')
    }
}

if ($eventName -and $eventName -ne 'PreToolUse') {
    return
}

$toolName = Get-PropertyValue -Object $payload -Names @('toolName', 'tool', 'tool_name')
if (-not $toolName) {
    $toolPayload = Get-PropertyValue -Object $payload -Names @('toolInput', 'input', 'parameters', 'arguments')
    if ($toolPayload) {
        $toolName = Get-PropertyValue -Object $toolPayload -Names @('toolName', 'tool', 'tool_name')
    }
}

$normalized = $raw.ToLowerInvariant()
$reminders = [System.Collections.Generic.List[string]]::new()

if ($normalized -match 'crates[\\/]+rustapi-rs[\\/]|api[\\/]+public[\\/]|contract\.md|cargo\.toml') {
    Add-Reminder -List $reminders -Message 'Public API-adjacent files are in play: check CONTRACT.md, public API snapshots, labels (`feature` / `breaking`), and changelog follow-up.'
}

if ($normalized -match 'changelog\.md|releases\.md') {
    Add-Reminder -List $reminders -Message 'Release-facing docs are being touched: keep entries user-focused, add migration notes for breaking changes, and call out MSRV changes explicitly.'
}

if ($normalized -match 'docs[\\/]+cookbook|[\\/]examples[\\/]|crates[\\/].+[\\/]examples[\\/]') {
    Add-Reminder -List $reminders -Message 'Examples or cookbook content are involved: prefer `use rustapi_rs::prelude::*;`, keep the sample teachable, and verify surrounding docs still match.'
}

if ($normalized -match 'crates[\\/].+\.rs|tests[\\/].+\.rs') {
    Add-Reminder -List $reminders -Message 'Rust source is being changed: prefer targeted crate validation first, then widen to workspace checks only if the scope really demands it.'
}

$dangerousPatterns = @(
    @{ Pattern = 'git\s+push\b'; Reason = 'Pushing remote changes should stay an explicit human decision.' },
    @{ Pattern = 'git\s+reset\s+--hard\b'; Reason = 'Hard resets discard local state and deserve a pause.' },
    @{ Pattern = 'git\s+clean\b[^\r\n]*\s-f'; Reason = 'Force-clean commands can remove untracked work.' },
    @{ Pattern = 'git\s+checkout\s+--\b'; Reason = 'Checkout-overwrite commands can discard local edits.' },
    @{ Pattern = 'git\s+restore\b[^\r\n]*--source\b'; Reason = 'Restore with an explicit source can overwrite working-tree state.' },
    @{ Pattern = 'gh\s+pr\s+merge\b'; Reason = 'Merging a PR is a release-significant action and should be deliberate.' },
    @{ Pattern = 'remove-item\b[^\r\n]*-recurse\b[^\r\n]*-force\b'; Reason = 'Recursive forced deletion should be confirmed before running.' }
)

$needsConfirmation = $false
$confirmationReason = $null

foreach ($entry in $dangerousPatterns) {
    if ($normalized -match $entry.Pattern) {
        $needsConfirmation = $true
        $confirmationReason = $entry.Reason
        break
    }
}

if ($needsConfirmation) {
    $response = [ordered]@{
        continue = $true
        systemMessage = if ($reminders.Count -gt 0) { $reminders -join "`n" } else { 'Potentially destructive terminal action detected.' }
        hookSpecificOutput = [ordered]@{
            hookEventName = 'PreToolUse'
            permissionDecision = 'ask'
            permissionDecisionReason = $confirmationReason
        }
    }

    Write-Output ($response | ConvertTo-Json -Depth 10 -Compress)
    return
}

if ($reminders.Count -gt 0) {
    $response = [ordered]@{
        continue = $true
        systemMessage = $reminders -join "`n"
    }

    Write-Output ($response | ConvertTo-Json -Depth 10 -Compress)
}
