using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;

namespace LoopW;

internal static class WindowStashService
{
    private const int DefaultEdgeHitZone = 14;

    private readonly record struct StashedWindow(
        NativeMethods.WindowPlacement OriginalPlacement,
        StashMonitor OriginalMonitor,
        NativeMethods.Rect StashedFrame,
        StashEdge Edge,
        WindowIdentity Identity,
        string PersistedId);

    private static readonly Dictionary<IntPtr, StashedWindow> Stashed = new();
    private static readonly List<IntPtr> Order = new();
    private static AppSettings _settings = new();
    private static IntPtr _pendingRevealWindow;
    private static long _pendingRevealStartedAt;

    public static void Configure(AppSettings settings)
    {
        _settings = settings;
        ResetPendingReveal();

        if (!_settings.StashPersistenceEnabled)
        {
            _settings.StashRecords.Clear();
            return;
        }

        RestorePersistedEntries();
    }

    public static void UpdateSettings(AppSettings settings)
    {
        _settings = settings;
        ResetPendingReveal();

        if (!_settings.StashPersistenceEnabled && _settings.StashRecords.Count > 0)
        {
            _settings.StashRecords.Clear();
            _settings.Save();
        }
    }

    public static void Poll()
    {
        PruneStaleEntries();
    }

    public static bool TryStash(IntPtr window, out string message)
    {
        message = string.Empty;
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            message = "The target window is no longer available.";
            return false;
        }

        if (Stashed.ContainsKey(window))
        {
            message = "The target window is already stashed.";
            return false;
        }

        if (!TryReadWindowIdentity(window, out var identity))
        {
            message = "Could not identify the target window.";
            return false;
        }

        var placement = new NativeMethods.WindowPlacement
        {
            Length = Marshal.SizeOf<NativeMethods.WindowPlacement>()
        };
        if (!NativeMethods.GetWindowPlacement(window, ref placement) ||
            !NativeMethods.GetWindowRect(window, out var current))
        {
            message = "Could not read the target window's placement.";
            return false;
        }

        NativeMethods.ShowWindow(window, NativeMethods.SwRestore);
        if (!NativeMethods.GetWindowRect(window, out current) ||
            !TryGetMonitorSnapshot(current, out var monitor))
        {
            RestoreOriginalPlacement(window, placement);
            message = "Could not determine the target monitor.";
            return false;
        }

        var edge = NearestEdge(monitor.Work.ToNative(), current);
        var stashedFrame = CalculateStashedFrame(
            monitor.Work.ToNative(),
            current,
            edge,
            EdgePeek);
        if (!NativeMethods.SetWindowPos(
                window,
                IntPtr.Zero,
                stashedFrame.Left,
                stashedFrame.Top,
                stashedFrame.Width,
                stashedFrame.Height,
                NativeMethods.SwpNoActivate | NativeMethods.SwpNoZOrder | NativeMethods.SwpAsyncWindowPos))
        {
            RestoreOriginalPlacement(window, placement);
            message = "Windows rejected the stash. The target may be elevated or non-movable.";
            return false;
        }

        var persistedId = CanPersist(identity)
            ? Guid.NewGuid().ToString("N")
            : string.Empty;
        var stashed = new StashedWindow(
            placement,
            monitor,
            stashedFrame,
            edge,
            identity,
            persistedId);
        Stashed[window] = stashed;
        Order.Remove(window);
        Order.Add(window);
        Persist(stashed);
        message = $"Stashed the window at the {edge.ToString().ToLowerInvariant()} edge";
        return true;
    }

    public static bool TryRevealNext(out string message)
    {
        PruneStaleEntries();
        for (var i = 0; i < Order.Count; i++)
        {
            var window = Order[i];
            if (!Stashed.ContainsKey(window) || !IsStashedWindowAlive(window, Stashed[window]))
            {
                RemoveRuntime(window, removePersisted: true);
                i--;
                continue;
            }

            ResetPendingReveal();
            return TryReveal(window, out message);
        }

        message = "No stashed windows to reveal.";
        return false;
    }

    internal static void RestoreAll()
    {
        var changed = false;
        foreach (var window in Order.ToArray())
        {
            if (!Stashed.TryGetValue(window, out var stashed))
            {
                continue;
            }

            if (IsStashedWindowAlive(window, stashed))
            {
                TryRestoreOriginalPlacement(window, stashed);
            }
            else
            {
                changed |= RemoveRuntime(window, removePersisted: true);
            }
        }

        Stashed.Clear();
        Order.Clear();
        ResetPendingReveal();
        if (!_settings.StashPersistenceEnabled)
        {
            _settings.StashRecords.Clear();
            changed = true;
        }

        if (changed)
        {
            SavePersistence();
        }
    }

    public static bool TryRevealAtCursor(NativeMethods.Point cursor, out string message)
    {
        PruneStaleEntries();
        for (var i = 0; i < Order.Count; i++)
        {
            var window = Order[i];
            if (!Stashed.TryGetValue(window, out var stashed) ||
                !IsStashedWindowAlive(window, stashed))
            {
                RemoveRuntime(window, removePersisted: true);
                i--;
                continue;
            }

            if (!TryGetMonitorSnapshot(stashed.StashedFrame, out var monitor) ||
                !IsInHitZone(cursor, monitor.Work.ToNative(), stashed.Edge, HitZone))
            {
                continue;
            }

            if (!ReadyToReveal(window))
            {
                message = string.Empty;
                return false;
            }

            ResetPendingReveal();
            return TryReveal(window, out message);
        }

        ResetPendingReveal();
        message = string.Empty;
        return false;
    }

    internal static StashEdge NearestEdge(NativeMethods.Rect work, NativeMethods.Rect window)
    {
        var distances = new[]
        {
            (Edge: StashEdge.Left, Distance: Math.Abs(window.Left - work.Left)),
            (Edge: StashEdge.Right, Distance: Math.Abs(work.Right - window.Right)),
            (Edge: StashEdge.Top, Distance: Math.Abs(window.Top - work.Top)),
            (Edge: StashEdge.Bottom, Distance: Math.Abs(work.Bottom - window.Bottom))
        };

        var nearest = distances[0];
        for (var i = 1; i < distances.Length; i++)
        {
            if (distances[i].Distance < nearest.Distance)
            {
                nearest = distances[i];
            }
        }

        return nearest.Edge;
    }

    internal static NativeMethods.Rect CalculateStashedFrame(
        NativeMethods.Rect work,
        NativeMethods.Rect window,
        StashEdge edge,
        int peek)
    {
        var width = Math.Max(1, window.Width);
        var height = Math.Max(1, window.Height);
        var visiblePeek = Math.Max(1, Math.Min(peek, Math.Min(width, height)));

        return edge switch
        {
            StashEdge.Left => Rect(work.Left - width + visiblePeek, window.Top, work.Left + visiblePeek, window.Bottom),
            StashEdge.Right => Rect(work.Right - visiblePeek, window.Top, work.Right + width - visiblePeek, window.Bottom),
            StashEdge.Top => Rect(window.Left, work.Top - height + visiblePeek, window.Right, work.Top + visiblePeek),
            StashEdge.Bottom => Rect(window.Left, work.Bottom - visiblePeek, window.Right, work.Bottom + height - visiblePeek),
            _ => window
        };
    }

    internal static bool IsInHitZone(
        NativeMethods.Point cursor,
        NativeMethods.Rect work,
        StashEdge edge,
        int hitZone = DefaultEdgeHitZone)
    {
        var zone = Math.Max(1, hitZone);
        return edge switch
        {
            StashEdge.Left => cursor.X >= work.Left && cursor.X <= work.Left + zone && cursor.Y >= work.Top && cursor.Y <= work.Bottom,
            StashEdge.Right => cursor.X >= work.Right - zone && cursor.X <= work.Right && cursor.Y >= work.Top && cursor.Y <= work.Bottom,
            StashEdge.Top => cursor.Y >= work.Top && cursor.Y <= work.Top + zone && cursor.X >= work.Left && cursor.X <= work.Right,
            StashEdge.Bottom => cursor.Y >= work.Bottom - zone && cursor.Y <= work.Bottom && cursor.X >= work.Left && cursor.X <= work.Right,
            _ => false
        };
    }

    private static int EdgePeek => Math.Clamp(_settings.StashEdgePeek, 1, 48);

    private static int HitZone => Math.Clamp(_settings.StashHitZone, 1, 96);

    private static int RevealDelayMilliseconds =>
        Math.Clamp(_settings.StashRevealDelayMilliseconds, 0, 2000);

    private static bool TryReveal(IntPtr window, out string message)
    {
        if (!Stashed.TryGetValue(window, out var stashed))
        {
            message = "The target window is not stashed.";
            return false;
        }

        var placement = stashed.OriginalPlacement;
        RebaseOriginalPlacement(ref placement, stashed.OriginalMonitor);
        if (!NativeMethods.SetWindowPlacement(window, ref placement))
        {
            if (!IsStashedWindowAlive(window, stashed))
            {
                RemoveRuntime(window, removePersisted: true);
            }

            message = "Windows rejected the reveal. The target may be elevated or closed.";
            return false;
        }

        RemoveRuntime(window, removePersisted: true);
        SavePersistence();
        message = "Revealed a stashed window";
        return true;
    }

    private static void RestorePersistedEntries()
    {
        if (_settings.StashRecords.Count == 0)
        {
            return;
        }

        var candidates = EnumerateWindowIdentities();
        var usedWindows = new HashSet<IntPtr>();
        var changed = false;

        foreach (var record in _settings.StashRecords.ToArray())
        {
            if (!WindowIdentityMatcher.TryFindUnambiguousMatch(record, candidates, out var window) ||
                usedWindows.Contains(window) ||
                !TryRestorePersistedEntry(record, window, out var stashed))
            {
                continue;
            }

            Stashed[window] = stashed;
            Order.Remove(window);
            Order.Add(window);
            usedWindows.Add(window);
            UpdateRecordIdentity(record, stashed.Identity);
            changed = true;
        }

        if (changed)
        {
            SavePersistence();
        }
    }

    private static bool TryRestorePersistedEntry(
        StashRecord record,
        IntPtr window,
        out StashedWindow stashed)
    {
        stashed = default;
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window) ||
            !TryReadWindowIdentity(window, out var identity))
        {
            return false;
        }

        NativeMethods.ShowWindow(window, NativeMethods.SwRestore);
        if (!NativeMethods.GetWindowRect(window, out var current) ||
            !TryGetMonitorSnapshot(current, out var currentMonitor))
        {
            return false;
        }

        var stashedFrame = IsUsable(record.StashedFrame)
            ? RebaseRect(record.StashedFrame.ToNative(), record.OriginalMonitor, currentMonitor)
            : CalculateStashedFrame(currentMonitor.Work.ToNative(), current, record.Edge, EdgePeek);
        if (!NativeMethods.SetWindowPos(
                window,
                IntPtr.Zero,
                stashedFrame.Left,
                stashedFrame.Top,
                stashedFrame.Width,
                stashedFrame.Height,
                NativeMethods.SwpNoActivate | NativeMethods.SwpNoZOrder | NativeMethods.SwpAsyncWindowPos))
        {
            return false;
        }

        var placement = ToNative(record.OriginalPlacement);
        var originalMonitor = record.OriginalMonitor;
        stashed = new StashedWindow(
            placement,
            originalMonitor,
            stashedFrame,
            record.Edge,
            identity,
            record.Id);
        return true;
    }

    private static void PruneStaleEntries()
    {
        var changed = false;
        foreach (var window in Order.ToArray())
        {
            if (!Stashed.TryGetValue(window, out var stashed) ||
                !IsStashedWindowAlive(window, stashed))
            {
                changed |= RemoveRuntime(window, removePersisted: true);
            }
        }

        if (changed)
        {
            SavePersistence();
        }
    }

    private static bool IsStashedWindowAlive(IntPtr window, StashedWindow stashed)
    {
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window) ||
            NativeMethods.GetWindowThreadProcessId(window, out var processId) == 0 ||
            processId != stashed.Identity.ProcessId)
        {
            return false;
        }

        if (!IsProcessAlive(processId))
        {
            return false;
        }

        if (TryReadWindowIdentity(window, out var currentIdentity))
        {
            if (!string.IsNullOrWhiteSpace(currentIdentity.ExecutablePath))
            {
                return WindowIdentityMatcher.IsSameRuntimeWindow(
                    ToRecord(stashed),
                    currentIdentity);
            }

            return currentIdentity.ProcessId == stashed.Identity.ProcessId &&
                string.Equals(
                    currentIdentity.WindowClass,
                    stashed.Identity.WindowClass,
                    StringComparison.OrdinalIgnoreCase);
        }

        // Access to an elevated process module can be denied. The HWND and
        // process ID checks above still provide a safe session-level fallback.
        return true;
    }

    private static bool IsProcessAlive(uint processId)
    {
        try
        {
            using var process = Process.GetProcessById((int)processId);
            return !process.HasExited;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (System.ComponentModel.Win32Exception)
        {
            return true;
        }
    }

    private static bool TryReadWindowIdentity(IntPtr window, out WindowIdentity identity)
    {
        identity = default;
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window) ||
            NativeMethods.GetWindowThreadProcessId(window, out var processId) == 0 ||
            processId == 0)
        {
            return false;
        }

        var className = new StringBuilder(256);
        if (NativeMethods.GetClassName(window, className, className.Capacity) == 0)
        {
            return false;
        }

        var title = new StringBuilder(512);
        NativeMethods.GetWindowText(window, title, title.Capacity);
        var executablePath = string.Empty;
        try
        {
            using var process = Process.GetProcessById((int)processId);
            executablePath = process.MainModule?.FileName ?? string.Empty;
        }
        catch (System.ComponentModel.Win32Exception)
        {
            // Some elevated or protected processes deny module inspection. They
            // remain usable for the current session, but cannot be restored from
            // persistence without an executable-path anchor.
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (ArgumentException)
        {
            return false;
        }

        identity = new WindowIdentity(
            executablePath,
            processId,
            className.ToString(),
            title.ToString());
        return true;
    }

    private static List<WindowIdentityCandidate<IntPtr>> EnumerateWindowIdentities()
    {
        var candidates = new List<WindowIdentityCandidate<IntPtr>>();
        NativeMethods.EnumWindows((window, _) =>
        {
            if (NativeMethods.GetWindowThreadProcessId(window, out var processId) == 0 ||
                processId == Environment.ProcessId ||
                !TryReadWindowIdentity(window, out var identity))
            {
                return true;
            }

            candidates.Add(new WindowIdentityCandidate<IntPtr>(window, identity));
            return true;
        }, IntPtr.Zero);
        return candidates;
    }

    private static bool CanPersist(WindowIdentity identity) =>
        _settings.StashPersistenceEnabled &&
        !string.IsNullOrWhiteSpace(identity.ExecutablePath) &&
        !string.IsNullOrWhiteSpace(identity.WindowClass);

    private static void Persist(StashedWindow stashed)
    {
        if (string.IsNullOrEmpty(stashed.PersistedId))
        {
            return;
        }

        var record = ToRecord(stashed);
        var index = _settings.StashRecords.FindIndex(existing => existing.Id == record.Id);
        if (index >= 0)
        {
            _settings.StashRecords[index] = record;
        }
        else
        {
            _settings.StashRecords.Add(record);
        }

        SavePersistence();
    }

    private static bool RemoveRuntime(IntPtr window, bool removePersisted)
    {
        if (!Stashed.Remove(window, out var stashed))
        {
            Order.Remove(window);
            return false;
        }

        Order.Remove(window);
        ResetPendingReveal(window);
        if (removePersisted && !string.IsNullOrEmpty(stashed.PersistedId))
        {
            _settings.StashRecords.RemoveAll(record => record.Id == stashed.PersistedId);
        }

        return true;
    }

    private static void SavePersistence()
    {
        if (_settings.StashPersistenceEnabled)
        {
            _settings.Save();
        }
    }

    private static StashRecord ToRecord(StashedWindow stashed) => new()
    {
        Id = stashed.PersistedId,
        ExecutablePath = stashed.Identity.ExecutablePath,
        ProcessId = stashed.Identity.ProcessId,
        WindowClass = stashed.Identity.WindowClass,
        Title = stashed.Identity.Title,
        Edge = stashed.Edge,
        OriginalPlacement = FromNative(stashed.OriginalPlacement),
        OriginalMonitor = stashed.OriginalMonitor,
        StashedFrame = StashRect.FromNative(stashed.StashedFrame)
    };

    private static void UpdateRecordIdentity(StashRecord record, WindowIdentity identity)
    {
        record.ExecutablePath = identity.ExecutablePath;
        record.ProcessId = identity.ProcessId;
        record.WindowClass = identity.WindowClass;
        record.Title = identity.Title;
    }

    private static NativeMethods.WindowPlacement ToNative(StashPlacement placement) => new()
    {
        Length = placement.Length == 0
            ? Marshal.SizeOf<NativeMethods.WindowPlacement>()
            : placement.Length,
        Flags = placement.Flags,
        ShowCmd = placement.ShowCommand,
        MinPosition = placement.MinPosition.ToNative(),
        MaxPosition = placement.MaxPosition.ToNative(),
        NormalPosition = placement.NormalPosition.ToNative()
    };

    private static StashPlacement FromNative(NativeMethods.WindowPlacement placement) => new()
    {
        Length = placement.Length,
        Flags = placement.Flags,
        ShowCommand = placement.ShowCmd,
        MinPosition = StashPoint.FromNative(placement.MinPosition),
        MaxPosition = StashPoint.FromNative(placement.MaxPosition),
        NormalPosition = StashRect.FromNative(placement.NormalPosition)
    };

    private static void RestoreOriginalPlacement(IntPtr window, NativeMethods.WindowPlacement placement)
    {
        NativeMethods.SetWindowPlacement(window, ref placement);
    }

    private static bool TryRestoreOriginalPlacement(IntPtr window, StashedWindow stashed)
    {
        var placement = stashed.OriginalPlacement;
        RebaseOriginalPlacement(ref placement, stashed.OriginalMonitor);
        return NativeMethods.SetWindowPlacement(window, ref placement);
    }

    private static void RebaseOriginalPlacement(
        ref NativeMethods.WindowPlacement placement,
        StashMonitor originalMonitor)
    {
        if (!TryFindRestoreMonitor(originalMonitor, out var targetMonitor))
        {
            return;
        }

        placement.NormalPosition = RebaseRect(
            placement.NormalPosition,
            originalMonitor,
            targetMonitor);
    }

    private static bool TryGetMonitorSnapshot(NativeMethods.Rect rect, out StashMonitor snapshot)
    {
        var monitor = NativeMethods.MonitorFromRect(ref rect, NativeMethods.MonitorDefaultToNearest);
        if (monitor == IntPtr.Zero)
        {
            snapshot = new StashMonitor();
            return false;
        }

        var info = new NativeMethods.MonitorInfo
        {
            Size = Marshal.SizeOf<NativeMethods.MonitorInfo>()
        };
        if (!NativeMethods.GetMonitorInfo(monitor, ref info))
        {
            snapshot = new StashMonitor();
            return false;
        }

        NativeMethods.TryGetMonitorDpi(monitor, out var dpiX, out var dpiY);
        snapshot = new StashMonitor
        {
            Monitor = StashRect.FromNative(info.Monitor),
            Work = StashRect.FromNative(info.Work),
            DpiX = NormalizeDpi(dpiX),
            DpiY = NormalizeDpi(dpiY)
        };
        return true;
    }

    private static IReadOnlyList<StashMonitor> EnumerateMonitors()
    {
        var monitors = new List<StashMonitor>();
        NativeMethods.EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (monitor, _, _, _) =>
        {
            var info = new NativeMethods.MonitorInfo
            {
                Size = Marshal.SizeOf<NativeMethods.MonitorInfo>()
            };
            if (!NativeMethods.GetMonitorInfo(monitor, ref info))
            {
                return true;
            }

            NativeMethods.TryGetMonitorDpi(monitor, out var dpiX, out var dpiY);
            monitors.Add(new StashMonitor
            {
                Monitor = StashRect.FromNative(info.Monitor),
                Work = StashRect.FromNative(info.Work),
                DpiX = NormalizeDpi(dpiX),
                DpiY = NormalizeDpi(dpiY)
            });
            return true;
        }, IntPtr.Zero);
        return monitors;
    }

    private static bool TryFindRestoreMonitor(StashMonitor original, out StashMonitor target)
    {
        var monitors = EnumerateMonitors();
        foreach (var monitor in monitors)
        {
            if (SameRect(monitor.Monitor, original.Monitor) ||
                SameRect(monitor.Work, original.Work))
            {
                target = monitor;
                return true;
            }
        }

        if (monitors.Count == 0 || !IsUsable(original.Monitor))
        {
            target = new StashMonitor();
            return false;
        }

        var originalCenterX = ((long)original.Monitor.Left + original.Monitor.Right) / 2;
        var originalCenterY = ((long)original.Monitor.Top + original.Monitor.Bottom) / 2;
        target = monitors
            .OrderBy(monitor =>
            {
                var centerX = ((long)monitor.Monitor.Left + monitor.Monitor.Right) / 2;
                var centerY = ((long)monitor.Monitor.Top + monitor.Monitor.Bottom) / 2;
                return Math.Abs(centerX - originalCenterX) + Math.Abs(centerY - originalCenterY);
            })
            .First();
        return true;
    }

    private static NativeMethods.Rect RebaseRect(
        NativeMethods.Rect rect,
        StashMonitor original,
        StashMonitor target)
    {
        if (!IsUsable(original.Work) || !IsUsable(target.Work))
        {
            return rect;
        }

        var scaleX = NormalizeDpi(target.DpiX) / NormalizeDpi(original.DpiX);
        var scaleY = NormalizeDpi(target.DpiY) / NormalizeDpi(original.DpiY);
        return Rect(
            Scale(rect.Left - original.Work.Left, scaleX) + target.Work.Left,
            Scale(rect.Top - original.Work.Top, scaleY) + target.Work.Top,
            Scale(rect.Right - original.Work.Left, scaleX) + target.Work.Left,
            Scale(rect.Bottom - original.Work.Top, scaleY) + target.Work.Top);
    }

    private static int Scale(int value, double scale)
    {
        var scaled = value * scale;
        return double.IsFinite(scaled)
            ? (int)Math.Clamp(Math.Round(scaled), int.MinValue, int.MaxValue)
            : value;
    }

    private static bool ReadyToReveal(IntPtr window)
    {
        var delay = RevealDelayMilliseconds;
        if (delay == 0)
        {
            return true;
        }

        var now = Environment.TickCount64;
        if (_pendingRevealWindow != window)
        {
            _pendingRevealWindow = window;
            _pendingRevealStartedAt = now;
            return false;
        }

        return now - _pendingRevealStartedAt >= delay;
    }

    private static void ResetPendingReveal(IntPtr? removedWindow = null)
    {
        if (removedWindow.HasValue && _pendingRevealWindow != removedWindow.Value)
        {
            return;
        }

        _pendingRevealWindow = IntPtr.Zero;
        _pendingRevealStartedAt = 0;
    }

    private static bool IsUsable(StashRect rect) => rect.Right > rect.Left && rect.Bottom > rect.Top;

    private static bool SameRect(StashRect left, StashRect right) =>
        left.Left == right.Left && left.Top == right.Top &&
        left.Right == right.Right && left.Bottom == right.Bottom;

    private static double NormalizeDpi(double dpi) => double.IsFinite(dpi) && dpi > 0 ? dpi : 96;

    private static double NormalizeDpi(uint dpi) => dpi > 0 ? dpi : 96;

    private static NativeMethods.Rect Rect(int left, int top, int right, int bottom) => new()
    {
        Left = left,
        Top = top,
        Right = right,
        Bottom = bottom
    };
}
