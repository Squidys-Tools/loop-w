using System;

namespace LoopW;

public enum TriggerModifierSide
{
    Any,
    Left,
    Right
}

/// <summary>
/// A trigger + key combination that applies a window action without opening the
/// radial menu. Persisted as part of AppSettings.
/// </summary>
public sealed class Keybind
{
    public string Id { get; set; } = Guid.NewGuid().ToString("N");

    public uint Modifiers { get; set; }

    public uint Vk { get; set; }

    public WindowAction Action { get; set; }

    /// <summary>
    /// Repeating this trigger + key advances through the action's directional
    /// cycle. Missing values in older settings files keep this default.
    /// </summary>
    public bool CycleEnabled { get; set; } = true;

    /// <summary>
    /// Applies this action when its key combination is pressed without the
    /// configured trigger key.
    /// </summary>
    public bool BypassTrigger { get; set; }

    public Keybind()
    {
    }

    public Keybind(uint modifiers, uint vk, WindowAction action)
    {
        Modifiers = modifiers;
        Vk = vk;
        Action = action;
    }
}
