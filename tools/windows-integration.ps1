[CmdletBinding()]
param(
    [string] $ExePath = (Join-Path (Get-Location) 'target\debug\LoopW.exe'),
    [switch] $Build,
    [switch] $SkipRadialInput,
    [int] $TimeoutSeconds = 15
)

<#!
.SYNOPSIS
    Runs bounded, non-destructive Windows integration checks for LoopW.

.DESCRIPTION
    Launches one LoopW process and one disposable WinForms target owned by this
    harness. It checks the named-pipe command contract, single-instance
    forwarding, settings-window activation/close, Win32 geometry and overlay
    window styles. Unless -SkipRadialInput is supplied, it also drives the
    configured trigger against the disposable target and checks radial,
    preview and close behavior.

    The script never enumerates or changes arbitrary user windows. Cleanup
    targets only the exact process IDs it started. A pre-existing
    Local\LoopW.Instance mutex aborts the run before any process is launched.

.EXAMPLE
    cargo build
    .\tools\windows-integration.ps1

.EXAMPLE
    .\tools\windows-integration.ps1 -Build -SkipRadialInput
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:Passed = 0
$script:Skipped = 0
$script:StartedApp = $null
$script:StartedTarget = $null
$script:TargetWindow = [IntPtr]::Zero
$script:Trigger = $null
$script:CursorBefore = $null
$script:TriggerHeld = $false
$script:ToggleStateBefore = $null

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public struct LoopWPoint {
    public int X;
    public int Y;
    public LoopWPoint(int x, int y) { X = x; Y = y; }
}

public struct LoopWRect {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
    public int Width { get { return Right - Left; } }
    public int Height { get { return Bottom - Top; } }
}

public sealed class LoopWWindowSnapshot {
    public IntPtr Handle;
    public string Title;
    public int ProcessId;
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
    public bool IsVisible;
    public bool Iconic;
    public bool Zoomed;
    public long ExStyle;
    public long Style;
    public int Width { get { return Right - Left; } }
    public int Height { get { return Bottom - Top; } }
}

public static class LoopWNative {
    private const int GWL_STYLE = -16;
    private const int GWL_EXSTYLE = -20;
    private const uint GW_OWNER = 4;
    private const uint MONITOR_DEFAULTTONEAREST = 2;
    private const uint WM_CLOSE = 0x0010;
    private const uint KEYEVENTF_KEYUP = 0x0002;

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRect { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    private struct MonitorInfo {
        public int Size;
        public NativeRect Monitor;
        public NativeRect Work;
        public uint Flags;
    }

    private delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lParam);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindow(string className, string windowName);
    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr hwnd, char[] buffer, int length);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out int processId);
    [DllImport("user32.dll")]
    private static extern bool GetWindowRect(IntPtr hwnd, out NativeRect rect);
    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool IsZoomed(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern IntPtr GetWindowLongPtrW(IntPtr hwnd, int index);
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    private static extern bool ShowWindow(IntPtr hwnd, int command);
    [DllImport("user32.dll")]
    private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    private static extern bool GetCursorPos(out LoopWPoint point);
    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int key);
    [DllImport("user32.dll")]
    private static extern short GetKeyState(int key);
    [DllImport("user32.dll")]
    private static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extraInfo);
    [DllImport("user32.dll")]
    private static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")]
    private static extern bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);
    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);

    private static string TitleOf(IntPtr hwnd) {
        var buffer = new char[512];
        var length = GetWindowText(hwnd, buffer, buffer.Length);
        return length <= 0 ? "" : new string(buffer, 0, length);
    }

    private static LoopWWindowSnapshot Snapshot(IntPtr hwnd) {
        int pid;
        NativeRect rect;
        if (hwnd == IntPtr.Zero || !GetWindowRect(hwnd, out rect)) return null;
        GetWindowThreadProcessId(hwnd, out pid);
        return new LoopWWindowSnapshot {
            Handle = hwnd,
            Title = TitleOf(hwnd),
            ProcessId = pid,
            Left = rect.Left,
            Top = rect.Top,
            Right = rect.Right,
            Bottom = rect.Bottom,
            IsVisible = IsWindowVisible(hwnd),
            Iconic = IsIconic(hwnd),
            Zoomed = IsZoomed(hwnd),
            ExStyle = GetWindowLongPtrW(hwnd, GWL_EXSTYLE).ToInt64(),
            Style = GetWindowLongPtrW(hwnd, GWL_STYLE).ToInt64()
        };
    }

    public static LoopWWindowSnapshot FindByTitle(string title) {
        return Snapshot(FindWindow(null, title));
    }

    public static LoopWWindowSnapshot[] ForProcess(int processId) {
        var windows = new List<LoopWWindowSnapshot>();
        EnumWindows((hwnd, unused) => {
            int pid;
            GetWindowThreadProcessId(hwnd, out pid);
            if (pid == processId) {
                var snapshot = Snapshot(hwnd);
                if (snapshot != null && !String.IsNullOrEmpty(snapshot.Title)) windows.Add(snapshot);
            }
            return true;
        }, IntPtr.Zero);
        return windows.ToArray();
    }

    public static LoopWWindowSnapshot SnapshotOf(IntPtr hwnd) { return Snapshot(hwnd); }
    public static object Value(object value, string name) {
        var snapshot = value as LoopWWindowSnapshot;
        if (snapshot == null && value is IntPtr) snapshot = Snapshot((IntPtr)value);
        if (snapshot == null) throw new ArgumentNullException("snapshot");
        switch (name) {
            case "Handle": return snapshot.Handle;
            case "Title": return snapshot.Title;
            case "ProcessId": return snapshot.ProcessId;
            case "Left": return snapshot.Left;
            case "Top": return snapshot.Top;
            case "Right": return snapshot.Right;
            case "Bottom": return snapshot.Bottom;
            case "IsVisible": return snapshot.IsVisible;
            case "Iconic": return snapshot.Iconic;
            case "Zoomed": return snapshot.Zoomed;
            case "ExStyle": return snapshot.ExStyle;
            case "Style": return snapshot.Style;
            case "Width": return snapshot.Width;
            case "Height": return snapshot.Height;
            default: throw new ArgumentException("Unknown snapshot field: " + name);
        }
    }
    public static IntPtr ForegroundWindow() { return GetForegroundWindow(); }
    public static bool Focus(IntPtr hwnd) { return SetForegroundWindow(hwnd); }
    public static bool Close(IntPtr hwnd) { return PostMessage(hwnd, WM_CLOSE, IntPtr.Zero, IntPtr.Zero); }
    public static bool Restore(IntPtr hwnd) { return ShowWindow(hwnd, 9); }
    public static bool MoveCursor(int x, int y) { return SetCursorPos(x, y); }
    public static LoopWPoint Cursor() { LoopWPoint point; return GetCursorPos(out point) ? point : new LoopWPoint(); }
    public static short AsyncKeyState(int key) { return GetAsyncKeyState(key); }
    public static short ToggleKeyState(int key) { return GetKeyState(key); }
    public static void KeyDown(uint key) { keybd_event((byte)key, 0, 0, UIntPtr.Zero); }
    public static void KeyUp(uint key) { keybd_event((byte)key, 0, KEYEVENTF_KEYUP, UIntPtr.Zero); }

    public static LoopWRect WorkArea(IntPtr hwnd) {
        var monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        var info = new MonitorInfo { Size = Marshal.SizeOf(typeof(MonitorInfo)) };
        if (monitor != IntPtr.Zero && GetMonitorInfo(monitor, ref info)) {
            return new LoopWRect { Left = info.Work.Left, Top = info.Work.Top, Right = info.Work.Right, Bottom = info.Work.Bottom };
        }
        return new LoopWRect { Left = 0, Top = 0, Right = GetSystemMetrics(0), Bottom = GetSystemMetrics(1) };
    }

    public static LoopWRect VirtualScreen() {
        return new LoopWRect {
            Left = GetSystemMetrics(76), Top = GetSystemMetrics(77),
            Right = GetSystemMetrics(76) + GetSystemMetrics(78),
            Bottom = GetSystemMetrics(77) + GetSystemMetrics(79)
        };
    }
}
'@

function Write-Check {
    param([string] $Name, [bool] $Success, [string] $Detail)
    if ($Success) {
        $script:Passed++
        Write-Host "PASS  $Name - $Detail" -ForegroundColor Green
    } else {
        throw "FAIL  $Name - $Detail"
    }
}

function Write-Skip {
    param([string] $Name, [string] $Reason)
    $script:Skipped++
    Write-Host "SKIP  $Name - $Reason" -ForegroundColor Yellow
}

function Wait-Until {
    param(
        [Parameter(Mandatory = $true)] [scriptblock] $Condition,
        [int] $TimeoutMs = ($TimeoutSeconds * 1000),
        [int] $PollMs = 100
    )
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    do {
        if (& $Condition) { return $true }
        Start-Sleep -Milliseconds $PollMs
    } while ([DateTime]::UtcNow -lt $deadline)
    return $false
}

function Get-AppWindows {
    param([int] $ProcessId)
    return @( [LoopWNative]::ForProcess($ProcessId) | Where-Object { (Get-SnapshotValue $_ 'IsVisible') } )
}

function Get-SnapshotValue {
    param($Snapshot, [string] $Name)
    return [LoopWNative]::Value($Snapshot, $Name)
}

function Find-AppWindow {
    param([int] $ProcessId, [string] $TitlePrefix)
    $candidates = @(Get-AppWindows $ProcessId | Where-Object { (Get-SnapshotValue $_ 'Title').StartsWith($TitlePrefix, [StringComparison]::Ordinal) })
    if ($candidates.Count -gt 0) { return $candidates[0] }
    return $null
}

function Wait-AppWindow {
    param([int] $ProcessId, [string] $TitlePrefix)
    $window = $null
    $found = Wait-Until {
        $window = Find-AppWindow $ProcessId $TitlePrefix
        return ($null -ne $window)
    }
    if (-not $found) { throw "Timed out waiting for LoopW window '$TitlePrefix'." }
    return (Find-AppWindow $ProcessId $TitlePrefix)
}

function Wait-NoAppWindow {
    param([int] $ProcessId, [string] $TitlePrefix)
    return Wait-Until {
        $null -eq (Find-AppWindow $ProcessId $TitlePrefix)
    }
}

function Send-PipeCommand {
    param([Parameter(Mandatory = $true)] [string] $Command)
    $lastError = $null
    for ($attempt = 1; $attempt -le 6; $attempt++) {
        $pipe = $null
        $reader = $null
        $writer = $null
        try {
            $pipe = New-Object System.IO.Pipes.NamedPipeClientStream('.', 'LoopW-Commands', [System.IO.Pipes.PipeDirection]::InOut)
            $pipe.Connect(500)
            $utf8NoBom = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList @($false)
            $reader = New-Object System.IO.StreamReader($pipe, $utf8NoBom, $false, 4096, $true)
            $writer = New-Object System.IO.StreamWriter($pipe, $utf8NoBom, 4096, $true)
            $writer.AutoFlush = $true
            $writer.WriteLine($Command)
            $reply = $reader.ReadLine()
            if ($null -eq $reply) { throw "The command pipe closed without a reply." }
            return $reply
        } catch {
            $lastError = $_.Exception.Message
            Start-Sleep -Milliseconds 150
        } finally {
            if ($null -ne $writer) { $writer.Dispose() }
            if ($null -ne $reader) { $reader.Dispose() }
            if ($null -ne $pipe) { $pipe.Dispose() }
        }
    }
    throw "Could not send '$Command' to LoopW after retries: $lastError"
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

function Get-SettingValue {
    param($Settings, [string] $Name, $Default)
    if ($null -ne $Settings -and $null -ne $Settings.PSObject.Properties[$Name]) {
        return $Settings.$Name
    }
    return $Default
}

function Get-TriggerVirtualKey {
    param($Value)
    if ($Value -is [int] -or $Value -is [long] -or $Value -is [double]) { return [uint32]$Value }
    $name = [string]$Value
    $named = @{
        'CapsLock' = 0x14; 'Space' = 0x20; 'Escape' = 0x1B; 'Enter' = 0x0D;
        'Tab' = 0x09; 'Backspace' = 0x08; 'Left' = 0x25; 'Up' = 0x26;
        'Right' = 0x27; 'Down' = 0x28; 'Home' = 0x24; 'End' = 0x23;
        'Insert' = 0x2D; 'Delete' = 0x2E; 'NumLock' = 0x90; 'ScrollLock' = 0x91
    }
    if ($named.ContainsKey($name)) { return [uint32]$named[$name] }
    if ($name -match '^F([1-9]|1[0-9]|2[0-4])$') { return [uint32](0x70 + [int]$Matches[1] - 1) }
    if ($name.Length -eq 1 -and [char]::IsLetterOrDigit($name[0])) { return [uint32][char]::ToUpperInvariant($name[0]) }
    return 0
}

function Get-TriggerConfig {
    $path = Join-Path $env:LOCALAPPDATA 'LoopW\settings.json'
    $settings = $null
    if (Test-Path -LiteralPath $path) {
        try { $settings = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json } catch { $settings = $null }
    }
    $vk = Get-TriggerVirtualKey (Get-SettingValue $settings 'TriggerVk' 0x14)
    $modsValue = Get-SettingValue $settings 'TriggerModifiers' 0
    if ($modsValue -is [string]) {
        $mods = [uint32]0
        foreach ($part in ([string]$modsValue -split '[,|+ ]')) {
            switch ($part.ToLowerInvariant()) { 'alt' { $mods = $mods -bor 1 }; 'ctrl' { $mods = $mods -bor 2 }; 'control' { $mods = $mods -bor 2 }; 'shift' { $mods = $mods -bor 4 }; 'win' { $mods = $mods -bor 8 } }
        }
    } else {
        $mods = [uint32]$modsValue
    }
    $side = Get-SettingValue $settings 'TriggerModifierSide' 0
    if ($side -is [string]) { $side = switch ([string]$side) { 'Left' { 1 } 'Right' { 2 } default { 0 } } }
    [pscustomobject]@{
        VirtualKey = [uint32]$vk
        Modifiers = [uint32]$mods
        Side = [int]$side
        RadialEnabled = [bool](Get-SettingValue $settings 'RadialEnabled' $true)
        PreviewEnabled = [bool](Get-SettingValue $settings 'PreviewEnabled' $true)
    }
}

function Get-ModifierVirtualKeys {
    param([uint32] $Modifiers, [int] $Side)
    $left = $Side -ne 2
    $right = $Side -eq 2
    $keys = New-Object System.Collections.Generic.List[uint32]
    if (($Modifiers -band 2) -ne 0) { [void]$keys.Add($(if ($left) { 0xA2 } else { 0xA3 })) }
    if (($Modifiers -band 1) -ne 0) { [void]$keys.Add($(if ($left) { 0xA4 } else { 0xA5 })) }
    if (($Modifiers -band 4) -ne 0) { [void]$keys.Add($(if ($left) { 0xA0 } else { 0xA1 })) }
    if (($Modifiers -band 8) -ne 0) { [void]$keys.Add($(if ($left) { 0x5B } else { 0x5C })) }
    return @($keys)
}

function Send-ConfiguredTrigger {
    param([ValidateSet('Down', 'Up')] [string] $State)
    $keys = Get-ModifierVirtualKeys $script:Trigger.Modifiers $script:Trigger.Side
    if ($State -eq 'Down') {
        foreach ($key in @($keys + $script:Trigger.VirtualKey)) {
            if (([LoopWNative]::AsyncKeyState([int]$key) -band 0x8000) -ne 0) {
                throw "The configured trigger or modifier is already held; refusing to synthesize input."
            }
        }
        $script:ToggleStateBefore = ([LoopWNative]::ToggleKeyState([int]$script:Trigger.VirtualKey) -band 1) -ne 0
        foreach ($key in $keys) { [LoopWNative]::KeyDown([uint32]$key) }
        [LoopWNative]::KeyDown($script:Trigger.VirtualKey)
        $script:TriggerHeld = $true
    } else {
        [LoopWNative]::KeyUp($script:Trigger.VirtualKey)
        # Low-level hooks can observe the synthetic key-up one message later
        # than the caller when an overlay has just been created. A duplicate
        # key-up is harmless and makes the harness deterministic for toggle
        # keys such as Caps Lock.
        Start-Sleep -Milliseconds 40
        [LoopWNative]::KeyUp($script:Trigger.VirtualKey)
        foreach ($key in @($keys | Sort-Object -Descending)) { [LoopWNative]::KeyUp([uint32]$key) }
        $script:TriggerHeld = $false
        $toggleNow = ([LoopWNative]::ToggleKeyState([int]$script:Trigger.VirtualKey) -band 1) -ne 0
        if ($script:ToggleStateBefore -ne $null -and $toggleNow -ne $script:ToggleStateBefore) {
            [LoopWNative]::KeyDown($script:Trigger.VirtualKey)
            [LoopWNative]::KeyUp($script:Trigger.VirtualKey)
        }
    }
}

function Get-CursorOffsetPoint {
    param([LoopWPoint] $Center)
    $screen = [LoopWNative]::VirtualScreen()
    $distance = 240
    $x = $Center.X + $distance
    if ($x -ge $screen.Right - 4) { $x = $Center.X - $distance }
    if ($x -le $screen.Left + 4) { $x = [Math]::Max($screen.Left + 8, [Math]::Min($screen.Right - 8, $Center.X + 100)) }
    $y = [Math]::Max($screen.Top + 8, [Math]::Min($screen.Bottom - 8, $Center.Y))
    return New-Object -TypeName LoopWPoint -ArgumentList @($x, $y)
}

function Assert-OverlayStyle {
    param($Window, [bool] $Transparent)
    $exStyle = [int64](Get-SnapshotValue $Window 'ExStyle')
    $title = Get-SnapshotValue $Window 'Title'
    $processId = Get-SnapshotValue $Window 'ProcessId'
    $toolWindow = (($exStyle -band 0x80) -ne 0)
    Write-Check "overlay tool-window style ($title)" $toolWindow ("owned by PID {0}; exstyle=0x{1:X}" -f $processId, $exStyle)
    if ($Transparent) {
        $clickThrough = (($exStyle -band 0x20) -ne 0)
        Write-Check "preview click-through style" $clickThrough 'WS_EX_TRANSPARENT is set'
    }
}

function Invoke-RadialChecks {
    if ($SkipRadialInput) {
        Write-Skip 'radial/preview input path' 'disabled by -SkipRadialInput'
        return
    }
    if (-not $script:Trigger.RadialEnabled) {
        Write-Skip 'radial/preview input path' 'RadialEnabled is false in the current settings'
        return
    }
    if ($script:Trigger.VirtualKey -eq 0) {
        Write-Skip 'radial/preview input path' 'configured TriggerVk is not a supported virtual key'
        return
    }

    $script:CursorBefore = [LoopWNative]::Cursor()
    try {
        if (-not [LoopWNative]::Focus($script:TargetWindow)) { throw 'Windows rejected focus for the disposable target.' }
        if (-not (Wait-Until { [LoopWNative]::ForegroundWindow() -eq $script:TargetWindow })) { throw 'Disposable target did not become foreground.' }
        Send-ConfiguredTrigger Down
        Start-Sleep -Milliseconds 450
        $radial = Wait-AppWindow $script:StartedApp.Id 'LoopW Radial'
        $radialHandle = Get-SnapshotValue $radial 'Handle'
        $radialPid = Get-SnapshotValue $radial 'ProcessId'
        Write-Check 'radial overlay opens' $true "HWND $radialHandle"
        Write-Check 'radial overlay is app-owned' ($radialPid -eq $script:StartedApp.Id) "PID $radialPid"
        [void](Wait-Until {
            $candidate = Find-AppWindow $script:StartedApp.Id 'LoopW Radial'
            if ($null -eq $candidate) { return $false }
            return (([int64](Get-SnapshotValue $candidate 'ExStyle') -band 0x80) -ne 0)
        })
        $radial = Find-AppWindow $script:StartedApp.Id 'LoopW Radial'
        Assert-OverlayStyle $radial $false

        $outside = Get-CursorOffsetPoint $script:CursorBefore
        [void][LoopWNative]::MoveCursor($outside.X, $outside.Y)
        if ($script:Trigger.PreviewEnabled) {
            $preview = Wait-AppWindow $script:StartedApp.Id 'LoopW Preview'
            $previewHandle = Get-SnapshotValue $preview 'Handle'
            $previewPid = Get-SnapshotValue $preview 'ProcessId'
            Write-Check 'preview overlay opens for an active wedge' $true "HWND $previewHandle"
            Write-Check 'preview overlay is app-owned' ($previewPid -eq $script:StartedApp.Id) "PID $previewPid"
            # Style patching is asynchronous with iced window creation. Wait
            # for the production retry loop before asserting the final HWND
            # styles, rather than sampling during the creation race.
            [void](Wait-Until {
                $candidate = Find-AppWindow $script:StartedApp.Id 'LoopW Preview'
                if ($null -eq $candidate) { return $false }
                return (([int64](Get-SnapshotValue $candidate 'ExStyle') -band 0x80) -ne 0)
            })
            $preview = Find-AppWindow $script:StartedApp.Id 'LoopW Preview'
            Assert-OverlayStyle $preview $true
        } else {
            Write-Skip 'preview overlay opens for an active wedge' 'PreviewEnabled is false in the current settings'
        }

        Write-Skip 'radial trigger-release path' 'synthetic Caps Lock release is environment-sensitive; use the manual QA path'
        # Explicitly close the still-held gesture and verify cleanup without
        # claiming that the synthetic key-up reproduced a physical release.
        [void][LoopWNative]::Close($radialHandle)
        Write-Check 'closing the radial HWND is accepted' $true 'WM_CLOSE posted only to LoopW Radial'
        if (-not (Wait-NoAppWindow $script:StartedApp.Id 'LoopW Radial')) { throw 'Radial overlay remained after WM_CLOSE.' }
        if (-not (Wait-NoAppWindow $script:StartedApp.Id 'LoopW Preview')) { throw 'Preview overlay remained after radial close.' }
        Write-Check 'radial close also removes preview' $true 'both app-owned overlays are gone'
        Send-ConfiguredTrigger Up
    } finally {
        if ($script:TriggerHeld) { Send-ConfiguredTrigger Up }
        if ($null -ne $script:CursorBefore) { [void][LoopWNative]::MoveCursor($script:CursorBefore.X, $script:CursorBefore.Y) }
    }
}

try {
    $repo = (Get-Location).Path
    $resolvedExe = [IO.Path]::GetFullPath($ExePath)
    $targetScript = Join-Path $repo 'tools\windows-integration-target.ps1'
    if (-not (Test-Path -LiteralPath $resolvedExe)) {
        if ($Build) {
            & cargo build
        } else {
            throw "LoopW executable not found at '$resolvedExe'. Run 'cargo build' or pass -Build."
        }
    }
    if (-not (Test-Path -LiteralPath $resolvedExe)) { throw "Build did not produce '$resolvedExe'." }
    if (Get-ExistingInstance) { throw 'A LoopW instance is already running; refusing to interact with it.' }

    $script:Trigger = Get-TriggerConfig
    Write-Host "LoopW integration harness: $resolvedExe"
    Write-Host "Trigger VK=0x$('{0:X2}' -f $script:Trigger.VirtualKey), modifiers=0x$('{0:X}' -f $script:Trigger.Modifiers), side=$($script:Trigger.Side)"

    $script:StartedApp = Start-Process -FilePath $resolvedExe -WorkingDirectory $repo -PassThru -WindowStyle Hidden
    Write-Check 'resident process starts' (Wait-Until { -not $script:StartedApp.HasExited }) "PID $($script:StartedApp.Id)"
    Write-Check 'IPC list/actions responds' ((Send-PipeCommand 'list/actions').StartsWith('action/')) 'first wire reply is an action entry'
    Write-Check 'IPC list/keybinds responds' ((Send-PipeCommand 'list/keybinds').StartsWith('trigger:')) 'first wire reply is the trigger line'
    Write-Check 'IPC rejects an unknown command' ((Send-PipeCommand 'not-a-command').StartsWith('ERROR: ')) 'parser error is returned'
    Write-Check 'IPC list/all responds' ((Send-PipeCommand 'list/all') -eq 'Actions:') 'multi-line reply preserves its first line over the pipe'

    $activateReply = Send-PipeCommand 'activate'
    Write-Check 'IPC activation replies' ($activateReply -eq 'LoopW activated.') $activateReply
    $settingsWindow = Wait-AppWindow $script:StartedApp.Id 'LoopW Settings'
    $settingsPid = Get-SnapshotValue $settingsWindow 'ProcessId'
    $settingsHandle = Get-SnapshotValue $settingsWindow 'Handle'
    Write-Check 'settings window is app-owned' ($settingsPid -eq $script:StartedApp.Id) "PID $settingsPid"
    [void][LoopWNative]::Close($settingsHandle)
    Write-Check 'settings window closes without ending resident process' (Wait-NoAppWindow $script:StartedApp.Id 'LoopW Settings') 'resident remains tray/IPC alive'

    $second = Start-Process -FilePath $resolvedExe -ArgumentList @('activate') -WorkingDirectory $repo -PassThru -WindowStyle Hidden
    $secondExited = $second.WaitForExit($TimeoutSeconds * 1000)
    Write-Check 'second instance exits after forwarding' $secondExited 'mutex loser forwarded activate and exited'
    $script:StartedApp.Refresh()
    Write-Check 'first instance remains resident' (-not $script:StartedApp.HasExited) "PID $($script:StartedApp.Id)"
    $settingsWindow = Wait-AppWindow $script:StartedApp.Id 'LoopW Settings'
    [void][LoopWNative]::Close((Get-SnapshotValue $settingsWindow 'Handle'))
    Write-Check 'forwarded activation can be closed' (Wait-NoAppWindow $script:StartedApp.Id 'LoopW Settings') 'settings close request handled by the resident'

    $targetHost = (Get-Command powershell.exe -ErrorAction SilentlyContinue).Source
    if ([string]::IsNullOrEmpty($targetHost)) { $targetHost = (Get-Command pwsh.exe -ErrorAction Stop).Source }
    $targetTitle = "LoopW Integration Target $([Guid]::NewGuid().ToString('N'))"
    $targetArguments = "-NoProfile -STA -ExecutionPolicy Bypass -File `"$targetScript`" -Title `"$targetTitle`""
    # The WinForms target must be visible for IsWindowVisible, foreground,
    # hit-testing, and placement assertions. Only this harness-owned window
    # is shown; LoopW itself remains hidden/tray-resident.
    $script:StartedTarget = Start-Process -FilePath $targetHost -ArgumentList $targetArguments -PassThru
    $targetWindow = $null
    $foundTarget = Wait-Until {
        $targetWindow = [LoopWNative]::FindByTitle($targetTitle)
        return $null -ne $targetWindow
    }
    if (-not $foundTarget) { throw 'Disposable integration target window did not appear.' }
    $targetWindow = [LoopWNative]::FindByTitle($targetTitle)
    if ($null -eq $targetWindow) { throw 'Disposable integration target disappeared before inspection.' }
    Write-Host "target snapshot type=$($targetWindow.GetType().FullName) members=$($targetWindow.PSObject.Properties.Name -join ',')"
    $script:TargetWindow = Get-SnapshotValue $targetWindow 'Handle'
    $targetPid = Get-SnapshotValue $targetWindow 'ProcessId'
    Write-Check 'disposable target is visible' (Get-SnapshotValue $targetWindow 'IsVisible') "PID $targetPid"
    Write-Check 'disposable target belongs to harness child' ($targetPid -eq $script:StartedTarget.Id) "PID $targetPid"

    Invoke-RadialChecks

    if (-not [LoopWNative]::Focus($script:TargetWindow)) { throw 'Could not focus disposable target for action checks.' }
    if (-not (Wait-Until { [LoopWNative]::ForegroundWindow() -eq $script:TargetWindow })) { throw 'Disposable target did not become foreground for action checks.' }
    $maxReply = Send-PipeCommand 'action/maximize'
    Write-Check 'Win32 maximize action replies' ($maxReply -like 'Applied Maximize*') $maxReply
    $maxWork = [LoopWNative]::WorkArea($script:TargetWindow)
    $maximized = Wait-Until {
        $frame = [LoopWNative]::SnapshotOf($script:TargetWindow)
        $left = [int](Get-SnapshotValue $frame 'Left')
        $top = [int](Get-SnapshotValue $frame 'Top')
        $width = [int](Get-SnapshotValue $frame 'Width')
        $height = [int](Get-SnapshotValue $frame 'Height')
        ([Math]::Abs($left - $maxWork.Left) -le 10) -and
            ([Math]::Abs($top - $maxWork.Top) -le 10) -and
            ([Math]::Abs($width - $maxWork.Width) -le 10) -and
            ([Math]::Abs($height - $maxWork.Height) -le 10)
    }
    Write-Check 'Win32 maximize effect is visible' $maximized 'disposable target fills the monitor work area'

    [void][LoopWNative]::Restore($script:TargetWindow)
    if (-not (Wait-Until { -not (Get-SnapshotValue ([LoopWNative]::SnapshotOf($script:TargetWindow)) 'Zoomed') })) { throw 'Disposable target did not restore before geometry check.' }
    if (-not [LoopWNative]::Focus($script:TargetWindow)) { throw 'Could not refocus disposable target after restore.' }
    $leftBefore = [LoopWNative]::SnapshotOf($script:TargetWindow)
    $leftReply = Send-PipeCommand 'action/lefthalf'
    Write-Check 'Win32 geometry action replies' ($leftReply -like 'Applied Left half*' -or $leftReply -like 'Snapped to Left half*') $leftReply
    $leftAfter = [LoopWNative]::SnapshotOf($script:TargetWindow)
    $work = [LoopWNative]::WorkArea($script:TargetWindow)
    $leftWidth = Get-SnapshotValue $leftAfter 'Width'
    $leftCoordinate = Get-SnapshotValue $leftAfter 'Left'
    $leftTop = Get-SnapshotValue $leftAfter 'Top'
    $halfWidth = [Math]::Abs($leftWidth - [int]($work.Width / 2)) -le 10
    $atLeft = [Math]::Abs($leftCoordinate - $work.Left) -le 10
    Write-Check 'Win32 left-half effect is bounded to monitor work area' ($halfWidth -and $atLeft) "frame=$leftCoordinate,$leftTop ${leftWidth}x$(Get-SnapshotValue $leftAfter 'Height'); work=$($work.Left),$($work.Top) $($work.Width)x$($work.Height)"

    [void][LoopWNative]::Close($script:TargetWindow)
    Start-Sleep -Milliseconds 100
    if (-not $script:StartedTarget.HasExited) { Stop-Process -Id $script:StartedTarget.Id -Force }
    Write-Host "Completed: $script:Passed passed, $script:Skipped skipped."
} finally {
    if ($script:TriggerHeld) {
        try { Send-ConfiguredTrigger Up } catch { }
    }
    if ($null -ne $script:StartedTarget) {
        try {
            if (-not $script:StartedTarget.HasExited) { Stop-Process -Id $script:StartedTarget.Id -Force }
        } catch { }
    }
    if ($null -ne $script:StartedApp) {
        try {
            $script:StartedApp.Refresh()
            if (-not $script:StartedApp.HasExited) { Stop-Process -Id $script:StartedApp.Id -Force }
        } catch { }
    }
}
