using System;
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
        var monitorInfo = new NativeMethods.MonitorInfo { Size = System.Runtime.InteropServices.Marshal.SizeOf<NativeMethods.MonitorInfo>() };
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

    public static bool ApplyHalf(IntPtr window, WindowHalf half, out string error)
    {
        if (!TryGetHalfFrame(window, half, out var target, out error))
        {
            return false;
        }

        NativeMethods.ShowWindow(window, NativeMethods.SwRestore);
        var moved = NativeMethods.SetWindowPos(
            window,
            NativeMethods.HwndTop,
            target.Left,
            target.Top,
            target.Width,
            target.Height,
            NativeMethods.SwpNoActivate | NativeMethods.SwpNoZOrder);

        if (!moved)
        {
            error = "Windows rejected the move. The target may be elevated or non-resizable.";
        }

        return moved;
    }
}
