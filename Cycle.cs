using System;
using System.Collections.Generic;

namespace LoopW;

/// <summary>
/// Describes the action selected for one cycle invocation. The cursor is only
/// committed after the window action succeeds, so a failed placement does not
/// consume a step.
/// </summary>
internal readonly record struct CycleSelection(
    WindowAction RequestedAction,
    WindowAction EffectiveAction,
    int Position,
    int Count,
    bool IsCycle)
{
    public string StatusSuffix => IsCycle ? $"  ·  Cycle {Position + 1}/{Count}" : string.Empty;
}

internal static class WindowCycleService
{
    private sealed record Cursor(WindowAction RequestedAction, int Position);

    private static readonly Dictionary<IntPtr, Cursor> Cursors = new();

    public static bool CanCycle(WindowAction action) => TryGetChain(action, out _);

    public static CycleSelection Select(IntPtr window, WindowAction requestedAction, bool enabled)
    {
        if (!enabled || !TryGetChain(requestedAction, out var chain))
        {
            Cursors.Remove(window);
            return new CycleSelection(requestedAction, requestedAction, 0, 0, false);
        }

        var requestedPosition = PositionOf(chain, requestedAction);
        if (requestedPosition < 0)
        {
            requestedPosition = 0;
        }

        var position = requestedPosition;
        if (Cursors.TryGetValue(window, out var cursor) && cursor.RequestedAction == requestedAction)
        {
            position = (cursor.Position + 1) % chain.Count;
        }

        return new CycleSelection(requestedAction, chain[position], position, chain.Count, true);
    }

    public static void Commit(IntPtr window, CycleSelection selection)
    {
        if (selection.IsCycle)
        {
            Cursors[window] = new Cursor(selection.RequestedAction, selection.Position);
        }
    }

    private static int PositionOf(IReadOnlyList<WindowAction> chain, WindowAction action)
    {
        for (var i = 0; i < chain.Count; i++)
        {
            if (chain[i] == action)
            {
                return i;
            }
        }

        return -1;
    }

    private static bool TryGetChain(WindowAction action, out IReadOnlyList<WindowAction> chain)
    {
        chain = action switch
        {
            WindowAction.LeftHalf or WindowAction.LeftThird or WindowAction.LeftTwoThirds =>
                new[] { WindowAction.LeftHalf, WindowAction.LeftThird, WindowAction.LeftTwoThirds },
            WindowAction.RightHalf or WindowAction.RightThird or WindowAction.RightTwoThirds =>
                new[] { WindowAction.RightHalf, WindowAction.RightThird, WindowAction.RightTwoThirds },
            WindowAction.TopHalf or WindowAction.TopThird or WindowAction.TopTwoThirds =>
                new[] { WindowAction.TopHalf, WindowAction.TopThird, WindowAction.TopTwoThirds },
            WindowAction.BottomHalf or WindowAction.BottomThird or WindowAction.BottomTwoThirds =>
                new[] { WindowAction.BottomHalf, WindowAction.BottomThird, WindowAction.BottomTwoThirds },
            _ => Array.Empty<WindowAction>()
        };

        return chain.Count > 0;
    }
}
