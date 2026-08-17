using System;
using System.Collections.Generic;

namespace LoopW;

public enum RadialTargetKind
{
    None,
    Action,
    Keybind
}

/// <summary>
/// Persisted radial target data. AppSettings.Normalize validates this boundary
/// representation before the runtime resolves it into a typed RadialTarget.
/// </summary>
public sealed class RadialTargetSettings
{
    public RadialTargetKind Kind { get; set; } = RadialTargetKind.Action;

    public WindowAction Action { get; set; } = WindowAction.RightHalf;

    public string KeybindId { get; set; } = string.Empty;

    public bool CycleEnabled { get; set; }
}

internal abstract record RadialTarget
{
    internal sealed record None : RadialTarget;

    internal sealed record BuiltInAction(WindowAction Value, bool CycleEnabled) : RadialTarget;

    internal sealed record KeybindAction(Keybind Binding, bool CycleEnabled) : RadialTarget;
}

internal abstract record RadialSelection
{
    internal abstract RadialTarget Target { get; }

    internal sealed record Wedge(int Index, RadialTarget Value) : RadialSelection
    {
        internal override RadialTarget Target => Value;
    }

    internal sealed record Center(RadialTarget Value) : RadialSelection
    {
        internal override RadialTarget Target => Value;
    }
}

internal static class RadialConfiguration
{
    public const int SlotCount = 8;

    public static List<RadialTargetSettings> CreateDefaultSlots() => new()
    {
        CreateAction(WindowAction.RightHalf),
        CreateAction(WindowAction.BottomRightQuarter),
        CreateAction(WindowAction.BottomHalf),
        CreateAction(WindowAction.BottomLeftQuarter),
        CreateAction(WindowAction.LeftHalf),
        CreateAction(WindowAction.TopLeftQuarter),
        CreateAction(WindowAction.TopHalf),
        CreateAction(WindowAction.TopRightQuarter)
    };

    public static RadialTargetSettings CreateDefaultCenter() => new()
    {
        Kind = RadialTargetKind.None
    };

    public static RadialTargetSettings CreateAction(WindowAction action) => new()
    {
        Kind = RadialTargetKind.Action,
        Action = action,
        CycleEnabled = WindowCycleService.CanCycle(action)
    };

    public static RadialTargetSettings CreateKeybind(Keybind keybind) => new()
    {
        Kind = RadialTargetKind.Keybind,
        KeybindId = keybind.Id,
        CycleEnabled = keybind.CycleEnabled
    };
}

internal static class RadialTargetResolver
{
    public static RadialTarget Resolve(RadialTargetSettings settings, IReadOnlyList<Keybind> keybinds)
    {
        return settings.Kind switch
        {
            RadialTargetKind.None => new RadialTarget.None(),
            RadialTargetKind.Action => new RadialTarget.BuiltInAction(settings.Action, settings.CycleEnabled),
            RadialTargetKind.Keybind => ResolveKeybind(settings.KeybindId, settings.CycleEnabled, keybinds),
            _ => new RadialTarget.None()
        };
    }

    public static IReadOnlyList<RadialTarget> ResolveSlots(
        IReadOnlyList<RadialTargetSettings> slots,
        IReadOnlyList<Keybind> keybinds)
    {
        var resolved = new RadialTarget[RadialConfiguration.SlotCount];
        for (var i = 0; i < resolved.Length; i++)
        {
            resolved[i] = i < slots.Count
                ? Resolve(slots[i], keybinds)
                : new RadialTarget.None();
        }

        return resolved;
    }

    public static WindowAction? ActionOf(RadialTarget target) => target switch
    {
        RadialTarget.BuiltInAction action => action.Value,
        RadialTarget.KeybindAction keybind => keybind.Binding.Action,
        _ => null
    };

    public static bool CycleEnabledOf(RadialTarget target) => target switch
    {
        RadialTarget.BuiltInAction action => action.CycleEnabled,
        RadialTarget.KeybindAction keybind => keybind.CycleEnabled,
        _ => false
    };

    public static string DisplayName(RadialTarget target) => target switch
    {
        RadialTarget.BuiltInAction action => WindowActionService.ActionName(action.Value),
        RadialTarget.KeybindAction keybind =>
            $"{HotkeyNames.For(keybind.Binding.Modifiers, keybind.Binding.Vk)} · " +
            WindowActionService.ActionName(keybind.Binding.Action),
        _ => "No action"
    };

    private static RadialTarget ResolveKeybind(
        string keybindId,
        bool cycleEnabled,
        IReadOnlyList<Keybind> keybinds)
    {
        for (var i = 0; i < keybinds.Count; i++)
        {
            if (string.Equals(keybinds[i].Id, keybindId, StringComparison.Ordinal))
            {
                return new RadialTarget.KeybindAction(keybinds[i], cycleEnabled);
            }
        }

        return new RadialTarget.None();
    }
}

