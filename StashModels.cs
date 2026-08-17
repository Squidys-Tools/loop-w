using System;

namespace LoopW;

public enum StashEdge
{
    Left,
    Right,
    Top,
    Bottom
}

public sealed class StashRecord
{
    public string Id { get; set; } = string.Empty;

    public string ExecutablePath { get; set; } = string.Empty;

    public uint ProcessId { get; set; }

    public string WindowClass { get; set; } = string.Empty;

    public string Title { get; set; } = string.Empty;

    public StashEdge Edge { get; set; }

    public StashPlacement OriginalPlacement { get; set; } = new();

    public StashMonitor OriginalMonitor { get; set; } = new();

    public StashRect StashedFrame { get; set; } = new();
}

public sealed class StashPlacement
{
    public int Length { get; set; }

    public uint Flags { get; set; }

    public uint ShowCommand { get; set; }

    public StashPoint MinPosition { get; set; } = new();

    public StashPoint MaxPosition { get; set; } = new();

    public StashRect NormalPosition { get; set; } = new();
}

public sealed class StashMonitor
{
    public StashRect Monitor { get; set; } = new();

    public StashRect Work { get; set; } = new();

    public double DpiX { get; set; } = 96;

    public double DpiY { get; set; } = 96;
}

public sealed class StashRect
{
    public int Left { get; set; }

    public int Top { get; set; }

    public int Right { get; set; }

    public int Bottom { get; set; }

    internal NativeMethods.Rect ToNative() => new()
    {
        Left = Left,
        Top = Top,
        Right = Right,
        Bottom = Bottom
    };

    internal static StashRect FromNative(NativeMethods.Rect value) => new()
    {
        Left = value.Left,
        Top = value.Top,
        Right = value.Right,
        Bottom = value.Bottom
    };
}

public sealed class StashPoint
{
    public int X { get; set; }

    public int Y { get; set; }

    internal NativeMethods.Point ToNative() => new() { X = X, Y = Y };

    internal static StashPoint FromNative(NativeMethods.Point value) => new() { X = value.X, Y = value.Y };
}
