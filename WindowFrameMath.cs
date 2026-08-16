using System;
using System.Collections.Generic;

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
            WindowAction.HorizontalCenterHalf => Rect(left + work.Width / 4, top, left + work.Width * 3 / 4, bottom),
            WindowAction.VerticalCenterHalf => Rect(left, top + work.Height / 4, right, top + work.Height * 3 / 4),
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
            WindowAction.FirstFourth => Rect(left, top, left + work.Width / 4, bottom),
            WindowAction.SecondFourth => Rect(left + work.Width / 4, top, left + work.Width / 2, bottom),
            WindowAction.ThirdFourth => Rect(left + work.Width / 2, top, left + work.Width * 3 / 4, bottom),
            WindowAction.FourthFourth => Rect(left + work.Width * 3 / 4, top, right, bottom),
            WindowAction.LeftThreeFourths => Rect(left, top, left + work.Width * 3 / 4, bottom),
            WindowAction.RightThreeFourths => Rect(left + work.Width / 4, top, right, bottom),
            _ => work
        };
    }

    public static NativeMethods.Rect MaximizeHeightFrame(
        NativeMethods.Rect work,
        NativeMethods.Rect current)
    {
        var width = Math.Min(current.Width, work.Width);
        var left = Math.Max(work.Left, Math.Min(current.Left, work.Right - width));
        return Rect(left, work.Top, left + width, work.Bottom);
    }

    public static NativeMethods.Rect MaximizeWidthFrame(
        NativeMethods.Rect work,
        NativeMethods.Rect current)
    {
        var height = Math.Min(current.Height, work.Height);
        var top = Math.Max(work.Top, Math.Min(current.Top, work.Bottom - height));
        return Rect(work.Left, top, work.Right, top + height);
    }

    public static NativeMethods.Rect FillAvailableFrame(
        NativeMethods.Rect work,
        NativeMethods.Rect current,
        IReadOnlyList<NativeMethods.Rect> obstacles)
    {
        var minX = work.Left;
        var minY = work.Top;
        var maxX = work.Right;
        var maxY = work.Bottom;

        var relevantObstacles = new List<NativeMethods.Rect>();
        foreach (var obstacle in obstacles)
        {
            if (Intersects(obstacle, current))
            {
                continue;
            }

            var clipped = Intersection(obstacle, work);
            if (clipped.Width <= 0 || clipped.Height <= 0)
            {
                continue;
            }

            relevantObstacles.Add(clipped);
            if (clipped.Right <= current.Left)
            {
                minX = Math.Max(minX, clipped.Right);
            }

            if (clipped.Bottom <= current.Top)
            {
                minY = Math.Max(minY, clipped.Bottom);
            }

            if (clipped.Left >= current.Right)
            {
                maxX = Math.Min(maxX, clipped.Left);
            }

            if (clipped.Top >= current.Bottom)
            {
                maxY = Math.Min(maxY, clipped.Top);
            }
        }

        var xBoundaries = new[]
        {
            (minX, maxX),
            (current.Left, maxX),
            (minX, current.Right),
            (current.Left, work.Right),
            (work.Left, current.Right),
            (work.Left, work.Right)
        };
        var yBoundaries = new[]
        {
            (minY, maxY),
            (current.Top, maxY),
            (minY, current.Bottom),
            (current.Top, work.Bottom),
            (work.Top, current.Bottom),
            (work.Top, work.Bottom)
        };

        var best = current;
        var bestArea = 0L;
        foreach (var (candidateLeft, candidateRight) in xBoundaries)
        {
            foreach (var (candidateTop, candidateBottom) in yBoundaries)
            {
                if (candidateRight <= candidateLeft || candidateBottom <= candidateTop)
                {
                    continue;
                }

                var candidate = Rect(candidateLeft, candidateTop, candidateRight, candidateBottom);
                var overlaps = false;
                foreach (var obstacle in relevantObstacles)
                {
                    if (Intersects(candidate, obstacle))
                    {
                        overlaps = true;
                        break;
                    }
                }

                if (overlaps)
                {
                    continue;
                }

                var area = (long)candidate.Width * candidate.Height;
                if (area > bestArea)
                {
                    best = candidate;
                    bestArea = area;
                }
            }
        }

        return best;
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
            case WindowAction.ScaleUp:
            case WindowAction.ScaleDown:
                var stepW = Math.Max(32, width / 10);
                var stepH = Math.Max(32, height / 10);
                if (action is WindowAction.ScaleUp or WindowAction.ScaleDown)
                {
                    var scale = action == WindowAction.ScaleUp ? 1.1 : 0.9;
                    width = (int)Math.Round(width * scale);
                    height = (int)Math.Round(height * scale);
                }
                else
                {
                    width = action == WindowAction.Larger ? width + stepW : width - stepW;
                    height = action == WindowAction.Larger ? height + stepH : height - stepH;
                }

                width = Math.Max(step, width);
                height = Math.Max(step, height);

                var cx = current.Left + current.Width / 2;
                var cy = current.Top + current.Height / 2;
                return Rect(cx - width / 2, cy - height / 2, cx - width / 2 + width, cy - height / 2 + height);

            case WindowAction.GrowLeft: return Rect(current.Left - step, current.Top, current.Right, current.Bottom);
            case WindowAction.GrowRight: return Rect(current.Left, current.Top, current.Right + step, current.Bottom);
            case WindowAction.GrowTop: return Rect(current.Left, current.Top - step, current.Right, current.Bottom);
            case WindowAction.GrowBottom: return Rect(current.Left, current.Top, current.Right, current.Bottom + step);
            case WindowAction.GrowHorizontal: return Rect(current.Left - step, current.Top, current.Right + step, current.Bottom);
            case WindowAction.GrowVertical: return Rect(current.Left, current.Top - step, current.Right, current.Bottom + step);
            case WindowAction.ShrinkLeft:
                return Rect(Math.Min(current.Right - 1, current.Left + step), current.Top, current.Right, current.Bottom);
            case WindowAction.ShrinkRight:
                return Rect(current.Left, current.Top, Math.Max(current.Left + 1, current.Right - step), current.Bottom);
            case WindowAction.ShrinkTop:
                return Rect(current.Left, Math.Min(current.Bottom - 1, current.Top + step), current.Right, current.Bottom);
            case WindowAction.ShrinkBottom:
                return Rect(current.Left, current.Top, current.Right, Math.Max(current.Top + 1, current.Bottom - step));
            case WindowAction.ShrinkHorizontal:
            {
                var newWidth = Math.Max(1, current.Width - 2 * step);
                var centerX = current.Left + current.Width / 2;
                return Rect(centerX - newWidth / 2, current.Top, centerX - newWidth / 2 + newWidth, current.Bottom);
            }
            case WindowAction.ShrinkVertical:
            {
                var newHeight = Math.Max(1, current.Height - 2 * step);
                var centerY = current.Top + current.Height / 2;
                return Rect(current.Left, centerY - newHeight / 2, current.Right, centerY - newHeight / 2 + newHeight);
            }
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
        action is WindowAction.Center or WindowAction.AlmostMaximize or WindowAction.HorizontalCenterHalf
            or WindowAction.VerticalCenterHalf or WindowAction.HorizontalCenterThird or WindowAction.VerticalCenterThird
            or WindowAction.Larger or WindowAction.Smaller or WindowAction.ScaleUp or WindowAction.ScaleDown
            or WindowAction.GrowHorizontal or WindowAction.GrowVertical
            or WindowAction.ShrinkHorizontal or WindowAction.ShrinkVertical;

    private static bool TouchesLeft(WindowAction action) =>
        action is WindowAction.LeftHalf or WindowAction.TopHalf or WindowAction.BottomHalf
            or WindowAction.TopLeftQuarter or WindowAction.BottomLeftQuarter
            or WindowAction.LeftThird or WindowAction.LeftTwoThirds
            or WindowAction.FirstFourth or WindowAction.LeftThreeFourths
            or WindowAction.TopThird or WindowAction.TopTwoThirds
            or WindowAction.BottomTwoThirds or WindowAction.BottomThird
            or WindowAction.MaximizeWidth or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool TouchesRight(WindowAction action) =>
        action is WindowAction.RightHalf or WindowAction.TopHalf or WindowAction.BottomHalf
            or WindowAction.TopRightQuarter or WindowAction.BottomRightQuarter
            or WindowAction.RightThird or WindowAction.RightTwoThirds
            or WindowAction.FourthFourth or WindowAction.RightThreeFourths
            or WindowAction.TopThird or WindowAction.TopTwoThirds
            or WindowAction.BottomTwoThirds or WindowAction.BottomThird
            or WindowAction.MaximizeWidth or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool TouchesTop(WindowAction action) =>
        action is WindowAction.TopHalf
            or WindowAction.TopLeftQuarter or WindowAction.TopRightQuarter
            or WindowAction.TopThird or WindowAction.TopTwoThirds
            or WindowAction.MaximizeHeight or WindowAction.FirstFourth or WindowAction.SecondFourth
            or WindowAction.ThirdFourth or WindowAction.FourthFourth or WindowAction.LeftThreeFourths
            or WindowAction.RightThreeFourths
            or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool TouchesBottom(WindowAction action) =>
        action is WindowAction.BottomHalf
            or WindowAction.BottomLeftQuarter or WindowAction.BottomRightQuarter
            or WindowAction.BottomThird or WindowAction.BottomTwoThirds
            or WindowAction.MaximizeHeight or WindowAction.FirstFourth or WindowAction.SecondFourth
            or WindowAction.ThirdFourth or WindowAction.FourthFourth or WindowAction.LeftThreeFourths
            or WindowAction.RightThreeFourths
            or WindowAction.Maximize or WindowAction.Fullscreen;

    private static bool Intersects(NativeMethods.Rect first, NativeMethods.Rect second) =>
        first.Left < second.Right && first.Right > second.Left &&
        first.Top < second.Bottom && first.Bottom > second.Top;

    private static NativeMethods.Rect Intersection(NativeMethods.Rect first, NativeMethods.Rect second) =>
        Rect(
            Math.Max(first.Left, second.Left),
            Math.Max(first.Top, second.Top),
            Math.Min(first.Right, second.Right),
            Math.Min(first.Bottom, second.Bottom));

    private static NativeMethods.Rect Rect(int left, int top, int right, int bottom) =>
        new() { Left = left, Top = top, Right = right, Bottom = bottom };
}
