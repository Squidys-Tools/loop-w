using System;
using System.Collections.Generic;

namespace LoopW;

internal readonly record struct RadialActionSlot(
    WindowAction Action,
    string Label,
    double FromDegrees,
    double ToDegrees)
{
    public double CenterDegrees => (FromDegrees + ToDegrees) / 2;
}

internal readonly record struct RadialSlotGeometry(
    string Label,
    double FromDegrees,
    double ToDegrees)
{
    public double CenterDegrees => (FromDegrees + ToDegrees) / 2;
}

internal static class RadialActionCatalog
{
    public static IReadOnlyList<RadialSlotGeometry> Geometry { get; } = new[]
    {
        new RadialSlotGeometry("Right", -22.5, 22.5),
        new RadialSlotGeometry("Bottom-right", 22.5, 67.5),
        new RadialSlotGeometry("Bottom", 67.5, 112.5),
        new RadialSlotGeometry("Bottom-left", 112.5, 157.5),
        new RadialSlotGeometry("Left", 157.5, 202.5),
        new RadialSlotGeometry("Top-left", 202.5, 247.5),
        new RadialSlotGeometry("Top", 247.5, 292.5),
        new RadialSlotGeometry("Top-right", 292.5, 337.5)
    };

    // Kept as a compatibility view for geometry tests and callers that still
    // need the original default action list.
    public static IReadOnlyList<RadialActionSlot> Slots { get; } = new[]
    {
        new RadialActionSlot(WindowAction.RightHalf, "Right half", -22.5, 22.5),
        new RadialActionSlot(WindowAction.BottomRightQuarter, "Bottom-right quarter", 22.5, 67.5),
        new RadialActionSlot(WindowAction.BottomHalf, "Bottom half", 67.5, 112.5),
        new RadialActionSlot(WindowAction.BottomLeftQuarter, "Bottom-left quarter", 112.5, 157.5),
        new RadialActionSlot(WindowAction.LeftHalf, "Left half", 157.5, 202.5),
        new RadialActionSlot(WindowAction.TopLeftQuarter, "Top-left quarter", 202.5, 247.5),
        new RadialActionSlot(WindowAction.TopHalf, "Top half", 247.5, 292.5),
        new RadialActionSlot(WindowAction.TopRightQuarter, "Top-right quarter", 292.5, 337.5)
    };

    public static int IndexAt(double angleDegrees)
    {
        var normalized = (angleDegrees + 360) % 360;
        return (int)Math.Floor((normalized + 22.5) / 45) % Geometry.Count;
    }

    public static IReadOnlyList<RadialTarget> LoadTargets(AppSettings settings) =>
        RadialTargetResolver.ResolveSlots(settings.RadialSlots, settings.Keybinds);

    public static WindowAction ActionAt(double angleDegrees)
    {
        var index = IndexAt(angleDegrees);
        return Slots[index].Action;
    }

}
