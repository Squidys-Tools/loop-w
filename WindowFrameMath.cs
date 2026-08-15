using System;

namespace LoopW;

internal static class WindowFrameMath
{
    public static NativeMethods.Rect ZoneFrame(NativeMethods.Rect work, WindowAction action)
    {
        var left = work.Left;
        var top = work.Top;
        var right = work.Right;
        var bottom = work.Bottom;
        var midX = left + work.Width / 2;
        var midY = top + work.Height / 2;
        var thirdW = work.Width / 3;
        var twoThirdW = work.Width * 2 / 3;
        var thirdH = work.Height / 3;
        var twoThirdH = work.Height * 2 / 3;

        return action switch
        {
            WindowAction.LeftHalf => Rect(left, top, midX, bottom),
            WindowAction.RightHalf => Rect(midX, top, right, bottom),
            WindowAction.TopHalf => Rect(left, top, right, midY),
            WindowAction.BottomHalf => Rect(left, midY, right, bottom),
            WindowAction.TopLeftQuarter => Rect(left, top, midX, midY),
            WindowAction.TopRightQuarter => Rect(midX, top, right, midY),
            WindowAction.BottomLeftQuarter => Rect(left, midY, midX, bottom),
            WindowAction.BottomRightQuarter => Rect(midX, midY, right, bottom),
            WindowAction.LeftThird => Rect(left, top, left + thirdW, bottom),
            WindowAction.LeftTwoThirds => Rect(left, top, left + twoThirdW, bottom),
            WindowAction.HorizontalCenterThird => Rect(left + thirdW, top, left + twoThirdW, bottom),
            WindowAction.RightTwoThirds => Rect(left + thirdW, top, right, bottom),
            WindowAction.RightThird => Rect(left + twoThirdW, top, right, bottom),
            WindowAction.TopThird => Rect(left, top, right, top + thirdH),
            WindowAction.TopTwoThirds => Rect(left, top, right, top + twoThirdH),
            WindowAction.VerticalCenterThird => Rect(left, top + thirdH, right, top + twoThirdH),
            WindowAction.BottomTwoThirds => Rect(left, top + thirdH, right, bottom),
            WindowAction.BottomThird => Rect(left, top + twoThirdH, right, bottom),
            _ => work
        };
    }

    public static NativeMethods.Rect CenterFrame(NativeMethods.Rect work, NativeMethods.Rect current)
    {
        var width = Math.Min(current.Width, work.Width);
        var height = Math.Min(current.Height, work.Height);
        var left = work.Left + (work.Width - width) / 2;
        var top = work.Top + (work.Height - height) / 2;
        return Rect(left, top, left + width, top + height);
    }

    public static NativeMethods.Rect ManipulateFrame(
        NativeMethods.Rect work,
        WindowAction action,
        NativeMethods.Rect current,
        double dpiScale)
    {
        var step = (int)Math.Round(48 * dpiScale);
        var width = current.Width;
        var height = current.Height;

        switch (action)
        {
            case WindowAction.Larger:
            case WindowAction.Smaller:
                var stepW = Math.Max(32, width / 10);
                var stepH = Math.Max(32, height / 10);
                width = action == WindowAction.Larger ? width + stepW : width - stepW;
                height = action == WindowAction.Larger ? height + stepH : height - stepH;
                width = Math.Max(step, width);
                height = Math.Max(step, height);

                var cx = current.Left + current.Width / 2;
                var cy = current.Top + current.Height / 2;
                return Rect(cx - width / 2, cy - height / 2, cx - width / 2 + width, cy - height / 2 + height);

            case WindowAction.GrowLeft: return Rect(current.Left - step, current.Top, current.Right, current.Bottom);
            case WindowAction.GrowRight: return Rect(current.Left, current.Top, current.Right + step, current.Bottom);
            case WindowAction.GrowTop: return Rect(current.Left, current.Top - step, current.Right, current.Bottom);
            case WindowAction.GrowBottom: return Rect(current.Left, current.Top, current.Right, current.Bottom + step);
            case WindowAction.ShrinkLeft: return Rect(current.Left + step, current.Top, current.Right, current.Bottom);
            case WindowAction.ShrinkRight: return Rect(current.Left, current.Top, current.Right - step, current.Bottom);
            case WindowAction.ShrinkTop: return Rect(current.Left, current.Top + step, current.Right, current.Bottom);
            case WindowAction.ShrinkBottom: return Rect(current.Left, current.Top, current.Right, current.Bottom - step);
            case WindowAction.MoveLeft: return Rect(current.Left - step, current.Top, current.Right - step, current.Bottom);
            case WindowAction.MoveRight: return Rect(current.Left + step, current.Top, current.Right + step, current.Bottom);
            case WindowAction.MoveUp: return Rect(current.Left, current.Top - step, current.Right, current.Bottom - step);
            case WindowAction.MoveDown: return Rect(current.Left, current.Top + step, current.Right, current.Bottom + step);
            default: return current;
        }
    }

    public static NativeMethods.Rect FitFrame(
        NativeMethods.Rect bounds,
        WindowAction action,
        NativeMethods.Rect frame,
        NativeMethods.MinMaxInfo limits)
    {
        var width = frame.Width;
        var height = frame.Height;

        width = Math.Max(width, limits.MinTrackSize.X);
        height = Math.Max(height, limits.MinTrackSize.Y);
        if (limits.MaxTrackSize.X > 0)
        {
            width = Math.Min(width, limits.MaxTrackSize.X);
        }

        if (limits.MaxTrackSize.Y > 0)
        {
            height = Math.Min(height, limits.MaxTrackSize.Y);
        }

        width = Math.Min(width, bounds.Width);
        height = Math.Min(height, bounds.Height);

        if (IsCenteredAction(action))
        {
            var centerLeft = frame.Left + (frame.Width - width) / 2;
            var centerTop = frame.Top + (frame.Height - height) / 2;
            centerLeft = Math.Max(bounds.Left, Math.Min(centerLeft, bounds.Right - width));
            centerTop = Math.Max(bounds.Top, Math.Min(centerTop, bounds.Bottom - height));
            return Rect(centerLeft, centerTop, centerLeft + width, centerTop + height);
        }

        var left = Math.Max(bounds.Left, Math.Min(frame.Left, bounds.Right - width));
        var top = Math.Max(bounds.Top, Math.Min(frame.Top, bounds.Bottom - height));
        if (TouchesRight(action))
        {
            left = bounds.Right - width;
        }

        if (TouchesBottom(action))
        {
            top = bounds.Bottom - height;
        }

        if (TouchesLeft(action))
        {
            left = bounds.Left;
        }

        if (TouchesTop(action))
        {
            top = bounds.Top;
        }

        return Rect(left, top, left + width, top + height);
    }

    private static bool IsCenteredAction(WindowAction action) =>
        action is WindowAction.Center or WindowAction.AlmostMaximize or WindowAction.HorizontalCenterThird or WindowAction.VerticalCenterThird;

    private static bool TouchesLeft(WindowAction action) =>
        action is WindowAction.LeftHalf or WindowAction.TopHalf or WindowAction.BottomHalf
            or WindowAction.TopLeftQuarter or WindowAction.BottomLeftQuarter
            or WindowAction.LeftThird or WindowAction.LeftTwoThirds
            or WindowAction.TopThird or WindowAction.TopTwoThirds
            or WindowAction.BottomTwoThirds or WindowAction.BottomThird
            or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool TouchesRight(WindowAction action) =>
        action is WindowAction.RightHalf or WindowAction.TopHalf or WindowAction.BottomHalf
            or WindowAction.TopRightQuarter or WindowAction.BottomRightQuarter
            or WindowAction.RightThird or WindowAction.RightTwoThirds
            or WindowAction.TopThird or WindowAction.TopTwoThirds
            or WindowAction.BottomTwoThirds or WindowAction.BottomThird
            or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool TouchesTop(WindowAction action) =>
        action is WindowAction.TopHalf
            or WindowAction.TopLeftQuarter or WindowAction.TopRightQuarter
            or WindowAction.TopThird or WindowAction.TopTwoThirds
            or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool TouchesBottom(WindowAction action) =>
        action is WindowAction.BottomHalf
            or WindowAction.BottomLeftQuarter or WindowAction.BottomRightQuarter
            or WindowAction.BottomThird or WindowAction.BottomTwoThirds
            or WindowAction.Maximize or WindowAction.Fullscreen;

    private static NativeMethods.Rect Rect(int left, int top, int right, int bottom) =>
        new() { Left = left, Top = top, Right = right, Bottom = bottom };
}
