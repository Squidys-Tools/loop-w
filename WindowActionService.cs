using System;
using System.Runtime.InteropServices;
using System.Threading;

namespace LoopW;

internal enum WindowHalf
{
    Left,
    Right,
    Top,
    Bottom
}

internal static class WindowActionService
{
    private const int PlaceTolerance = 2;

    public static bool TryGetHalfFrame(IntPtr window, WindowHalf half, out NativeMethods.Rect target, out string error)
    {
        target = default;
        error = string.Empty;

        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            error = "The target window is no longer available.";
            return false;
        }

        var monitor = NativeMethods.MonitorFromWindow(window, 2);
        var monitorInfo = new NativeMethods.MonitorInfo { Size = Marshal.SizeOf<NativeMethods.MonitorInfo>() };
        if (monitor == IntPtr.Zero || !NativeMethods.GetMonitorInfo(monitor, ref monitorInfo))
        {
            error = "Could not determine the target monitor.";
            return false;
        }

        var work = monitorInfo.Work;
        target = half switch
        {
            WindowHalf.Left => new NativeMethods.Rect { Left = work.Left, Top = work.Top, Right = work.Left + work.Width / 2, Bottom = work.Bottom },
            WindowHalf.Right => new NativeMethods.Rect { Left = work.Left + work.Width / 2, Top = work.Top, Right = work.Right, Bottom = work.Bottom },
            WindowHalf.Top => new NativeMethods.Rect { Left = work.Left, Top = work.Top, Right = work.Right, Bottom = work.Top + work.Height / 2 },
            WindowHalf.Bottom => new NativeMethods.Rect { Left = work.Left, Top = work.Top + work.Height / 2, Right = work.Right, Bottom = work.Bottom },
            _ => work
        };

        return true;
    }

    /// <summary>
    /// Snaps the window to a half of its monitor's work area, then returns a status
    /// message describing what actually happened. Works for any window regardless of
    /// its current state (maximized, minimized) or its min/max size constraints.
    /// </summary>
    public static bool ApplyHalf(IntPtr window, WindowHalf half, out string message)
    {
        message = string.Empty;

        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            message = "The target window is no longer available.";
            return false;
        }

        if (!TryGetHalfFrame(window, half, out var ideal, out message) ||
            !NativeMethods.TryGetMonitorWorkRect(ideal, out var work))
        {
            if (string.IsNullOrEmpty(message))
            {
                message = "Could not determine the target monitor.";
            }

            return false;
        }

        // Clamp the ideal frame to the window's real size limits, then anchor it to
        // the zone edge so it can never overflow off-screen.
        var frame = BestFitFrame(work, half, ideal, GetMinMaxInfo(window));

        if (!PlaceWindow(window, frame))
        {
            message = "Windows rejected the move. The target may be elevated or non-resizable.";
            return false;
        }

        // Some apps clamp to their min size without honoring the zone anchor. Read the
        // settled result and re-anchor once so the window stays fully on-screen.
        if (!NativeMethods.GetWindowRect(window, out var actual))
        {
            message = "Could not read the window's final position.";
            return false;
        }

        if (!RectsEqual(actual, frame))
        {
            var reanchored = BestFitFrame(work, half, actual, new NativeMethods.MinMaxInfo());
            if (!RectsEqual(reanchored, frame))
            {
                PlaceWindow(window, reanchored);
                NativeMethods.GetWindowRect(window, out actual);
            }
        }

        var label = HalfLabel(half);
        message = SizesEqual(actual, ideal)
            ? $"Applied {label} to target window"
            : $"Snapped to {label}, but the window's minimum/maximum size forced {actual.Width}\u00d7{actual.Height} instead of {ideal.Width}\u00d7{ideal.Height}.";

        return true;
    }

    private static string HalfLabel(WindowHalf half) => half switch
    {
        WindowHalf.Left => "Left half",
        WindowHalf.Right => "Right half",
        WindowHalf.Top => "Top half",
        WindowHalf.Bottom => "Bottom half",
        _ => "Window"
    };

    private static NativeMethods.MinMaxInfo GetMinMaxInfo(IntPtr window)
    {
        var info = new NativeMethods.MinMaxInfo();
        // Never block on an unresponsive app; a failed query just means no pre-clamp.
        if (NativeMethods.SendMessageTimeout(
            window,
            NativeMethods.WmGetMinMaxInfo,
            IntPtr.Zero,
            ref info,
            NativeMethods.SmtoAbortIfHung,
            200,
            out _) != IntPtr.Zero)
        {
            return info;
        }

        return new NativeMethods.MinMaxInfo();
    }

    /// <summary>
    /// Clamps the frame size to the window's min/max track sizes and the monitor
    /// bounds, then pins the frame to the zone edge along the split axis so the
    /// window always stays fully on-screen.
    /// </summary>
    private static NativeMethods.Rect BestFitFrame(
        NativeMethods.Rect work,
        WindowHalf half,
        NativeMethods.Rect frame,
        NativeMethods.MinMaxInfo limits)
    {
        var width = frame.Width;
        var height = frame.Height;

        width = Math.Max(width, limits.MinTrackSize.X);
        height = Math.Max(height, limits.MinTrackSize.Y);
        if (limits.MaxTrackSize.X > 0)
        {
            width = Math.Min(width, limits.MaxTrackSize.X);
        }

        if (limits.MaxTrackSize.Y > 0)
        {
            height = Math.Min(height, limits.MaxTrackSize.Y);
        }

        width = Math.Min(width, work.Width);
        height = Math.Min(height, work.Height);

        return half switch
        {
            WindowHalf.Left => new NativeMethods.Rect { Left = work.Left, Right = work.Left + width, Top = work.Top, Bottom = work.Bottom },
            WindowHalf.Right => new NativeMethods.Rect { Left = work.Right - width, Right = work.Right, Top = work.Top, Bottom = work.Bottom },
            WindowHalf.Top => new NativeMethods.Rect { Left = work.Left, Right = work.Right, Top = work.Top, Bottom = work.Top + height },
            WindowHalf.Bottom => new NativeMethods.Rect { Left = work.Left, Right = work.Right, Top = work.Bottom - height, Bottom = work.Bottom },
            _ => work
        };
    }

    /// <summary>
    /// Places the window at the given frame using WINDOWPLACEMENT so maximized and
    /// minimized windows are restored reliably (SetWindowPlacement is called twice,
    /// matching the PowerToys FancyZones pattern that also fixes DPI scaling), then
    /// waits for the async restore to settle before returning.
    /// </summary>
    private static bool PlaceWindow(IntPtr window, NativeMethods.Rect frame)
    {
        var placement = new NativeMethods.WindowPlacement { Length = Marshal.SizeOf<NativeMethods.WindowPlacement>() };
        if (!NativeMethods.GetWindowPlacement(window, ref placement))
        {
            return false;
        }

        // Give a minimized window a moment to come back before repositioning it.
        for (var i = 0; i < 10 && placement.ShowCmd == NativeMethods.SwShowMinimized; i++)
        {
            Thread.Sleep(50);
            NativeMethods.GetWindowPlacement(window, ref placement);
        }

        var next = new NativeMethods.WindowPlacement
        {
            Length = Marshal.SizeOf<NativeMethods.WindowPlacement>(),
            ShowCmd = NativeMethods.SwRestore,
            Flags = NativeMethods.WpfAsyncWindowPlacement,
            NormalPosition = frame
        };

        if (!NativeMethods.SetWindowPlacement(window, ref next))
        {
            return false;
        }

        NativeMethods.SetWindowPlacement(window, ref next);

        // The restore transition is asynchronous: GetWindowRect can keep reporting
        // the maximized frame for a while. Wait for the window to actually leave the
        // maximized state and reach the frame (or stabilize somewhere else).
        if (WaitForPlacement(window, frame))
        {
            return true;
        }

        // Fallback for apps that ignore WINDOWPLACEMENT: hard-restore + SetWindowPos.
        NativeMethods.ShowWindow(window, NativeMethods.SwRestore);
        if (!NativeMethods.SetWindowPos(window, IntPtr.Zero, frame.Left, frame.Top, frame.Width, frame.Height,
                NativeMethods.SwpNoZOrder | NativeMethods.SwpNoActivate | NativeMethods.SwpAsyncWindowPos))
        {
            return false;
        }

        return WaitForPlacement(window, frame);
    }

    private static bool WaitForPlacement(IntPtr window, NativeMethods.Rect frame)
    {
        var deadline = Environment.TickCount64 + 600;
        NativeMethods.Rect previous = default;
        var previousValid = false;

        while (Environment.TickCount64 < deadline)
        {
            var placement = new NativeMethods.WindowPlacement { Length = Marshal.SizeOf<NativeMethods.WindowPlacement>() };
            if (!NativeMethods.GetWindowPlacement(window, ref placement))
            {
                return false;
            }

            if (placement.ShowCmd != NativeMethods.SwShowMaximized && !NativeMethods.IsZoomed(window))
            {
                if (NativeMethods.GetWindowRect(window, out var actual))
                {
                    if (RectsEqual(actual, frame))
                    {
                        return true;
                    }

                    if (previousValid && RectsEqual(actual, previous))
                    {
                        return true;
                    }

                    previous = actual;
                    previousValid = true;
                }
            }

            Thread.Sleep(40);
        }

        return !NativeMethods.IsZoomed(window);
    }

    private static bool RectsEqual(NativeMethods.Rect a, NativeMethods.Rect b) =>
        Math.Abs(a.Left - b.Left) <= PlaceTolerance &&
        Math.Abs(a.Top - b.Top) <= PlaceTolerance &&
        Math.Abs(a.Width - b.Width) <= PlaceTolerance &&
        Math.Abs(a.Height - b.Height) <= PlaceTolerance;

    private static bool SizesEqual(NativeMethods.Rect a, NativeMethods.Rect b) =>
        Math.Abs(a.Width - b.Width) <= PlaceTolerance &&
        Math.Abs(a.Height - b.Height) <= PlaceTolerance;
}
