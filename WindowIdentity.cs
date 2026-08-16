using System;
using System.Collections.Generic;

namespace LoopW;

internal readonly record struct WindowIdentity(
    string ExecutablePath,
    uint ProcessId,
    string WindowClass,
    string Title);

internal readonly record struct WindowIdentityCandidate<T>(T Value, WindowIdentity Identity);

internal static class WindowIdentityMatcher
{
    public static bool TryFindUnambiguousMatch<T>(
        StashRecord record,
        IReadOnlyList<WindowIdentityCandidate<T>> candidates,
        out T match)
    {
        match = default!;
        var found = false;

        foreach (var candidate in candidates)
        {
            if (!IsMatch(record, candidate.Identity))
            {
                continue;
            }

            if (found)
            {
                match = default!;
                return false;
            }

            found = true;
            match = candidate.Value;
        }

        return found;
    }

    public static bool IsMatch(StashRecord record, WindowIdentity candidate)
    {
        if (string.IsNullOrWhiteSpace(record.ExecutablePath) ||
            !string.Equals(record.ExecutablePath, candidate.ExecutablePath, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var matchingHints = 0;
        if (record.ProcessId != 0 && record.ProcessId == candidate.ProcessId)
        {
            matchingHints++;
        }

        if (!string.IsNullOrWhiteSpace(record.WindowClass) &&
            string.Equals(record.WindowClass, candidate.WindowClass, StringComparison.OrdinalIgnoreCase))
        {
            matchingHints++;
        }

        if (!string.IsNullOrEmpty(record.Title) &&
            string.Equals(record.Title, candidate.Title, StringComparison.Ordinal))
        {
            matchingHints++;
        }

        // The executable path is the anchor. Require two independent hints in
        // addition to it so a reused process ID or a generic window title cannot
        // restore an unrelated window on its own.
        return matchingHints >= 2;
    }

    public static bool IsSameRuntimeWindow(StashRecord record, WindowIdentity candidate)
    {
        if (record.ProcessId == 0 || record.ProcessId != candidate.ProcessId ||
            !string.Equals(record.ExecutablePath, candidate.ExecutablePath, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        return !string.IsNullOrWhiteSpace(record.WindowClass) &&
            string.Equals(record.WindowClass, candidate.WindowClass, StringComparison.OrdinalIgnoreCase);
    }
}
