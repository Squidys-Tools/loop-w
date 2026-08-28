using System;
using System.Windows;
using System.Windows.Media;
using Point = System.Windows.Point;
using Size = System.Windows.Size;

namespace LoopW;

internal static class RadialGeometry
{
    public static PathGeometry BuildAnnulus(double center, double outerRadius, double innerRadius)
    {
        var ring = new PathGeometry { FillRule = FillRule.EvenOdd };
        ring.Figures.Add(BuildCircle(center, outerRadius));
        ring.Figures.Add(BuildCircle(center, innerRadius));
        return ring;
    }

    public static PathGeometry BuildWedge(double center, double outerRadius, double innerRadius, double fromDeg, double toDeg)
    {
        var from = DegToRad(fromDeg);
        var to = DegToRad(toDeg);

        var figure = new PathFigure
        {
            StartPoint = Polar(center, outerRadius, from),
            IsClosed = true,
            IsFilled = true
        };
        figure.Segments.Add(new LineSegment(Polar(center, innerRadius, from), true));
        figure.Segments.Add(new ArcSegment(Polar(center, innerRadius, to), new Size(innerRadius, innerRadius), 0, false, SweepDirection.Clockwise, true));
        figure.Segments.Add(new LineSegment(Polar(center, outerRadius, to), true));
        figure.Segments.Add(new ArcSegment(Polar(center, outerRadius, from), new Size(outerRadius, outerRadius), 0, false, SweepDirection.Counterclockwise, true));

        var geometry = new PathGeometry();
        geometry.Figures.Add(figure);
        return geometry;
    }

    public static int? IndexAtDirection(Point point, double center, double deadzone)
    {
        var dx = point.X - center;
        var dy = point.Y - center;
        var minimumDistance = Math.Max(0, deadzone);
        if (dx * dx + dy * dy < minimumDistance * minimumDistance)
        {
            return null;
        }

        return RadialActionCatalog.IndexAt(Math.Atan2(dy, dx) * 180 / Math.PI);
    }

    private static PathFigure BuildCircle(double center, double radius)
    {
        var start = Polar(center, radius, 0);
        var figure = new PathFigure { StartPoint = start, IsClosed = true, IsFilled = true };
        figure.Segments.Add(new ArcSegment(Polar(center, radius, Math.PI), new Size(radius, radius), 0, false, SweepDirection.Clockwise, true));
        figure.Segments.Add(new ArcSegment(start, new Size(radius, radius), 0, false, SweepDirection.Clockwise, true));
        return figure;
    }

    private static double DegToRad(double deg) => deg * Math.PI / 180;

    private static Point Polar(double center, double radius, double angleRad) =>
        new(center + radius * Math.Cos(angleRad), center + radius * Math.Sin(angleRad));
}
