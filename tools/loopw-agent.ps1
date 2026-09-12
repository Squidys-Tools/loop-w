[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string] $Command = 'help',

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CommandArgs = @()
)

<#
.SYNOPSIS
    Agent-friendly control surface for building and exercising LoopW on Windows.

.DESCRIPTION
    This wrapper keeps process lifetime and result formatting predictable while
    reusing the existing Windows integration harness for the actual Win32 checks.
    It never prompts. Commands that change process state require explicit flags.

.EXAMPLE
    .\tools\loopw-agent.ps1 doctor --json

.EXAMPLE
    .\tools\loopw-agent.ps1 run --suite all --profile release --build --json

.EXAMPLE
    $run = .\tools\loopw-agent.ps1 launch --profile release --json | ConvertFrom-Json
    .\tools\loopw-agent.ps1 status --run-id $run.run_id --json
    .\tools\loopw-agent.ps1 stop --run-id $run.run_id --yes --json
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:RepoRoot = Split-Path -Parent $PSScriptRoot
$script:JsonOutput = $false
$script:StateRoot = Join-Path ([IO.Path]::GetTempPath()) 'LoopW\agent-runs'

function Show-Help {
    param([string] $Topic = '')

    switch ($Topic.ToLowerInvariant()) {
        'doctor' {
            @'
doctor - check the local build and harness prerequisites without starting LoopW.

Options:
  --exe <path>       Executable to inspect (default: target\release\LoopW.exe)
  --profile <name>   debug or release (default: release)
  --json             Emit one JSON object

Examples:
  .\tools\loopw-agent.ps1 doctor
  .\tools\loopw-agent.ps1 doctor --profile debug --json
'@ | Write-Output
        }
        'launch' {
            @'
launch - start one hidden, tracked LoopW process.

Options:
  --exe <path>       Executable to start
  --profile <name>   debug or release (default: release)
  --build            Build the selected profile first
  --json             Emit one JSON object containing run_id and pid

Examples:
  .\tools\loopw-agent.ps1 launch --profile release --build --json
  .\tools\loopw-agent.ps1 launch --exe .\target\debug\LoopW.exe
'@ | Write-Output
        }
        'status' {
            @'
status - inspect a process started by launch.

Options:
  --run-id <id>      Exact run identifier returned by launch
  --json             Emit one JSON object

Examples:
  .\tools\loopw-agent.ps1 status --run-id <run-id>
  .\tools\loopw-agent.ps1 status --run-id <run-id> --json
'@ | Write-Output
        }
        'run' {
            @'
run - execute the bounded Windows integration suite and return its result.

Options:
  --suite all        Current suite; defaults to all
  --exe <path>       Executable to test
  --profile <name>   debug or release (default: release)
  --build            Build the selected profile first
  --skip-radial      Skip synthetic trigger input checks
  --timeout <secs>   Per-check timeout (default: 15)
  --evidence-dir <p> Save the raw log and result JSON in this directory
  --json             Emit one JSON result object

Examples:
  .\tools\loopw-agent.ps1 run --suite all --profile release --build
  .\tools\loopw-agent.ps1 run --skip-radial --evidence-dir .\artifacts\run --json
'@ | Write-Output
        }
        'stop' {
            @'
stop - terminate the exact process recorded by launch.

Options:
  --run-id <id>      Exact run identifier returned by launch
  --dry-run          Report what would be stopped without changing state
  --yes              Required to perform the stop
  --json             Emit one JSON object

Examples:
  .\tools\loopw-agent.ps1 stop --run-id <run-id> --dry-run --json
  .\tools\loopw-agent.ps1 stop --run-id <run-id> --yes
'@ | Write-Output
        }
        default {
            @'
LoopW agent CLI

Usage:
  .\tools\loopw-agent.ps1 <command> [options]

Commands:
  doctor    Check executable, cargo, harness, and resident-instance state
  launch   Start one hidden, tracked LoopW process
  status   Inspect a tracked process
  run      Run the bounded Windows integration suite
  stop     Stop one tracked process with explicit confirmation
  help     Show this help or <command> --help

Examples:
  .\tools\loopw-agent.ps1 doctor --json
  .\tools\loopw-agent.ps1 run --suite all --profile release --build --json
  .\tools\loopw-agent.ps1 launch --profile release --json
  .\tools\loopw-agent.ps1 stop --run-id <run-id> --yes --json
'@ | Write-Output
        }
    }
}

function Parse-Options {
    param(
        [string[]] $Tokens,
        [string[]] $Allowed,
        [string[]] $Flags
    )

    $options = @{}
    $positionals = New-Object System.Collections.Generic.List[string]
    for ($index = 0; $index -lt $Tokens.Count; $index++) {
        $token = [string]$Tokens[$index]
        if ($token -eq '--') {
            for ($rest = $index + 1; $rest -lt $Tokens.Count; $rest++) {
                [void]$positionals.Add([string]$Tokens[$rest])
            }
            break
        }
        if (-not $token.StartsWith('--', [StringComparison]::Ordinal)) {
            [void]$positionals.Add($token)
            continue
        }

        $pair = $token.Substring(2).Split('=', 2)
        $name = $pair[0].ToLowerInvariant()
        if ($Allowed -notcontains $name) {
            throw "Unknown option '--$name'. Run '.\tools\loopw-agent.ps1 $Command --help'."
        }
        if ($Flags -contains $name) {
            if ($pair.Count -gt 1) { throw "Flag '--$name' does not take a value." }
            $options[$name] = $true
            continue
        }
        if ($pair.Count -gt 1) {
            $options[$name] = $pair[1]
            continue
        }
        if ($index + 1 -ge $Tokens.Count -or [string]$Tokens[$index + 1] -match '^--') {
            throw "Option '--$name' needs a value. Run '.\tools\loopw-agent.ps1 $Command --help'."
        }
        $index++
        $options[$name] = [string]$Tokens[$index]
    }
    $options['_'] = @($positionals)
    return $options
}

function Get-Option {
    param($Options, [string] $Name, $Default = $null)
    if ($Options.ContainsKey($Name)) { return $Options[$Name] }
    return $Default
}

function Resolve-Profile {
    param($Options)
    $profile = [string](Get-Option $Options 'profile' 'release')
    if ($profile -notin @('debug', 'release')) {
        throw "Invalid profile '$profile'. Choose 'debug' or 'release'."
    }
    return $profile
}

function Resolve-ExePath {
    param($Options)
    if ($Options.ContainsKey('exe')) {
        $candidate = [string]$Options['exe']
        if (-not [IO.Path]::IsPathRooted($candidate)) {
            $candidate = Join-Path (Get-Location).Path $candidate
        }
        return [IO.Path]::GetFullPath($candidate)
    }
    $profile = Resolve-Profile $Options
    return Join-Path $script:RepoRoot ("target\{0}\LoopW.exe" -f $profile)
}

function Build-Profile {
    param([string] $Profile)
    $cargoArgs = @('build', '--locked')
    if ($Profile -eq 'release') { $cargoArgs += '--release' }
    $cargo = (Get-Command cargo.exe -ErrorAction Stop).Source
    $outputPath = Join-Path ([IO.Path]::GetTempPath()) ("LoopW-cargo-{0}.out" -f [Guid]::NewGuid().ToString('N'))
    $errorPath = Join-Path ([IO.Path]::GetTempPath()) ("LoopW-cargo-{0}.err" -f [Guid]::NewGuid().ToString('N'))
    try {
        $buildProcess = Start-Process -FilePath $cargo -ArgumentList $cargoArgs -WorkingDirectory $script:RepoRoot -Wait -PassThru -NoNewWindow -RedirectStandardOutput $outputPath -RedirectStandardError $errorPath
        $cargoExitCode = $buildProcess.ExitCode
        $output = @()
        if (Test-Path -LiteralPath $outputPath) { $output += @(Get-Content -LiteralPath $outputPath) }
        if (Test-Path -LiteralPath $errorPath) { $output += @(Get-Content -LiteralPath $errorPath) }
    } finally {
        Remove-Item -LiteralPath $outputPath, $errorPath -Force -ErrorAction SilentlyContinue
    }
    if ($cargoExitCode -ne 0) {
        $output = @($output | ForEach-Object { $_.ToString() })
        $tail = ($output | Select-Object -Last 12) -join [Environment]::NewLine
        throw "cargo build failed for profile '$Profile'.$([Environment]::NewLine)$tail"
    }
    if (-not $script:JsonOutput) { $output | ForEach-Object { Write-Host $_ } }
    if (-not $script:JsonOutput) { Write-Host "Built $Profile profile." }
}

function Assert-Executable {
    param([string] $ExePath, [bool] $Build, [string] $Profile)
    if ($Build) { Build-Profile $Profile }
    if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
        if (-not $Build) {
            throw "LoopW executable not found at '$ExePath'. Run with '--build' or build it first."
        }
    }
    if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
        throw "Build completed but '$ExePath' was not produced."
    }
}

function Get-ExistingInstance {
    try {
        $mutex = [Threading.Mutex]::OpenExisting('Local\LoopW.Instance')
        if ($null -ne $mutex) { $mutex.Dispose(); return $true }
    } catch [Threading.WaitHandleCannotBeOpenedException] {
        return $false
    }
    return $false
}

function Write-Result {
    param($Value)
    if ($script:JsonOutput) {
        $Value | ConvertTo-Json -Depth 8 -Compress | Write-Output
    } else {
        $Value
    }
}

function Get-StatePath {
    param([string] $RunId)
    if ($RunId -notmatch '^[0-9a-fA-F]{32}$') {
        throw "Invalid run id '$RunId'. Use the exact run_id returned by launch."
    }
    return Join-Path $script:StateRoot "$RunId.json"
}

function Read-RunState {
    param([string] $RunId)
    $path = Get-StatePath $RunId
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "No tracked run exists for '$RunId'."
    }
    return Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
}

function Get-ProcessPath {
    param([Diagnostics.Process] $Process)
    try { return $Process.MainModule.FileName } catch { return $null }
}

function Get-RunStatus {
    param($State)
    $process = $null
    try { $process = Get-Process -Id ([int]$State.pid) -ErrorAction Stop } catch { }
    $running = $null -ne $process
    $pathMatches = $false
    if ($running) {
        $processPath = Get-ProcessPath $process
        $pathMatches = $null -ne $processPath -and
            [StringComparer]::OrdinalIgnoreCase.Equals([IO.Path]::GetFullPath($processPath), [IO.Path]::GetFullPath([string]$State.exe_path))
    }
    return [ordered]@{
        ok = $true
        run_id = [string]$State.run_id
        pid = [int]$State.pid
        exe_path = [string]$State.exe_path
        started_utc = [string]$State.started_utc
        running = $running
        process_path_matches = $pathMatches
    }
}

function Invoke-Doctor {
    param($Options)
    $profile = Resolve-Profile $Options
    $exe = Resolve-ExePath $Options
    $cargo = Get-Command cargo.exe -ErrorAction SilentlyContinue
    $harness = Join-Path $script:RepoRoot 'tools\windows-integration.ps1'
    $target = Join-Path $script:RepoRoot 'tools\windows-integration-target.ps1'
    $result = [ordered]@{
        ok = (Test-Path -LiteralPath $exe -PathType Leaf) -and
            ($null -ne $cargo) -and
            (Test-Path -LiteralPath $harness -PathType Leaf) -and
            (Test-Path -LiteralPath $target -PathType Leaf) -and
            (-not (Get-ExistingInstance))
        repo_root = $script:RepoRoot
        profile = $profile
        exe_path = $exe
        executable_exists = Test-Path -LiteralPath $exe -PathType Leaf
        cargo = if ($null -ne $cargo) { $cargo.Source } else { $null }
        harness_exists = Test-Path -LiteralPath $harness -PathType Leaf
        target_exists = Test-Path -LiteralPath $target -PathType Leaf
        resident_instance_detected = Get-ExistingInstance
    }
    Write-Result $result
    if (-not $result.ok) { exit 1 }
}

function Invoke-Launch {
    param($Options)
    $profile = Resolve-Profile $Options
    $exe = Resolve-ExePath $Options
    Assert-Executable $exe ($Options.ContainsKey('build')) $profile
    if (Get-ExistingInstance) {
        throw 'A LoopW instance is already running. Use the existing run id or stop it first.'
    }
    New-Item -ItemType Directory -Force -Path $script:StateRoot | Out-Null
    $process = Start-Process -FilePath $exe -WorkingDirectory $script:RepoRoot -WindowStyle Hidden -PassThru
    Start-Sleep -Milliseconds 150
    $process.Refresh()
    if ($process.HasExited) { throw "LoopW exited immediately with code $($process.ExitCode)." }
    $runId = [Guid]::NewGuid().ToString('N')
    $state = [ordered]@{
        run_id = $runId
        pid = $process.Id
        exe_path = $exe
        started_utc = [DateTime]::UtcNow.ToString('o')
    }
    $statePath = Get-StatePath $runId
    try {
        $state | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $statePath -Encoding UTF8
    } catch {
        try { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue } catch { }
        throw "Could not record launch state; the newly started process was stopped. $($_.Exception.Message)"
    }
    Write-Result $state
}

function Invoke-Status {
    param($Options)
    if (-not $Options.ContainsKey('run-id')) { throw "Missing '--run-id'. Example: .\tools\loopw-agent.ps1 status --run-id <run-id>." }
    Write-Result (Get-RunStatus (Read-RunState ([string]$Options['run-id'])))
}

function Invoke-Stop {
    param($Options)
    if (-not $Options.ContainsKey('run-id')) { throw "Missing '--run-id'. Example: .\tools\loopw-agent.ps1 stop --run-id <run-id> --yes." }
    $runId = [string]$Options['run-id']
    $state = Read-RunState $runId
    $status = Get-RunStatus $state
    if (-not $status.running) {
        Remove-Item -LiteralPath (Get-StatePath $runId) -Force
        $status.stopped = $true
        $status.already_stopped = $true
        Write-Result $status
        return
    }
    if (-not $status.process_path_matches) { throw 'Tracked PID no longer belongs to the recorded executable; refusing to stop it.' }
    if ($Options.ContainsKey('dry-run')) {
        $status.would_stop = $true
        Write-Result $status
        return
    }
    if (-not $Options.ContainsKey('yes')) { throw "Stopping a process requires '--yes' or '--dry-run'." }
    Stop-Process -Id ([int]$state.pid) -Force
    $stopped = Wait-Process -Id ([int]$state.pid) -Timeout 5 -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Get-StatePath $runId) -Force
    $status.stopped = $true
    $status.already_stopped = $false
    Write-Result $status
}

function Invoke-Run {
    param($Options)
    $suite = [string](Get-Option $Options 'suite' 'all')
    if ($suite -ne 'all') { throw "Unsupported suite '$suite'. Available suites: all." }
    $profile = Resolve-Profile $Options
    $exe = Resolve-ExePath $Options
    Assert-Executable $exe ($Options.ContainsKey('build')) $profile
    $harness = Join-Path $script:RepoRoot 'tools\windows-integration.ps1'
    $timeout = [int](Get-Option $Options 'timeout' 15)
    if ($timeout -lt 1 -or $timeout -gt 300) { throw "Timeout must be between 1 and 300 seconds." }

    $harnessArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $harness,
        '-ExePath', $exe, '-TimeoutSeconds', $timeout)
    if ($Options.ContainsKey('skip-radial')) { $harnessArgs += '-SkipRadialInput' }
    $started = [DateTime]::UtcNow
    $lines = @()
    Push-Location $script:RepoRoot
    try {
        $lines = @(& powershell.exe @harnessArgs 2>&1 | ForEach-Object { $_.ToString() })
        $exitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    $durationMs = [int](([DateTime]::UtcNow - $started).TotalMilliseconds)
    $passed = @($lines | Where-Object { $_ -match '^PASS\s' }).Count
    $skipped = @($lines | Where-Object { $_ -match '^SKIP\s' }).Count
    $result = [ordered]@{
        ok = ($exitCode -eq 0)
        suite = $suite
        profile = $profile
        exe_path = $exe
        exit_code = $exitCode
        passed = $passed
        skipped = $skipped
        duration_ms = $durationMs
        output = @($lines)
    }

    if ($Options.ContainsKey('evidence-dir')) {
        $evidence = [string]$Options['evidence-dir']
        if (-not [IO.Path]::IsPathRooted($evidence)) { $evidence = Join-Path (Get-Location).Path $evidence }
        New-Item -ItemType Directory -Force -Path $evidence | Out-Null
        $stamp = [DateTime]::Now.ToString('yyyyMMdd-HHmmss')
        $logPath = Join-Path $evidence "windows-integration-$stamp.log"
        $resultPath = Join-Path $evidence "windows-integration-$stamp.json"
        $lines | Set-Content -LiteralPath $logPath -Encoding UTF8
        $result.log_path = [IO.Path]::GetFullPath($logPath)
        $result.result_path = [IO.Path]::GetFullPath($resultPath)
        $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $resultPath -Encoding UTF8
    }

    if (-not $script:JsonOutput) {
        $lines | Write-Output
        Write-Output ("Result: {0}; passed={1}; skipped={2}; duration_ms={3}" -f $result.ok, $passed, $skipped, $durationMs)
    } else {
        Write-Result $result
    }
    if (-not $result.ok) { exit $exitCode }
}

try {
    $normalized = $Command.ToLowerInvariant()
    if ($CommandArgs -contains '--json' -or $CommandArgs -match '^--json=') { $script:JsonOutput = $true }
    if ($CommandArgs -contains '--help' -or $CommandArgs -contains '-h') {
        Show-Help $normalized
        exit 0
    }

    switch ($normalized) {
        'help' {
            $topic = if ($CommandArgs.Count -gt 0) { [string]$CommandArgs[0] } else { '' }
            Show-Help $topic
        }
        'doctor' {
            $options = Parse-Options $CommandArgs @('exe', 'profile', 'json') @('json')
            Invoke-Doctor $options
        }
        'launch' {
            $options = Parse-Options $CommandArgs @('exe', 'profile', 'build', 'json') @('build', 'json')
            Invoke-Launch $options
        }
        'status' {
            $options = Parse-Options $CommandArgs @('run-id', 'json') @('json')
            Invoke-Status $options
        }
        'run' {
            $options = Parse-Options $CommandArgs @('suite', 'exe', 'profile', 'build', 'skip-radial', 'timeout', 'evidence-dir', 'json') @('build', 'skip-radial', 'json')
            Invoke-Run $options
        }
        'stop' {
            $options = Parse-Options $CommandArgs @('run-id', 'dry-run', 'yes', 'json') @('dry-run', 'yes', 'json')
            Invoke-Stop $options
        }
        default { throw "Unknown command '$Command'. Run '.\tools\loopw-agent.ps1 --help'." }
    }
} catch {
    $message = $_.Exception.Message
    if ($script:JsonOutput) {
        [ordered]@{ ok = $false; error = $message } | ConvertTo-Json -Depth 4 -Compress | Write-Output
    } else {
        [Console]::Error.WriteLine("Error: $message")
    }
    exit 1
}
