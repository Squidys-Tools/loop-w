namespace LoopW;

[Flags]
public enum SettingsChangeDomain
{
    None = 0,
    Trigger = 1 << 0,
    Radial = 1 << 1,
    Preview = 1 << 2,
    DragSnap = 1 << 3,
    Stash = 1 << 4,
    Monitor = 1 << 5,
    Exclusions = 1 << 6,
    Appearance = 1 << 7,
    Keybinds = 1 << 8,
    All = Trigger | Radial | Preview | DragSnap | Stash | Monitor | Exclusions | Appearance | Keybinds
}
