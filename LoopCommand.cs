using System.Globalization;

namespace LoopW;

internal abstract record LoopCommand
{
    internal sealed record Activate : LoopCommand;

    internal sealed record Apply(WindowAction Action) : LoopCommand;

    internal sealed record ListActions : LoopCommand;

    internal sealed record ListKeybinds : LoopCommand;

    internal sealed record ListAll : LoopCommand;
}

internal static class LoopCommandParser
{
    private static readonly Dictionary<string, WindowAction> DirectionAliases = new(StringComparer.OrdinalIgnoreCase)
    {
        ["left"] = WindowAction.LeftHalf,
        ["right"] = WindowAction.RightHalf,
        ["top"] = WindowAction.TopHalf,
        ["bottom"] = WindowAction.BottomHalf,
        ["next"] = WindowAction.NextScreen,
        ["previous"] = WindowAction.PreviousScreen,
        ["prev"] = WindowAction.PreviousScreen,
        ["left-screen"] = WindowAction.LeftScreen,
        ["right-screen"] = WindowAction.RightScreen,
        ["top-screen"] = WindowAction.TopScreen,
        ["bottom-screen"] = WindowAction.BottomScreen
    };

    public static bool TryParse(string? raw, out LoopCommand? command, out string error)
    {
        command = null;
        error = string.Empty;

        var parts = (raw ?? string.Empty).Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (parts.Length != 1)
        {
            error = "Expected one command, such as direction/right or list/actions.";
            return false;
        }

        var token = parts[0];
        if (token.Equals("activate", StringComparison.OrdinalIgnoreCase))
        {
            command = new LoopCommand.Activate();
            return true;
        }

        if (token.Equals("list/actions", StringComparison.OrdinalIgnoreCase))
        {
            command = new LoopCommand.ListActions();
            return true;
        }

        if (token.Equals("list/keybinds", StringComparison.OrdinalIgnoreCase))
        {
            command = new LoopCommand.ListKeybinds();
            return true;
        }

        if (token.Equals("list/all", StringComparison.OrdinalIgnoreCase))
        {
            command = new LoopCommand.ListAll();
            return true;
        }

        if (token.StartsWith("direction/", StringComparison.OrdinalIgnoreCase))
        {
            var direction = token["direction/".Length..];
            if (DirectionAliases.TryGetValue(direction, out var directionAction))
            {
                command = new LoopCommand.Apply(directionAction);
                return true;
            }

            error = $"Unknown direction '{direction}'. Use left, right, top, bottom, next, previous, or a directional screen command.";
            return false;
        }

        if (token.StartsWith("action/", StringComparison.OrdinalIgnoreCase))
        {
            var actionName = token["action/".Length..];
            if (TryParseAction(actionName, out var action))
            {
                command = new LoopCommand.Apply(action);
                return true;
            }

            error = $"Unknown action '{actionName}'. Use list/actions to see valid actions.";
            return false;
        }

        error = $"Unknown command '{token}'. Use direction/<name>, action/<name>, or list/<scope>.";
        return false;
    }

    internal static bool TryParseAction(string value, out WindowAction action)
    {
        var normalized = Normalize(value);
        foreach (var candidate in Enum.GetValues<WindowAction>())
        {
            if (Normalize(candidate.ToString()) == normalized)
            {
                action = candidate;
                return true;
            }
        }

        action = default;
        return false;
    }

    internal static string Normalize(string value) =>
        value.Replace("-", string.Empty, StringComparison.Ordinal)
            .Replace("_", string.Empty, StringComparison.Ordinal)
            .ToLower(CultureInfo.InvariantCulture);
}

internal static class LoopCommandFormatter
{
    internal static string Actions() => string.Join(
        Environment.NewLine,
        Enum.GetValues<WindowAction>().Select(action => $"action/{ActionToken(action)} - {WindowActionService.ActionName(action)}"));

    internal static string Keybinds(
        IReadOnlyList<Keybind> keybinds,
        uint triggerModifiers,
        uint triggerVk,
        TriggerModifierSide triggerSide = TriggerModifierSide.Any)
    {
        var lines = new List<string> { $"trigger: {HotkeyNames.For(triggerModifiers, triggerVk, triggerSide)}" };
        if (keybinds.Count == 0)
        {
            lines.Add("keybinds: none");
            return string.Join(Environment.NewLine, lines);
        }

        lines.AddRange(keybinds.Select(bind =>
            $"{HotkeyNames.For(bind.Modifiers, bind.Vk)} -> action/{ActionToken(bind.Action)}" +
            (bind.CycleEnabled ? " (cycle)" : string.Empty) +
            (bind.BypassTrigger ? " (bypass trigger)" : string.Empty)));
        return string.Join(Environment.NewLine, lines);
    }

    internal static string All(
        IReadOnlyList<Keybind> keybinds,
        uint triggerModifiers,
        uint triggerVk,
        TriggerModifierSide triggerSide = TriggerModifierSide.Any) =>
        $"Actions:{Environment.NewLine}{Actions()}{Environment.NewLine}{Environment.NewLine}" +
        $"Keybinds:{Environment.NewLine}{Keybinds(keybinds, triggerModifiers, triggerVk, triggerSide)}";

    private static string ActionToken(WindowAction action) =>
        LoopCommandParser.Normalize(action.ToString());
}
