[CmdletBinding()]
param(
    [ValidateSet('rust', 'wpf', 'both')]
    [string] $Target = 'both',
    [string] $RustExe = (Join-Path (Get-Location) 'target\release\LoopW.exe'),
    [string] $WpfExe = '',
    [int] $Samples = 3,
    [string] $EvidenceDir = (Join-Path ([IO.Path]::GetTempPath()) ('LoopW\verification\performance-' + [Guid]::NewGuid().ToString('N')))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:TimeoutSeconds = 15
$script:RepoRoot = Split-Path -Parent $PSScriptRoot
$script:StartedProcesses = New-Object System.Collections.Generic.List[int]

# Reuse the existing user32 declarations and settings/trigger helpers without
# duplicating the integration harness. Only the declarations and functions
# before its executable test body are loaded.
$integration = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'windows-integration.ps1')
$helperStart = $integration.IndexOf('Set-StrictMode -Version Latest', [StringComparison]::Ordinal)
$helperEnd = $integration.LastIndexOf("`ntry {", [StringComparison]::Ordinal)
if ($helperStart -lt 0 -or $helperEnd -le $helperStart) {
    throw 'Could not load the shared Windows integration helpers.'
}
Invoke-Expression $integration.Substring($helperStart, $helperEnd - $helperStart)

Add-Type -AssemblyName System.Drawing
$drawingAssembly = [System.Drawing.Bitmap].Assembly.Location
Add-Type -ReferencedAssemblies $drawingAssembly -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public static class LoopWPerformanceCapture {
    public static ulong HashScreen(int left, int top, int width, int height) {
        if (width <= 0 || height <= 0) return 0;
        using (var bitmap = new Bitmap(width, height, PixelFormat.Format32bppArgb))
        using (var graphics = Graphics.FromImage(bitmap)) {
            graphics.CopyFromScreen(left, top, 0, 0, new Size(width, height), CopyPixelOperation.SourceCopy);
            var data = bitmap.LockBits(
                new Rectangle(0, 0, width, height),
                ImageLockMode.ReadOnly,
                PixelFormat.Format32bppArgb);
            try {
                var bytes = new byte[Math.Abs(data.Stride) * height];
                Marshal.Copy(data.Scan0, bytes, 0, bytes.Length);
                ulong hash = 1469598103934665603UL;
                foreach (var value in bytes) {
                    hash ^= value;
                    hash *= 1099511628211UL;
                }
                return hash;
            } finally {
                bitmap.UnlockBits(data);
            }
        }
    }
}
'@

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class LoopWPerformanceInput {
    [StructLayout(LayoutKind.Sequential)]
    private struct KeyboardInput {
        public ushort VirtualKey;
        public ushort ScanCode;
        public uint Flags;
        public uint Time;
        public IntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Explicit, Size = 40)]
    private struct NativeInput {
        [FieldOffset(0)] public uint Type;
        [FieldOffset(8)] public KeyboardInput Keyboard;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint count, NativeInput[] inputs, int size);
    [DllImport("user32.dll")]
    private static extern uint MapVirtualKey(uint code, uint mapType);

    public static bool Key(ushort virtualKey, bool down) {
        var input = new NativeInput {
            Type = 1,
            Keyboard = new KeyboardInput {
                VirtualKey = 0,
                ScanCode = (ushort)MapVirtualKey(virtualKey, 0),
                Flags = 0x0008u | (down ? 0u : 0x0002u),
                Time = 0,
                ExtraInfo = IntPtr.Zero
            }
        };
        return SendInput(1, new[] { input }, Marshal.SizeOf(typeof(NativeInput))) == 1;
    }
}
'@

function Get-ExactProcesses {
    param([string] $ExePath)
    $resolved = [IO.Path]::GetFullPath($ExePath)
    return @(Get-CimInstance Win32_Process -Filter "Name='LoopW.exe'" |
        Where-Object { $_.ExecutablePath -eq $resolved })
}

function Stop-ExactProcesses {
    param([string] $ExePath)
    foreach ($process in @(Get-ExactProcesses $ExePath)) {
        Stop-Process -Id ([int]$process.ProcessId) -Force -ErrorAction SilentlyContinue
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (@(Get-ExactProcesses $ExePath).Count -eq 0) { return }
        Start-Sleep -Milliseconds 50
    }
    throw "Process cleanup timed out for '$ExePath'."
}

function Wait-ResidentMutex {
    param([int] $TimeoutMs = 5000)
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt $TimeoutMs) {
        try {
            $mutex = [Threading.Mutex]::OpenExisting('Local\LoopW.Instance')
            $mutex.Dispose()
            $watch.Stop()
            return [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
        } catch [Threading.WaitHandleCannotBeOpenedException] {
            Start-Sleep -Milliseconds 2
        }
    }
    $watch.Stop()
    return $null
}

function Invoke-ProcessCommand {
    param([string] $ExePath, [string] $Command, [int] $TimeoutMs = 5000)
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $ExePath
    $info.Arguments = $Command
    $info.WorkingDirectory = Split-Path -Parent $ExePath
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $client = [Diagnostics.Process]::new()
    $client.StartInfo = $info
    [void]$client.Start()
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $ready = $client.WaitForExit($TimeoutMs)
    $watch.Stop()
    $output = if ($ready) { $client.StandardOutput.ReadToEnd() } else { '' }
    if (-not $ready) { try { $client.Kill() } catch {} }
    return [pscustomobject]@{
        ready = $ready -and $output.StartsWith('action/', [StringComparison]::Ordinal)
        elapsed_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
    }
}

function Wait-WindowByTitle {
    param([int] $ProcessId, [string] $Title, [int] $TimeoutMs = 5000)
    $window = $null
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt $TimeoutMs) {
        $window = Find-AppWindow $ProcessId $Title
        if ($null -ne $window) {
            $watch.Stop()
            return [pscustomobject]@{
                window = $window
                elapsed_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
            }
        }
        Start-Sleep -Milliseconds 2
    }
    $watch.Stop()
    return [pscustomobject]@{
        window = $null
        elapsed_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
    }
}

function Wait-ScreenHashChange {
    param(
        [LoopWWindowSnapshot] $Window,
        [ulong] $Before,
        [int] $TimeoutMs = 1000
    )
    $left = [int](Get-SnapshotValue $Window 'Left')
    $top = [int](Get-SnapshotValue $Window 'Top')
    $width = [int](Get-SnapshotValue $Window 'Width')
    $height = [int](Get-SnapshotValue $Window 'Height')
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt $TimeoutMs) {
        $hash = [LoopWPerformanceCapture]::HashScreen($left, $top, $width, $height)
        if ($hash -ne $Before) {
            $watch.Stop()
            return [pscustomobject]@{
                changed = $true
                elapsed_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
            }
        }
        Start-Sleep -Milliseconds 2
    }
    $watch.Stop()
    return [pscustomobject]@{
        changed = $false
        elapsed_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
    }
}

function Invoke-InteractionMeasurement {
    param([int] $ProcessId, [IntPtr] $TargetHandle, $Trigger)
    if (-not [LoopWNative]::Focus([IntPtr]$TargetHandle)) {
        throw 'Could not focus the disposable performance target.'
    }
    if (-not (Wait-Until { [LoopWNative]::ForegroundWindow() -eq [IntPtr]$TargetHandle } -TimeoutMs 2000 -PollMs 10)) {
        throw 'The disposable performance target did not become foreground.'
    }
    Start-Sleep -Milliseconds 50
    $center = [LoopWNative]::Cursor()
    [LoopWNative]::MoveCursor($center.X, $center.Y) | Out-Null

    $script:Trigger = $Trigger
    $TriggerHeld = $false
    $radial = $null
    $preview = $null
    try {
        $radialWatch = [Diagnostics.Stopwatch]::StartNew()
        if ($Trigger.Modifiers -ne 0) { throw 'The performance trigger probe currently requires zero modifiers.' }
        if (-not [LoopWPerformanceInput]::Key([uint16]$Trigger.VirtualKey, $true)) {
            throw 'SendInput rejected the performance trigger key-down.'
        }
        $TriggerHeld = $true
        $radial = Wait-WindowByTitle $ProcessId 'LoopW Radial'
        $radialWatch.Stop()
        if ($null -eq $radial.window) { throw 'Timed out waiting for the radial overlay.' }

        $right = [LoopWPoint]::new($center.X + 150, $center.Y)
        $previewWatch = [Diagnostics.Stopwatch]::StartNew()
        [LoopWNative]::MoveCursor($right.X, $right.Y) | Out-Null
        $preview = Wait-WindowByTitle $ProcessId 'LoopW Preview'
        $previewWatch.Stop()
        if ($null -eq $preview.window) { throw 'Timed out waiting for the preview overlay.' }
        Start-Sleep -Milliseconds 100
        $previewHash = [LoopWPerformanceCapture]::HashScreen(
            [int](Get-SnapshotValue $preview.window 'Left'),
            [int](Get-SnapshotValue $preview.window 'Top'),
            [int](Get-SnapshotValue $preview.window 'Width'),
            [int](Get-SnapshotValue $preview.window 'Height'))

        $left = [LoopWPoint]::new($center.X - 150, $center.Y)
        $crossWatch = [Diagnostics.Stopwatch]::StartNew()
        [LoopWNative]::MoveCursor($left.X, $left.Y) | Out-Null
        $cross = Wait-ScreenHashChange $preview.window $previewHash
        $crossWatch.Stop()

        return [pscustomobject]@{
            radial_open_ms = [math]::Round($radialWatch.Elapsed.TotalMilliseconds, 2)
            preview_open_ms = [math]::Round($previewWatch.Elapsed.TotalMilliseconds, 2)
            wedge_crossing_ms = if ($cross.changed) { $cross.elapsed_ms } else { $null }
            wedge_crossing_observed = $cross.changed
        }
    } finally {
        if ($null -ne $radial -and $null -ne $radial.window) {
            try { [LoopWNative]::Close([IntPtr](Get-SnapshotValue $radial.window 'Handle')) | Out-Null } catch {}
        }
        if ($TriggerHeld) { try { [LoopWPerformanceInput]::Key([uint16]$Trigger.VirtualKey, $false) | Out-Null } catch {} }
        Start-Sleep -Milliseconds 150
    }
}

function Measure-Build {
    param([string] $Name, [string] $ExePath, [IntPtr] $TargetWindowHandle, $Trigger, [int] $Count)
    $rows = @()
    for ($sample = 1; $sample -le $Count; $sample++) {
        Stop-ExactProcesses $ExePath
        try {
            $launchWatch = [Diagnostics.Stopwatch]::StartNew()
            $process = Start-Process -FilePath $ExePath -WorkingDirectory (Split-Path -Parent $ExePath) -PassThru -WindowStyle Hidden
            $mutexMs = Wait-ResidentMutex
            $ipc = Invoke-ProcessCommand $ExePath 'list/actions'
            $launchWatch.Stop()
            Start-Sleep -Milliseconds 1000
            $live = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
            if ($null -eq $live) {
                $pathProcess = @(Get-ExactProcesses $ExePath) | Select-Object -First 1
                if ($null -ne $pathProcess) { $live = Get-Process -Id ([int]$pathProcess.ProcessId) -ErrorAction SilentlyContinue }
            }
            if ($null -eq $live) { throw "Resident process disappeared during $Name sample $sample." }
            $memory = [pscustomobject]@{
                working_set_mib = [math]::Round($live.WorkingSet64 / 1MB, 2)
                private_mib = [math]::Round($live.PrivateMemorySize64 / 1MB, 2)
            }
            $interactionError = $null
            try {
                $interaction = Invoke-InteractionMeasurement $live.Id $TargetWindowHandle $Trigger
            } catch {
                $interactionError = $_.Exception.Message
                $interaction = [pscustomobject]@{
                    radial_open_ms = $null
                    preview_open_ms = $null
                    wedge_crossing_ms = $null
                    wedge_crossing_observed = $false
                }
            }
            $row = [pscustomobject]@{
                build = $Name
                sample = $sample
                mutex_ready_ms = $mutexMs
                ipc_ready_ms = $ipc.elapsed_ms
                ipc_ready = $ipc.ready
                launch_to_ipc_ms = [math]::Round($launchWatch.Elapsed.TotalMilliseconds, 2)
                working_set_mib = $memory.working_set_mib
                private_mib = $memory.private_mib
                radial_open_ms = $interaction.radial_open_ms
                preview_open_ms = $interaction.preview_open_ms
                wedge_crossing_ms = $interaction.wedge_crossing_ms
                wedge_crossing_observed = $interaction.wedge_crossing_observed
                interaction_error = $interactionError
            }
            $rows += $row
            Write-Host ($row | ConvertTo-Json -Compress)
        } finally {
            Stop-ExactProcesses $ExePath
        }
    }
    return $rows
}

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$targetTitle = 'LoopW Performance Target ' + [Guid]::NewGuid().ToString('N')
$targetScript = Join-Path $script:RepoRoot 'tools\windows-integration-target.ps1'
$targetHost = (Get-Command pwsh.exe -ErrorAction SilentlyContinue).Source
if ([string]::IsNullOrEmpty($targetHost)) { $targetHost = (Get-Command powershell.exe -ErrorAction Stop).Source }
$targetArguments = "-NoProfile -STA -ExecutionPolicy Bypass -File `"$targetScript`" -Title `"$targetTitle`""
$performanceTarget = Start-Process -FilePath $targetHost -ArgumentList $targetArguments -PassThru
$targetWindow = $null
try {
    $targetReady = Wait-Until { $targetWindow = [LoopWNative]::FindByTitle($targetTitle); return ($null -ne $targetWindow) } -TimeoutMs 10000 -PollMs 20
    if (-not $targetReady) { throw 'Timed out waiting for the disposable performance target.' }
    $targetWindow = [LoopWNative]::FindByTitle($targetTitle)
    $targetHandle = [IntPtr](Get-SnapshotValue $targetWindow 'Handle')
    $trigger = Get-TriggerConfig
    if (-not $trigger.RadialEnabled -or -not $trigger.PreviewEnabled) {
        throw 'RadialEnabled and PreviewEnabled must both be true for interaction metrics.'
    }

    $rows = @()
    if ($Target -in @('rust', 'both')) {
        $rows += Measure-Build 'rust-release' ([IO.Path]::GetFullPath($RustExe)) $targetHandle $trigger $Samples
    }
    if ($Target -in @('wpf', 'both')) {
        if ([string]::IsNullOrWhiteSpace($WpfExe)) {
            throw 'Pass -WpfExe when Target is wpf or both.'
        }
        $rows += Measure-Build 'wpf-release' ([IO.Path]::GetFullPath($WpfExe)) $targetHandle $trigger $Samples
    }
    $rows | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $EvidenceDir 'performance-samples.json')
    [pscustomobject]@{
        ok = $true
        target = $Target
        samples = $Samples
        evidence_dir = $EvidenceDir
        sample_file = (Join-Path $EvidenceDir 'performance-samples.json')
        rows = $rows
    } | ConvertTo-Json -Depth 6
} finally {
    if ($null -ne $performanceTarget -and -not $performanceTarget.HasExited) {
        Stop-Process -Id $performanceTarget.Id -Force -ErrorAction SilentlyContinue
    }
}
