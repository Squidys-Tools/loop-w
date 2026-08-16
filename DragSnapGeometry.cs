using System;

namespace LoopW;

internal enum DragSnapZone
{
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter
}

internal static class DragSnapGeometry
{
    public static bool TryResolve(
        NativeMethods.Rect monitor,
        NativeMethods.Rect workArea,
        NativeMethods.Point cursor,
        int threshold,
        out DragSnapZone zone)
    {
        zone = default;
        threshold = Math.Clamp(threshold, 1, 256);
        if (monitor.Width <= 0 || monitor.Height <= 0 ||
            workArea.Width <= 0 || workArea.Height <= 0 ||
            cursor.X < monitor.Left || cursor.X > monitor.Right ||
            cursor.Y < monitor.Top || cursor.Y > monitor.Bottom)
        {
            return false;
        }

        var nearLeft = cursor.X - monitor.Left <= threshold;
        var nearRight = monitor.Right - cursor.X <= threshold;
        var nearTop = cursor.Y - monitor.Top <= threshold;
        var nearBottom = monitor.Bottom - cursor.Y <= threshold;

        if (!nearLeft && !nearRight && !nearTop && !nearBottom)
        {
            return false;
        }

        zone = (nearLeft, nearRight, nearTop, nearBottom) switch
        {
            (true, false, true, false) => DragSnapZone.TopLeftQuarter,
            (false, true, true, false) => DragSnapZone.TopRightQuarter,
            (true, false, false, true) => DragSnapZone.BottomLeftQuarter,
            (false, true, false, true) => DragSnapZone.BottomRightQuarter,
            (true, false, _, _) => DragSnapZone.LeftHalf,
            (false, true, _, _) => DragSnapZone.RightHalf,
            (_, _, true, false) => DragSnapZone.TopHalf,
            (_, _, false, true) => DragSnapZone.BottomHalf,
            _ => default
        };

        return true;
    }

    public static WindowAction ActionOf(DragSnapZone zone) => zone switch
    {
        DragSnapZone.LeftHalf => WindowAction.LeftHalf,
        DragSnapZone.RightHalf => WindowAction.RightHalf,
        DragSnapZone.TopHalf => WindowAction.TopHalf,
        DragSnapZone.BottomHalf => WindowAction.BottomHalf,
        DragSnapZone.TopLeftQuarter => WindowAction.TopLeftQuarter,
        DragSnapZone.TopRightQuarter => WindowAction.TopRightQuarter,
        DragSnapZone.BottomLeftQuarter => WindowAction.BottomLeftQuarter,
        DragSnapZone.BottomRightQuarter => WindowAction.BottomRightQuarter,
        _ => throw new ArgumentOutOfRangeException(nameof(zone), zone, "Unknown drag snap zone.")
    };
}
