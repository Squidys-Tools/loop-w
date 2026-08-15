namespace LoopW;

/// <summary>
/// A trigger + key combination that applies a window action without opening the
/// radial menu. Persisted as part of AppSettings.
/// </summary>
public sealed class Keybind
{
    public uint Modifiers { get; set; }

    public uint Vk { get; set; }

    public WindowAction Action { get; set; }

    /// <summary>
    /// Repeating this trigger + key advances through the action's directional
    /// cycle. Missing values in older settings files keep this default.
    /// </summary>
    public bool CycleEnabled { get; set; } = true;

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
