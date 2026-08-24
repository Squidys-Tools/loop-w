using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;

namespace LoopW;

public enum MonitorMoveSizePolicy
{
    PreservePixels,
    PreserveLogicalSize
}

internal readonly record struct MonitorSnapshot(
    NativeMethods.Rect Monitor,
    NativeMethods.Rect Work,
    double DpiX,
    double DpiY);

internal static class MonitorService
{
    private static readonly object CacheGate = new();
    private static readonly Dictionary<IntPtr, MonitorSnapshot> SnapshotCache = new();
    private static AppSettings _settings = new();
    private static long _generation;
    private static MonitorSnapshot[]? _allMonitors;

    public static long Generation
    {
        get
        {
            lock (CacheGate)
            {
                return _generation;
            }
        }
    }

    public static MonitorMoveSizePolicy MoveSizePolicy
    {
        get
        {
            lock (CacheGate)
            {
                return _settings.MonitorMoveSizePolicy;
            }
        }
    }

    public static void Configure(AppSettings settings)
    {
        lock (CacheGate)
        {
            _settings = settings;
            InvalidateLocked();
        }
    }

    public static void UpdateSettings(AppSettings settings)
    {
        lock (CacheGate)
        {
            _settings = settings;
            InvalidateLocked();
        }
    }

    public static void Invalidate()
    {
        lock (CacheGate)
        {
            InvalidateLocked();
        }
    }

    public static bool TryGetForWindow(IntPtr window, out MonitorSnapshot snapshot)
    {
        var monitor = NativeMethods.MonitorFromWindow(window, NativeMethods.MonitorDefaultToNearest);
        return TryRead(monitor, out snapshot);
    }

    public static bool TryGetForRect(NativeMethods.Rect rect, out MonitorSnapshot snapshot)
    {
        var monitor = NativeMethods.MonitorFromRect(ref rect, NativeMethods.MonitorDefaultToNearest);
        return TryRead(monitor, out snapshot);
    }

    public static bool TryGetForPoint(NativeMethods.Point point, out MonitorSnapshot snapshot)
    {
        var monitor = NativeMethods.MonitorFromPoint(point, NativeMethods.MonitorDefaultToNearest);
        return TryRead(monitor, out snapshot);
    }

    public static IReadOnlyList<MonitorSnapshot> GetAll()
    {
        lock (CacheGate)
        {
            if (_allMonitors is not null)
            {
                return _allMonitors;
            }

            var monitors = new List<MonitorSnapshot>();
            NativeMethods.EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (monitor, _, _, _) =>
            {
                if (TryRead(monitor, out var snapshot))
                {
                    monitors.Add(snapshot);
                }

                return true;
            }, IntPtr.Zero);

            _allMonitors = monitors
                .OrderBy(snapshot => snapshot.Work.Left)
                .ThenBy(snapshot => snapshot.Work.Top)
                .ToArray();
            return _allMonitors;
        }
    }

    internal static NativeMethods.Rect ApplyPadding(
        NativeMethods.Rect work,
        AppSettings settings)
    {
        var left = Math.Max(0, settings.GlobalScreenPadding + settings.ScreenPaddingLeft);
        var top = Math.Max(0, settings.GlobalScreenPadding + settings.ScreenPaddingTop);
        var right = Math.Max(0, settings.GlobalScreenPadding + settings.ScreenPaddingRight);
        var bottom = Math.Max(0, settings.GlobalScreenPadding + settings.ScreenPaddingBottom);

        left = Math.Min(left, Math.Max(0, work.Width - 1));
        right = Math.Min(right, Math.Max(0, work.Width - left - 1));
        top = Math.Min(top, Math.Max(0, work.Height - 1));
        bottom = Math.Min(bottom, Math.Max(0, work.Height - top - 1));

        var horizontal = Math.Max(1, work.Width - left - right);
        var vertical = Math.Max(1, work.Height - top - bottom);
        return new NativeMethods.Rect
        {
            Left = work.Left + left,
            Top = work.Top + top,
            Right = work.Left + left + horizontal,
            Bottom = work.Top + top + vertical
        };
    }

    internal static NativeMethods.Rect TranslateFrame(
        NativeMethods.Rect current,
        MonitorSnapshot source,
        MonitorSnapshot target,
        MonitorMoveSizePolicy policy)
    {
        var scaleX = policy == MonitorMoveSizePolicy.PreserveLogicalSize
            ? NormalizeDpi(target.DpiX) / NormalizeDpi(source.DpiX)
            : 1;
        var scaleY = policy == MonitorMoveSizePolicy.PreserveLogicalSize
            ? NormalizeDpi(target.DpiY) / NormalizeDpi(source.DpiY)
            : 1;

        var left = target.Work.Left + Scale(current.Left - source.Work.Left, scaleX);
        var top = target.Work.Top + Scale(current.Top - source.Work.Top, scaleY);
        var width = Math.Max(1, Scale(current.Width, scaleX));
        var height = Math.Max(1, Scale(current.Height, scaleY));
        return new NativeMethods.Rect
        {
            Left = left,
            Top = top,
            Right = left + width,
            Bottom = top + height
        };
    }

    private static bool TryRead(IntPtr monitor, out MonitorSnapshot snapshot)
    {
        snapshot = default;
        if (monitor == IntPtr.Zero)
        {
            return false;
        }

        lock (CacheGate)
        {
            if (SnapshotCache.TryGetValue(monitor, out snapshot))
            {
                return true;
            }

            var info = new NativeMethods.MonitorInfo
            {
                Size = Marshal.SizeOf<NativeMethods.MonitorInfo>()
            };
            if (!NativeMethods.GetMonitorInfo(monitor, ref info))
            {
                return false;
            }

            NativeMethods.TryGetMonitorDpi(monitor, out var dpiX, out var dpiY);
            snapshot = new MonitorSnapshot(
                info.Monitor,
                ApplyPadding(info.Work, _settings),
                NormalizeDpi(dpiX),
                NormalizeDpi(dpiY));
            SnapshotCache[monitor] = snapshot;
            return true;
        }
    }

    private static void InvalidateLocked()
    {
        _generation++;
        SnapshotCache.Clear();
        _allMonitors = null;
    }

    private static int Scale(int value, double scale)
    {
        var scaled = value * scale;
        return double.IsFinite(scaled)
            ? (int)Math.Clamp(Math.Round(scaled), int.MinValue, int.MaxValue)
            : value;
    }

    private static double NormalizeDpi(double dpi) => double.IsFinite(dpi) && dpi > 0 ? dpi : 96;
}
