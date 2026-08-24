using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Collections.Generic;

namespace LoopW;

internal enum WindowRestriction
{
    None,
    Invalid,
    LoopW,
    Hidden,
    Child,
    Tool,
    Owned,
    Excluded,
    BorderlessFullscreen
}

internal readonly record struct WindowPolicyDecision(
    bool IsAllowed,
    string Diagnostic,
    WindowRestriction Restriction,
    bool IsResizable,
    bool IsBorderlessFullscreen);

internal static class WindowPolicy
{
    private static AppSettings _settings = new();
    private static HashSet<string> _excludedExecutablePaths = new(StringComparer.OrdinalIgnoreCase);
    private static HashSet<string> _excludedProcessNames = new(StringComparer.OrdinalIgnoreCase);

    public static void Configure(AppSettings settings)
    {
        _settings = settings;
        RebuildExclusionLookups(settings);
    }

    public static void UpdateSettings(AppSettings settings)
    {
        _settings = settings;
        RebuildExclusionLookups(settings);
    }

    public static bool TryAuthorizeAction(
        IntPtr window,
        WindowAction action,
        out string diagnostic)
    {
        var decision = Evaluate(window);
        if (!decision.IsAllowed)
        {
            diagnostic = decision.Diagnostic;
            return false;
        }

        if (decision.IsBorderlessFullscreen && !AllowsBorderlessAction(action))
        {
            diagnostic = "The target is borderless fullscreen. Exit fullscreen in the app before applying a layout.";
            return false;
        }

        if (!decision.IsResizable && RequiresResize(action))
        {
            diagnostic = "The target window is non-resizable, so this layout action was skipped.";
            return false;
        }

        diagnostic = string.Empty;
        return true;
    }

    public static bool IsEligibleForEnumeration(IntPtr window, IntPtr excludedWindow)
    {
        if (window == excludedWindow)
        {
            return false;
        }

        var decision = Evaluate(window);
        return decision.IsAllowed && !decision.IsBorderlessFullscreen;
    }

    public static bool IsExcluded(IntPtr window) => Evaluate(window).Restriction == WindowRestriction.Excluded;

    private static WindowPolicyDecision Evaluate(IntPtr window)
    {
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window))
        {
            return Denied(WindowRestriction.Invalid, "The target window is no longer available.");
        }

        if (!NativeMethods.IsWindowVisible(window))
        {
            return Denied(WindowRestriction.Hidden, "The target window is hidden.");
        }

        NativeMethods.GetWindowThreadProcessId(window, out var processId);
        if (processId == 0 || processId == Environment.ProcessId)
        {
            return Denied(WindowRestriction.LoopW, "LoopW windows are not action targets.");
        }

        var style = NativeMethods.GetWindowLongPtr(window, NativeMethods.GwlStyle).ToInt64();
        var extendedStyle = NativeMethods.GetWindowLongPtr(window, NativeMethods.GwlExStyle).ToInt64();
        if ((style & NativeMethods.WsChild) != 0)
        {
            return Denied(WindowRestriction.Child, "Child windows are not independent action targets.");
        }

        if ((extendedStyle & NativeMethods.WsExToolWindow) != 0)
        {
            return Denied(WindowRestriction.Tool, "Tool windows are not action targets.");
        }

        if (NativeMethods.GetWindow(window, NativeMethods.GwOwner) != IntPtr.Zero)
        {
            return Denied(WindowRestriction.Owned, "Owned utility windows are not action targets.");
        }

        if (IsExcludedBySettings(processId))
        {
            return Denied(WindowRestriction.Excluded, "The target application is excluded in LoopW settings.");
        }

        var isResizable = (style & NativeMethods.WsThickFrame) != 0;
        var isBorderlessFullscreen = IsBorderlessFullscreen(window, style);
        return new WindowPolicyDecision(
            true,
            string.Empty,
            isBorderlessFullscreen ? WindowRestriction.BorderlessFullscreen : WindowRestriction.None,
            isResizable,
            isBorderlessFullscreen);
    }

    private static bool IsExcludedBySettings(uint processId)
    {
        if (_excludedExecutablePaths.Count == 0 && _excludedProcessNames.Count == 0)
        {
            return false;
        }

        try
        {
            using var process = Process.GetProcessById((int)processId);
            string? path = null;
            try
            {
                path = process.MainModule?.FileName;
            }
            catch (Win32Exception)
            {
                // Protected or elevated processes can deny module inspection.
            }

            string? processName = null;
            try
            {
                processName = process.ProcessName;
            }
            catch (Win32Exception)
            {
                // The path check above remains useful when the name is denied.
            }

            return (!string.IsNullOrWhiteSpace(path) && _excludedExecutablePaths.Contains(path)) ||
                (!string.IsNullOrWhiteSpace(processName) && _excludedProcessNames.Contains(processName));
        }
        catch (Win32Exception)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (ArgumentException)
        {
            return false;
        }
    }

    private static void RebuildExclusionLookups(AppSettings settings)
    {
        _excludedExecutablePaths = new HashSet<string>(
            settings.ExcludedExecutablePaths ?? new List<string>(),
            StringComparer.OrdinalIgnoreCase);
        _excludedProcessNames = new HashSet<string>(
            settings.ExcludedProcessNames ?? new List<string>(),
            StringComparer.OrdinalIgnoreCase);
    }

    private static bool IsBorderlessFullscreen(IntPtr window, long style)
    {
        if ((style & NativeMethods.WsCaption) != 0 ||
            !TryGetVisibleFrame(window, out var frame) ||
            !MonitorService.TryGetForWindow(window, out var monitor))
        {
            return false;
        }

        return NearlyEqual(frame, monitor.Monitor);
    }

    private static bool TryGetVisibleFrame(IntPtr window, out NativeMethods.Rect frame)
    {
        if (NativeMethods.TryGetVisibleWindowRect(window, out frame))
        {
            return true;
        }

        return NativeMethods.GetWindowRect(window, out frame);
    }

    private static bool AllowsBorderlessAction(WindowAction action) => action switch
    {
        WindowAction.Minimize or
        WindowAction.Hide or
        WindowAction.FocusUp or
        WindowAction.FocusDown or
        WindowAction.FocusLeft or
        WindowAction.FocusRight or
        WindowAction.FocusNextInStack or
        WindowAction.MinimizeOthers or
        WindowAction.RestoreInitialFrame or
        WindowAction.Undo => true,
        _ => false
    };

    private static bool RequiresResize(WindowAction action) => action switch
    {
        WindowAction.NextScreen or
        WindowAction.PreviousScreen or
        WindowAction.LeftScreen or
        WindowAction.RightScreen or
        WindowAction.TopScreen or
        WindowAction.BottomScreen or
        WindowAction.MoveLeft or
        WindowAction.MoveRight or
        WindowAction.MoveUp or
        WindowAction.MoveDown or
        WindowAction.Center or
        WindowAction.Minimize or
        WindowAction.Hide or
        WindowAction.FocusUp or
        WindowAction.FocusDown or
        WindowAction.FocusLeft or
        WindowAction.FocusRight or
        WindowAction.FocusNextInStack or
        WindowAction.RestoreInitialFrame or
        WindowAction.Undo => false,
        _ => true
    };

    private static WindowPolicyDecision Denied(WindowRestriction restriction, string diagnostic) =>
        new(false, diagnostic, restriction, false, false);

    private static bool NearlyEqual(NativeMethods.Rect left, NativeMethods.Rect right) =>
        Math.Abs(left.Left - right.Left) <= 2 &&
        Math.Abs(left.Top - right.Top) <= 2 &&
        Math.Abs(left.Right - right.Right) <= 2 &&
        Math.Abs(left.Bottom - right.Bottom) <= 2;
}
