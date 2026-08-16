using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading;

namespace LoopW;

public enum WindowAction
{
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,

    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,

    LeftThird,
    LeftTwoThirds,
    HorizontalCenterThird,
    RightTwoThirds,
    RightThird,

    TopThird,
    TopTwoThirds,
    VerticalCenterThird,
    BottomTwoThirds,
    BottomThird,

    Center,
    Maximize,
    AlmostMaximize,
    Fullscreen,
    MaximizeHeight,
    MaximizeWidth,
    FillAvailableSpace,
    MinimizeOthers,

    HorizontalCenterHalf,
    VerticalCenterHalf,

    FirstFourth,
    SecondFourth,
    ThirdFourth,
    FourthFourth,
    LeftThreeFourths,
    RightThreeFourths,

    NextScreen,
    PreviousScreen,
    LeftScreen,
    RightScreen,
    TopScreen,
    BottomScreen,

    Larger,
    Smaller,
    ScaleUp,
    ScaleDown,
    GrowLeft,
    GrowRight,
    GrowTop,
    GrowBottom,
    GrowHorizontal,
    GrowVertical,
    ShrinkLeft,
    ShrinkRight,
    ShrinkTop,
    ShrinkBottom,
    ShrinkHorizontal,
    ShrinkVertical,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,

    FocusUp,
    FocusDown,
    FocusLeft,
    FocusRight,
    FocusNextInStack,

    Minimize,
    Stash,
    RevealStashed,
    Hide,
    RestoreInitialFrame,
    Undo
}

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
    private const int MaxUndoDepth = 20;

    private static readonly Dictionary<IntPtr, NativeMethods.Rect> InitialFrames = new();
    private static readonly Dictionary<IntPtr, Stack<NativeMethods.WindowPlacement>> UndoStacks = new();

    public static string ActionName(WindowAction action) => action switch
    {
        WindowAction.LeftHalf => "Left half",
        WindowAction.RightHalf => "Right half",
        WindowAction.TopHalf => "Top half",
        WindowAction.BottomHalf => "Bottom half",
        WindowAction.TopLeftQuarter => "Top-left quarter",
        WindowAction.TopRightQuarter => "Top-right quarter",
        WindowAction.BottomLeftQuarter => "Bottom-left quarter",
        WindowAction.BottomRightQuarter => "Bottom-right quarter",
        WindowAction.LeftThird => "Left third",
        WindowAction.LeftTwoThirds => "Left two-thirds",
        WindowAction.HorizontalCenterThird => "Horizontal center third",
        WindowAction.RightTwoThirds => "Right two-thirds",
        WindowAction.RightThird => "Right third",
        WindowAction.TopThird => "Top third",
        WindowAction.TopTwoThirds => "Top two-thirds",
        WindowAction.VerticalCenterThird => "Vertical center third",
        WindowAction.BottomTwoThirds => "Bottom two-thirds",
        WindowAction.BottomThird => "Bottom third",
        WindowAction.Center => "Center",
        WindowAction.Maximize => "Maximize",
        WindowAction.AlmostMaximize => "Almost maximize",
        WindowAction.Fullscreen => "Fullscreen",
        WindowAction.MaximizeHeight => "Maximize height",
        WindowAction.MaximizeWidth => "Maximize width",
        WindowAction.FillAvailableSpace => "Fill available space",
        WindowAction.MinimizeOthers => "Minimize other windows",
        WindowAction.HorizontalCenterHalf => "Horizontal center half",
        WindowAction.VerticalCenterHalf => "Vertical center half",
        WindowAction.FirstFourth => "First fourth",
        WindowAction.SecondFourth => "Second fourth",
        WindowAction.ThirdFourth => "Third fourth",
        WindowAction.FourthFourth => "Fourth fourth",
        WindowAction.LeftThreeFourths => "Left three-fourths",
        WindowAction.RightThreeFourths => "Right three-fourths",
        WindowAction.NextScreen => "Next screen",
        WindowAction.PreviousScreen => "Previous screen",
        WindowAction.LeftScreen => "Screen to the left",
        WindowAction.RightScreen => "Screen to the right",
        WindowAction.TopScreen => "Screen above",
        WindowAction.BottomScreen => "Screen below",
        WindowAction.Larger => "Larger",
        WindowAction.Smaller => "Smaller",
        WindowAction.ScaleUp => "Scale up",
        WindowAction.ScaleDown => "Scale down",
        WindowAction.GrowLeft => "Grow left",
        WindowAction.GrowRight => "Grow right",
        WindowAction.GrowTop => "Grow top",
        WindowAction.GrowBottom => "Grow bottom",
        WindowAction.GrowHorizontal => "Grow horizontally",
        WindowAction.GrowVertical => "Grow vertically",
        WindowAction.ShrinkLeft => "Shrink left",
        WindowAction.ShrinkRight => "Shrink right",
        WindowAction.ShrinkTop => "Shrink top",
        WindowAction.ShrinkBottom => "Shrink bottom",
        WindowAction.ShrinkHorizontal => "Shrink horizontally",
        WindowAction.ShrinkVertical => "Shrink vertically",
        WindowAction.MoveLeft => "Move left",
        WindowAction.MoveRight => "Move right",
        WindowAction.MoveUp => "Move up",
        WindowAction.MoveDown => "Move down",
        WindowAction.FocusUp => "Focus up",
        WindowAction.FocusDown => "Focus down",
        WindowAction.FocusLeft => "Focus left",
        WindowAction.FocusRight => "Focus right",
        WindowAction.FocusNextInStack => "Focus next in stack",
        WindowAction.Minimize => "Minimize",
        WindowAction.Stash => "Stash at edge",
        WindowAction.RevealStashed => "Reveal stashed window",
        WindowAction.Hide => "Hide",
        WindowAction.RestoreInitialFrame => "Restore original frame",
        WindowAction.Undo => "Undo",
        _ => action.ToString()
    };

    public static bool TryGetHalfFrame(IntPtr window, WindowHalf half, out NativeMethods.Rect target, out string error)
    {
        var action = half switch
        {
            WindowHalf.Left => WindowAction.LeftHalf,
            WindowHalf.Right => WindowAction.RightHalf,
            WindowHalf.Top => WindowAction.TopHalf,
            WindowHalf.Bottom => WindowAction.BottomHalf,
            _ => WindowAction.LeftHalf
        };
        return TryGetTargetFrame(window, action, out target, out error);
    }

    public static bool ApplyHalf(IntPtr window, WindowHalf half, out string message)
    {
        var action = half switch
        {
            WindowHalf.Left => WindowAction.LeftHalf,
            WindowHalf.Right => WindowAction.RightHalf,
            WindowHalf.Top => WindowAction.TopHalf,
            WindowHalf.Bottom => WindowAction.BottomHalf,
            _ => WindowAction.LeftHalf
        };
        return TryApply(window, action, out message);
    }

    /// <summary>
    /// Applies any window action to the target window and returns a status message
    /// describing what actually happened. Works for any window regardless of its
    /// current state (maximized, minimized) or its min/max size constraints.
    /// </summary>
    public static bool TryApply(IntPtr window, WindowAction action, out string message)
    {
        message = string.Empty;

        if (action == WindowAction.RevealStashed)
        {
            return WindowStashService.TryRevealNext(out message);
        }

        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            message = "The target window is no longer available.";
            return false;
        }

        switch (action)
        {
            case WindowAction.Minimize:
                return ShowAndReport(window, NativeMethods.SwMinimize, ActionName(action), out message);
            case WindowAction.MinimizeOthers:
                return MinimizeOtherWindows(window, out message);
            case WindowAction.FocusUp:
            case WindowAction.FocusDown:
            case WindowAction.FocusLeft:
            case WindowAction.FocusRight:
                return FocusDirectionalWindow(window, action, out message);
            case WindowAction.FocusNextInStack:
                return FocusNextWindow(window, out message);
            case WindowAction.Stash:
                return WindowStashService.TryStash(window, out message);
            case WindowAction.Hide:
                return ShowAndReport(window, NativeMethods.SwHide, ActionName(action), out message);
            case WindowAction.RestoreInitialFrame:
                return RestoreInitialFrame(window, out message);
            case WindowAction.Undo:
                return Undo(window, out message);
        }

        if (!TryGetTargetFrame(window, action, out var ideal, out message))
        {
            return false;
        }

        if (!TryGetMonitorBounds(ideal, out _, out var work))
        {
            message = "Could not determine the target monitor.";
            return false;
        }

        var bounds = action == WindowAction.Fullscreen ? GetMonitorBounds(ideal) : work;

        PushUndo(window);

        var frame = WindowFrameMath.FitFrame(bounds, action, ideal, GetMinMaxInfo(window));

        if (!PlaceWindow(window, frame))
        {
            message = "Windows rejected the move. The target may be elevated or non-resizable.";
            return false;
        }

        if (!NativeMethods.GetWindowRect(window, out var actual))
        {
            message = "Could not read the window's final position.";
            return false;
        }

        // Some apps clamp to their min size without honoring the zone anchor. Read the
        // settled result and re-anchor once so the window stays fully on-screen.
        if (!RectsEqual(actual, frame))
        {
            var reanchored = WindowFrameMath.FitFrame(bounds, action, actual, new NativeMethods.MinMaxInfo());
            if (!RectsEqual(reanchored, frame))
            {
                PlaceWindow(window, reanchored);
                NativeMethods.GetWindowRect(window, out actual);
            }
        }

        var label = ActionName(action);
        message = SizesEqual(actual, ideal)
            ? $"Applied {label} to target window"
            : $"Snapped to {label}, but the window's minimum/maximum size forced {actual.Width}\u00d7{actual.Height} instead of {ideal.Width}\u00d7{ideal.Height}.";

        return true;
    }

    internal static bool TryApplySnap(IntPtr window, DragSnapTarget target, out string message)
    {
        message = string.Empty;
        if (window == IntPtr.Zero || !WindowQuery.IsEligibleForSnap(window))
        {
            message = "The dragged window is no longer available.";
            return false;
        }

        var ideal = target.Frame;
        if (!NativeMethods.TryGetMonitorWorkRect(ideal, out var work))
        {
            message = "Could not determine the snap monitor.";
            return false;
        }

        var frame = WindowFrameMath.FitFrame(work, target.Action, ideal, GetMinMaxInfo(window));
        PushUndo(window);
        if (!PlaceWindow(window, frame))
        {
            message = "Windows rejected the snap. The target may be elevated or non-resizable.";
            return false;
        }

        if (!NativeMethods.GetWindowRect(window, out var actual))
        {
            message = "Could not read the snapped window's final position.";
            return false;
        }

        if (!RectsEqual(actual, frame))
        {
            var reanchored = WindowFrameMath.FitFrame(work, target.Action, actual, new NativeMethods.MinMaxInfo());
            if (!RectsEqual(reanchored, frame))
            {
                PlaceWindow(window, reanchored);
                NativeMethods.GetWindowRect(window, out actual);
            }
        }

        var label = ActionName(target.Action);
        message = SizesEqual(actual, ideal)
            ? $"Applied drag snap: {label}"
            : $"Snapped to {label}, but the window's minimum/maximum size forced {actual.Width}\u00d7{actual.Height} instead of {ideal.Width}\u00d7{ideal.Height}.";
        return true;
    }

    internal static bool TryRestoreFrame(IntPtr window, NativeMethods.Rect frame, out string message)
    {
        message = string.Empty;
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            message = "The dragged window is no longer available.";
            return false;
        }

        if (!PlaceWindow(window, frame) ||
            !NativeMethods.GetWindowRect(window, out var actual))
        {
            message = "Could not restore the pre-drag frame.";
            return false;
        }

        message = RectsEqual(actual, frame)
            ? "Restored the pre-drag frame"
            : "The application did not accept the pre-drag frame.";
        return RectsEqual(actual, frame);
    }

    /// <summary>
    /// Computes the ideal frame for an action without applying anything. Used by the
    /// preview overlay and by tests.
    /// </summary>
    public static bool TryGetTargetFrame(IntPtr window, WindowAction action, out NativeMethods.Rect target, out string error)
    {
        target = default;
        error = string.Empty;

        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            error = "The target window is no longer available.";
            return false;
        }

        if (!TryGetMonitorInfo(window, out var monitorRect, out var work))
        {
            error = "Could not determine the target monitor.";
            return false;
        }

        switch (action)
        {
            case WindowAction.LeftHalf:
            case WindowAction.RightHalf:
            case WindowAction.TopHalf:
            case WindowAction.BottomHalf:
            case WindowAction.TopLeftQuarter:
            case WindowAction.TopRightQuarter:
            case WindowAction.BottomLeftQuarter:
            case WindowAction.BottomRightQuarter:
            case WindowAction.HorizontalCenterHalf:
            case WindowAction.VerticalCenterHalf:
            case WindowAction.FirstFourth:
            case WindowAction.SecondFourth:
            case WindowAction.ThirdFourth:
            case WindowAction.FourthFourth:
            case WindowAction.LeftThreeFourths:
            case WindowAction.RightThreeFourths:
            case WindowAction.LeftThird:
            case WindowAction.LeftTwoThirds:
            case WindowAction.HorizontalCenterThird:
            case WindowAction.RightTwoThirds:
            case WindowAction.RightThird:
            case WindowAction.TopThird:
            case WindowAction.TopTwoThirds:
            case WindowAction.VerticalCenterThird:
            case WindowAction.BottomTwoThirds:
            case WindowAction.BottomThird:
                target = WindowFrameMath.ZoneFrame(work, action);
                return true;

            case WindowAction.Maximize:
                target = work;
                return true;

            case WindowAction.MaximizeHeight:
                target = WindowFrameMath.MaximizeHeightFrame(work, GetCurrentRect(window));
                return true;

            case WindowAction.MaximizeWidth:
                target = WindowFrameMath.MaximizeWidthFrame(work, GetCurrentRect(window));
                return true;

            case WindowAction.FillAvailableSpace:
                target = WindowFrameMath.FillAvailableFrame(
                    work,
                    GetCurrentRect(window),
                    WindowQuery.GetLayoutWindows(window));
                return true;

            case WindowAction.Fullscreen:
                target = monitorRect;
                return true;

            case WindowAction.Center:
                target = WindowFrameMath.CenterFrame(work, GetCurrentRect(window));
                return true;

            case WindowAction.AlmostMaximize:
                var margin = (int)Math.Round(12 * DpiScale(window));
                target = new NativeMethods.Rect
                {
                    Left = work.Left + margin,
                    Top = work.Top + margin,
                    Right = work.Right - margin,
                    Bottom = work.Bottom - margin
                };
                return true;

            case WindowAction.NextScreen:
            case WindowAction.PreviousScreen:
                return TryScreenFrame(window, action, work, out target, out error);

            case WindowAction.LeftScreen:
            case WindowAction.RightScreen:
            case WindowAction.TopScreen:
            case WindowAction.BottomScreen:
                return TryDirectionalScreenFrame(window, action, work, out target, out error);

            case WindowAction.Larger:
            case WindowAction.Smaller:
            case WindowAction.ScaleUp:
            case WindowAction.ScaleDown:
            case WindowAction.GrowLeft:
            case WindowAction.GrowRight:
            case WindowAction.GrowTop:
            case WindowAction.GrowBottom:
            case WindowAction.GrowHorizontal:
            case WindowAction.GrowVertical:
            case WindowAction.ShrinkLeft:
            case WindowAction.ShrinkRight:
            case WindowAction.ShrinkTop:
            case WindowAction.ShrinkBottom:
            case WindowAction.ShrinkHorizontal:
            case WindowAction.ShrinkVertical:
            case WindowAction.MoveLeft:
            case WindowAction.MoveRight:
            case WindowAction.MoveUp:
            case WindowAction.MoveDown:
                target = WindowFrameMath.ManipulateFrame(work, action, GetCurrentRect(window), DpiScale(window));
                return true;

            default:
                error = $"Unsupported action: {ActionName(action)}";
                return false;
        }
    }

    private static bool ShowAndReport(IntPtr window, int command, string label, out string message)
    {
        if (!NativeMethods.ShowWindow(window, command))
        {
            message = "The target window rejected the command.";
            return false;
        }

        message = $"Applied {label} to target window";
        return true;
    }

    private static bool MinimizeOtherWindows(IntPtr targetWindow, out string message)
    {
        var minimized = 0;
        foreach (var candidate in WindowQuery.GetMinimizableWindows(targetWindow))
        {
            NativeMethods.ShowWindow(candidate.Handle, NativeMethods.SwMinimize);
            if (NativeMethods.IsIconic(candidate.Handle))
            {
                minimized++;
            }
        }

        if (minimized == 0)
        {
            message = "No other eligible windows were found.";
            return false;
        }

        message = $"Minimized {minimized} other window{(minimized == 1 ? string.Empty : "s")}";
        return true;
    }

    private static bool FocusDirectionalWindow(IntPtr currentWindow, WindowAction action, out string message)
    {
        var direction = action switch
        {
            WindowAction.FocusLeft => WindowNavigationDirection.Left,
            WindowAction.FocusRight => WindowNavigationDirection.Right,
            WindowAction.FocusUp => WindowNavigationDirection.Up,
            WindowAction.FocusDown => WindowNavigationDirection.Down,
            _ => throw new ArgumentOutOfRangeException(nameof(action), action, "Not a directional focus action.")
        };

        if (!NativeMethods.GetWindowRect(currentWindow, out var currentFrame) ||
            !WindowNavigation.TryFindDirectional(
                currentFrame,
                WindowQuery.GetFocusableWindows(currentWindow),
                direction,
                out var targetWindow))
        {
            message = $"No eligible window found {ActionName(action)[6..].ToLowerInvariant()}.";
            return false;
        }

        return FocusWindow(targetWindow, ActionName(action), out message);
    }

    private static bool FocusNextWindow(IntPtr currentWindow, out string message)
    {
        if (!WindowNavigation.TryFindNextInStack(
                WindowQuery.GetFocusableWindows(currentWindow),
                currentWindow,
                out var targetWindow))
        {
            message = "No other eligible window was found in the window stack.";
            return false;
        }

        return FocusWindow(targetWindow, ActionName(WindowAction.FocusNextInStack), out message);
    }

    private static bool FocusWindow(IntPtr targetWindow, string actionLabel, out string message)
    {
        if (NativeMethods.IsIconic(targetWindow))
        {
            NativeMethods.ShowWindow(targetWindow, NativeMethods.SwRestore);
        }

        if (!NativeMethods.SetForegroundWindow(targetWindow))
        {
            message = "Windows rejected the focus request.";
            return false;
        }

        message = $"Applied {actionLabel} to target window";
        return true;
    }

    private static bool Undo(IntPtr window, out string message)
    {
        message = string.Empty;
        if (!UndoStacks.TryGetValue(window, out var stack) || stack.Count == 0)
        {
            message = "No previous placement to undo.";
            return false;
        }

        var placement = stack.Pop();
        if (!NativeMethods.SetWindowPlacement(window, ref placement))
        {
            message = "Could not restore the previous placement.";
            return false;
        }

        message = "Restored the previous placement";
        return true;
    }

    private static bool RestoreInitialFrame(IntPtr window, out string message)
    {
        message = string.Empty;
        if (!InitialFrames.TryGetValue(window, out var frame))
        {
            message = "No original frame has been recorded for this window.";
            return false;
        }

        PushUndo(window);

        if (!PlaceWindow(window, frame))
        {
            message = "Windows rejected the restore. The target may be elevated or non-resizable.";
            return false;
        }

        message = "Restored the window's original frame";
        return true;
    }

    private static void PushUndo(IntPtr window)
    {
        var placement = new NativeMethods.WindowPlacement { Length = Marshal.SizeOf<NativeMethods.WindowPlacement>() };
        if (!NativeMethods.GetWindowPlacement(window, ref placement))
        {
            return;
        }

        if (!InitialFrames.ContainsKey(window))
        {
            InitialFrames[window] = placement.NormalPosition;
        }

        if (!UndoStacks.TryGetValue(window, out var stack))
        {
            stack = new Stack<NativeMethods.WindowPlacement>();
            UndoStacks[window] = stack;
        }

        stack.Push(placement);

        // Trim the oldest entries to keep memory bounded.
        if (stack.Count > MaxUndoDepth)
        {
            var items = stack.ToArray();
            stack.Clear();
            for (var i = MaxUndoDepth - 1; i >= 0; i--)
            {
                stack.Push(items[i]);
            }
        }
    }

    private static bool TryScreenFrame(IntPtr window, WindowAction action, NativeMethods.Rect currentWork, out NativeMethods.Rect target, out string error)
    {
        target = default;
        error = string.Empty;

        var monitors = GetMonitorWorkAreas();
        if (monitors.Count < 2)
        {
            error = "Only one monitor is connected.";
            return false;
        }

        var index = monitors.FindIndex(r => RectsEqual(r, currentWork));
        if (index < 0)
        {
            index = 0;
        }

        var next = action == WindowAction.NextScreen
            ? monitors[(index + 1) % monitors.Count]
            : monitors[(index - 1 + monitors.Count) % monitors.Count];

        target = TranslateFrame(GetCurrentRect(window), currentWork, next);
        return true;
    }

    private static bool TryDirectionalScreenFrame(IntPtr window, WindowAction action, NativeMethods.Rect currentWork, out NativeMethods.Rect target, out string error)
    {
        target = default;
        error = string.Empty;

        var (dirX, dirY) = action switch
        {
            WindowAction.LeftScreen => (-1, 0),
            WindowAction.RightScreen => (1, 0),
            WindowAction.TopScreen => (0, -1),
            WindowAction.BottomScreen => (0, 1),
            _ => (0, 0)
        };

        var curCenterX = currentWork.Left + currentWork.Width / 2;
        var curCenterY = currentWork.Top + currentWork.Height / 2;
        var best = currentWork;
        var bestScore = double.NegativeInfinity;

        foreach (var monitor in GetMonitorWorkAreas())
        {
            if (RectsEqual(monitor, currentWork))
            {
                continue;
            }

            var dx = monitor.Left + monitor.Width / 2 - curCenterX;
            var dy = monitor.Top + monitor.Height / 2 - curCenterY;
            var score = dx * dirX + dy * dirY;

            // Only move to a monitor that actually lies in the requested
            // direction; otherwise the best (least-negative) candidate would be
            // on the opposite side, e.g. LeftScreen on the leftmost display.
            if (score > 0 && score > bestScore)
            {
                bestScore = score;
                best = monitor;
            }
        }

        // No monitor lies in the requested direction (e.g. LeftScreen on the
        // leftmost display): report a no-op instead of claiming success so the
        // caller does not push a redundant entry onto the undo history.
        if (RectsEqual(best, currentWork))
        {
            error = "No monitor in that direction.";
            return false;
        }

        target = TranslateFrame(GetCurrentRect(window), currentWork, best);
        return true;
    }

    private static NativeMethods.Rect TranslateFrame(NativeMethods.Rect current, NativeMethods.Rect from, NativeMethods.Rect to)
    {
        var left = to.Left + (current.Left - from.Left);
        var top = to.Top + (current.Top - from.Top);
        return Rect(left, top, left + current.Width, top + current.Height);
    }

    private static NativeMethods.Rect GetCurrentRect(IntPtr window)
    {
        NativeMethods.GetWindowRect(window, out var rect);
        return rect;
    }

    private static double DpiScale(IntPtr window)
    {
        var monitor = NativeMethods.MonitorFromWindow(window, NativeMethods.MonitorDefaultToNearest);
        if (NativeMethods.TryGetMonitorDpi(monitor, out var dpiX, out _))
        {
            return dpiX / 96.0;
        }

        return 1.0;
    }

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

    private static bool TryGetMonitorInfo(IntPtr window, out NativeMethods.Rect monitorRect, out NativeMethods.Rect work)
    {
        monitorRect = default;
        work = default;
        var monitor = NativeMethods.MonitorFromWindow(window, NativeMethods.MonitorDefaultToNearest);
        var info = new NativeMethods.MonitorInfo { Size = Marshal.SizeOf<NativeMethods.MonitorInfo>() };
        if (monitor == IntPtr.Zero || !NativeMethods.GetMonitorInfo(monitor, ref info))
        {
            return false;
        }

        monitorRect = info.Monitor;
        work = info.Work;
        return true;
    }

    private static bool TryGetMonitorBounds(NativeMethods.Rect rect, out NativeMethods.Rect monitorRect, out NativeMethods.Rect work)
    {
        monitorRect = default;
        work = default;
        var monitor = NativeMethods.MonitorFromRect(ref rect, NativeMethods.MonitorDefaultToNearest);
        var info = new NativeMethods.MonitorInfo { Size = Marshal.SizeOf<NativeMethods.MonitorInfo>() };
        if (monitor == IntPtr.Zero || !NativeMethods.GetMonitorInfo(monitor, ref info))
        {
            return false;
        }

        monitorRect = info.Monitor;
        work = info.Work;
        return true;
    }

    private static NativeMethods.Rect GetMonitorBounds(NativeMethods.Rect rect)
    {
        TryGetMonitorBounds(rect, out var monitorRect, out _);
        return monitorRect;
    }

    private static List<NativeMethods.Rect> GetMonitorWorkAreas()
    {
        var monitors = new List<NativeMethods.Rect>();
        NativeMethods.EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (monitor, hdc, rect, data) =>
        {
            var info = new NativeMethods.MonitorInfo { Size = Marshal.SizeOf<NativeMethods.MonitorInfo>() };
            if (NativeMethods.GetMonitorInfo(monitor, ref info))
            {
                monitors.Add(info.Work);
            }

            return true;
        }, IntPtr.Zero);
        monitors.Sort((a, b) => a.Left != b.Left ? a.Left.CompareTo(b.Left) : a.Top.CompareTo(b.Top));
        return monitors;
    }

    private static NativeMethods.Rect Rect(int left, int top, int right, int bottom) =>
        new() { Left = left, Top = top, Right = right, Bottom = bottom };

    private static bool RectsEqual(NativeMethods.Rect a, NativeMethods.Rect b) =>
        Math.Abs(a.Left - b.Left) <= PlaceTolerance &&
        Math.Abs(a.Top - b.Top) <= PlaceTolerance &&
        Math.Abs(a.Width - b.Width) <= PlaceTolerance &&
        Math.Abs(a.Height - b.Height) <= PlaceTolerance;

    private static bool SizesEqual(NativeMethods.Rect a, NativeMethods.Rect b) =>
        Math.Abs(a.Width - b.Width) <= PlaceTolerance &&
        Math.Abs(a.Height - b.Height) <= PlaceTolerance;
}
