using System;
using System.Collections.Generic;

namespace LoopW;

internal enum WindowNavigationDirection
{
    Left,
    Right,
    Up,
    Down
}

internal readonly record struct WindowCandidate(IntPtr Handle, NativeMethods.Rect Frame);

internal static class WindowQuery
{
    public static IReadOnlyList<WindowCandidate> GetFocusableWindows(IntPtr excludedWindow)
    {
        return Enumerate(excludedWindow);
    }

    public static IReadOnlyList<WindowCandidate> GetMinimizableWindows(IntPtr excludedWindow)
    {
        return Enumerate(excludedWindow);
    }

    public static IReadOnlyList<NativeMethods.Rect> GetLayoutWindows(IntPtr excludedWindow)
    {
        var windows = Enumerate(excludedWindow);
        var frames = new List<NativeMethods.Rect>(windows.Count);
        foreach (var window in windows)
        {
            frames.Add(window.Frame);
        }

        return frames;
    }

    public static bool IsEligibleForSnap(IntPtr window) =>
        !NativeMethods.IsIconic(window) &&
        WindowPolicy.IsEligibleForEnumeration(window, IntPtr.Zero);

    private static List<WindowCandidate> Enumerate(IntPtr excludedWindow)
    {
        var windows = new List<WindowCandidate>();
        var cache = WindowPolicy.CreateEnumerationCache();
        NativeMethods.EnumWindows((window, _) =>
        {
            if (!NativeMethods.IsIconic(window) &&
                WindowPolicy.IsEligibleForEnumeration(window, excludedWindow, cache) &&
                NativeMethods.GetWindowRect(window, out var frame) &&
                frame.Width > 0 && frame.Height > 0)
            {
                windows.Add(new WindowCandidate(window, frame));
            }

            return true;
        }, IntPtr.Zero);
        return windows;
    }

}

internal static class WindowNavigation
{
    public static bool TryFindDirectional(
        NativeMethods.Rect source,
        IReadOnlyList<WindowCandidate> candidates,
        WindowNavigationDirection direction,
        out IntPtr target)
    {
        target = IntPtr.Zero;
        var found = false;
        var bestPrimaryDistance = int.MaxValue;
        var bestPerpendicularDistance = int.MaxValue;

        foreach (var candidate in candidates)
        {
            if (!IsInDirection(source, candidate.Frame, direction))
            {
                continue;
            }

            var primaryDistance = PrimaryDistance(source, candidate.Frame, direction);
            var perpendicularDistance = PerpendicularDistance(source, candidate.Frame, direction);
            if (found &&
                (perpendicularDistance > bestPerpendicularDistance ||
                 (perpendicularDistance == bestPerpendicularDistance && primaryDistance >= bestPrimaryDistance)))
            {
                continue;
            }

            found = true;
            target = candidate.Handle;
            bestPrimaryDistance = primaryDistance;
            bestPerpendicularDistance = perpendicularDistance;
        }

        return found;
    }

    public static bool TryFindNextInStack(
        IReadOnlyList<WindowCandidate> candidates,
        IntPtr current,
        out IntPtr target)
    {
        target = IntPtr.Zero;
        if (candidates.Count == 0)
        {
            return false;
        }

        var currentIndex = -1;
        for (var i = 0; i < candidates.Count; i++)
        {
            if (candidates[i].Handle == current)
            {
                currentIndex = i;
                break;
            }
        }

        var start = currentIndex < 0 ? 0 : (currentIndex + 1) % candidates.Count;
        for (var offset = 0; offset < candidates.Count; offset++)
        {
            var candidate = candidates[(start + offset) % candidates.Count];
            if (candidate.Handle != current)
            {
                target = candidate.Handle;
                return true;
            }
        }

        return false;
    }

    private static bool IsInDirection(
        NativeMethods.Rect source,
        NativeMethods.Rect candidate,
        WindowNavigationDirection direction)
    {
        var sourceCenterX = source.Left + source.Width / 2;
        var sourceCenterY = source.Top + source.Height / 2;
        var candidateCenterX = candidate.Left + candidate.Width / 2;
        var candidateCenterY = candidate.Top + candidate.Height / 2;

        return direction switch
        {
            WindowNavigationDirection.Left => candidateCenterX < sourceCenterX,
            WindowNavigationDirection.Right => candidateCenterX > sourceCenterX,
            WindowNavigationDirection.Up => candidateCenterY < sourceCenterY,
            WindowNavigationDirection.Down => candidateCenterY > sourceCenterY,
            _ => false
        };
    }

    private static int PrimaryDistance(
        NativeMethods.Rect source,
        NativeMethods.Rect candidate,
        WindowNavigationDirection direction)
    {
        var sourceCenter = direction is WindowNavigationDirection.Left or WindowNavigationDirection.Right
            ? source.Left + source.Width / 2
            : source.Top + source.Height / 2;
        var candidateCenter = direction is WindowNavigationDirection.Left or WindowNavigationDirection.Right
            ? candidate.Left + candidate.Width / 2
            : candidate.Top + candidate.Height / 2;
        return Math.Abs(sourceCenter - candidateCenter);
    }

    private static int PerpendicularDistance(
        NativeMethods.Rect source,
        NativeMethods.Rect candidate,
        WindowNavigationDirection direction)
    {
        return direction is WindowNavigationDirection.Left or WindowNavigationDirection.Right
            ? IntervalGap(source.Top, source.Bottom, candidate.Top, candidate.Bottom)
            : IntervalGap(source.Left, source.Right, candidate.Left, candidate.Right);
    }

    private static int IntervalGap(int firstStart, int firstEnd, int secondStart, int secondEnd)
    {
        if (firstEnd >= secondStart && secondEnd >= firstStart)
        {
            return 0;
        }

        return firstEnd < secondStart ? secondStart - firstEnd : firstStart - secondEnd;
    }
}
