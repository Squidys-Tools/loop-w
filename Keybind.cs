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
