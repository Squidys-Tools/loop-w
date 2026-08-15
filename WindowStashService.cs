using System;
using System.Collections.Generic;

namespace LoopW;

internal enum StashEdge
{
    Left,
    Right,
    Top,
    Bottom
}

internal static class WindowStashService
{
    private const int EdgeHitZone = 14;
    private const int EdgePeek = 8;

    private readonly record struct StashedWindow(
        NativeMethods.WindowPlacement OriginalPlacement,
        NativeMethods.Rect StashedFrame,
        StashEdge Edge);

    private static readonly Dictionary<IntPtr, StashedWindow> Stashed = new();
    private static readonly List<IntPtr> Order = new();

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

        var placement = new NativeMethods.WindowPlacement
        {
            Length = System.Runtime.InteropServices.Marshal.SizeOf<NativeMethods.WindowPlacement>()
        };
        if (!NativeMethods.GetWindowPlacement(window, ref placement) ||
            !NativeMethods.GetWindowRect(window, out var current))
        {
            message = "Could not read the target window's placement.";
            return false;
        }

        NativeMethods.ShowWindow(window, NativeMethods.SwRestore);
        if (!NativeMethods.GetWindowRect(window, out current) ||
            !NativeMethods.TryGetMonitorWorkRect(current, out var work))
        {
            RestoreOriginalPlacement(window, placement);
            message = "Could not determine the target monitor.";
            return false;
        }

        var edge = NearestEdge(work, current);
        var stashedFrame = CalculateStashedFrame(work, current, edge, EdgePeek);
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

        Stashed[window] = new StashedWindow(placement, stashedFrame, edge);
        Order.Remove(window);
        Order.Add(window);
        message = $"Stashed the window at the {edge.ToString().ToLowerInvariant()} edge";
        return true;
    }

    public static bool TryRevealNext(out string message)
    {
        for (var i = 0; i < Order.Count; i++)
        {
            var window = Order[i];
            if (!Stashed.ContainsKey(window) || !NativeMethods.IsWindow(window))
            {
                Remove(window);
                i--;
                continue;
            }

            return TryReveal(window, out message);
        }

        message = "No stashed windows to reveal.";
        return false;
    }

    internal static void RestoreAll()
    {
        foreach (var window in Order.ToArray())
        {
            if (Stashed.TryGetValue(window, out var stashed) && NativeMethods.IsWindow(window))
            {
                var placement = stashed.OriginalPlacement;
                NativeMethods.SetWindowPlacement(window, ref placement);
            }
        }

        Stashed.Clear();
        Order.Clear();
    }

    public static bool TryRevealAtCursor(NativeMethods.Point cursor, out string message)
    {
        for (var i = 0; i < Order.Count; i++)
        {
            var window = Order[i];
            if (!Stashed.TryGetValue(window, out var stashed) || !NativeMethods.IsWindow(window))
            {
                Remove(window);
                i--;
                continue;
            }

            if (!NativeMethods.TryGetMonitorWorkRect(stashed.StashedFrame, out var work) ||
                !IsInHitZone(cursor, work, stashed.Edge))
            {
                continue;
            }

            return TryReveal(window, out message);
        }

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

    private static bool TryReveal(IntPtr window, out string message)
    {
        if (!Stashed.TryGetValue(window, out var stashed))
        {
            message = "The target window is not stashed.";
            return false;
        }

        var placement = stashed.OriginalPlacement;
        if (!NativeMethods.SetWindowPlacement(window, ref placement))
        {
            message = "Windows rejected the reveal. The target may be elevated or closed.";
            return false;
        }

        Remove(window);
        message = "Revealed a stashed window";
        return true;
    }

    private static void RestoreOriginalPlacement(IntPtr window, NativeMethods.WindowPlacement placement)
    {
        NativeMethods.SetWindowPlacement(window, ref placement);
    }

    private static bool IsInHitZone(NativeMethods.Point cursor, NativeMethods.Rect work, StashEdge edge)
    {
        return edge switch
        {
            StashEdge.Left => cursor.X >= work.Left && cursor.X <= work.Left + EdgeHitZone && cursor.Y >= work.Top && cursor.Y <= work.Bottom,
            StashEdge.Right => cursor.X >= work.Right - EdgeHitZone && cursor.X <= work.Right && cursor.Y >= work.Top && cursor.Y <= work.Bottom,
            StashEdge.Top => cursor.Y >= work.Top && cursor.Y <= work.Top + EdgeHitZone && cursor.X >= work.Left && cursor.X <= work.Right,
            StashEdge.Bottom => cursor.Y >= work.Bottom - EdgeHitZone && cursor.Y <= work.Bottom && cursor.X >= work.Left && cursor.X <= work.Right,
            _ => false
        };
    }

    private static void Remove(IntPtr window)
    {
        Stashed.Remove(window);
        Order.Remove(window);
    }

    private static NativeMethods.Rect Rect(int left, int top, int right, int bottom) =>
        new() { Left = left, Top = top, Right = right, Bottom = bottom };
}
