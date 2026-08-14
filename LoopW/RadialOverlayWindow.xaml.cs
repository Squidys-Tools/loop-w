using System;
using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;
using System.Windows.Threading;

namespace LoopW;

public partial class RadialOverlayWindow : Window
{
    private const double Center = 200;
    private const double OuterRadius = 176;
    private const double InnerRadius = 64;
    private const double DeadZoneRadius = 64;
    private const double BlurMargin = 48;

    private readonly IntPtr _targetWindow;
    private readonly Action<WindowHalf> _commit;
    private readonly PreviewOverlayWindow _preview = new();
    private readonly DispatcherTimer _pollTimer;
    private WindowHalf? _selected;
    private bool _closing;
    private double _dpiX = 96;
    private double _dpiY = 96;

    internal RadialOverlayWindow(IntPtr targetWindow, Action<WindowHalf> commit)
    {
        InitializeComponent();
        _targetWindow = targetWindow;
        _commit = commit;
        BuildGeometry();

        // The ring's transparent center hole is skipped by Windows hit-testing, so
        // MouseMove never fires there and the dead-zone can't clear a selection.
        // Poll the real cursor position instead so selection stays accurate even
        // over the hole and after mouse-move coalescing.
        _pollTimer = new DispatcherTimer(DispatcherPriority.Input)
        {
            Interval = TimeSpan.FromMilliseconds(20)
        };
        _pollTimer.Tick += PollTimer_Tick;
    }

    private void BuildGeometry()
    {
        var annulus = BuildAnnulus(Center);
        Ring.Data = annulus;
        BackdropImage.Clip = BuildAnnulus(Center + BlurMargin);
        TopWedge.Data = BuildWedge(-135, -45);
        RightWedge.Data = BuildWedge(-45, 45);
        BottomWedge.Data = BuildWedge(45, 135);
        LeftWedge.Data = BuildWedge(135, 225);
    }

    private static PathGeometry BuildAnnulus(double center)
    {
        var ring = new PathGeometry { FillRule = FillRule.EvenOdd };
        ring.Figures.Add(BuildCircle(center, OuterRadius));
        ring.Figures.Add(BuildCircle(center, InnerRadius));
        return ring;
    }

    private static PathFigure BuildCircle(double center, double radius)
    {
        var start = Polar(center, radius, 0);
        var figure = new PathFigure { StartPoint = start, IsClosed = true, IsFilled = true };
        figure.Segments.Add(new ArcSegment(Polar(center, radius, Math.PI), new Size(radius, radius), 0, false, SweepDirection.Clockwise, true));
        figure.Segments.Add(new ArcSegment(start, new Size(radius, radius), 0, false, SweepDirection.Clockwise, true));
        return figure;
    }

    private static PathGeometry BuildWedge(double fromDeg, double toDeg)
    {
        var from = DegToRad(fromDeg);
        var to = DegToRad(toDeg);

        var figure = new PathFigure
        {
            StartPoint = Polar(Center, OuterRadius, from),
            IsClosed = true,
            IsFilled = true
        };
        figure.Segments.Add(new LineSegment(Polar(Center, InnerRadius, from), true));
        figure.Segments.Add(new ArcSegment(Polar(Center, InnerRadius, to), new Size(InnerRadius, InnerRadius), 0, false, SweepDirection.Clockwise, true));
        figure.Segments.Add(new LineSegment(Polar(Center, OuterRadius, to), true));
        figure.Segments.Add(new ArcSegment(Polar(Center, OuterRadius, from), new Size(OuterRadius, OuterRadius), 0, false, SweepDirection.Counterclockwise, true));

        var geometry = new PathGeometry();
        geometry.Figures.Add(figure);
        return geometry;
    }

    private static double DegToRad(double deg) => deg * Math.PI / 180;

    private static Point Polar(double center, double radius, double angleRad) =>
        new(center + radius * Math.Cos(angleRad), center + radius * Math.Sin(angleRad));

    private void Overlay_Loaded(object sender, RoutedEventArgs e)
    {
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(220));
        var scale = (ScaleTransform)OverlaySurface.RenderTransform;
        OverlaySurface.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(0, 1, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(0.92, 1, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(0.92, 1, duration) { EasingFunction = ease });
    }

    public void ShowAtCursor()
    {
        if (!NativeMethods.GetCursorPos(out var cursor))
        {
            return;
        }

        var monitor = NativeMethods.MonitorFromPoint(cursor, NativeMethods.MonitorDefaultToNearest);
        double dpiX = 96;
        double dpiY = 96;
        if (NativeMethods.TryGetMonitorDpi(monitor, out dpiX, out dpiY))
        {
            Left = cursor.X * 96.0 / dpiX - Width / 2;
            Top = cursor.Y * 96.0 / dpiY - Height / 2;
        }
        else
        {
            Left = cursor.X - Width / 2;
            Top = cursor.Y - Height / 2;
        }

        _dpiX = dpiX;
        _dpiY = dpiY;

        CaptureBlurredBackdrop(dpiX, dpiY);

        Show();
        _pollTimer.Start();
    }

    private void PollTimer_Tick(object? sender, EventArgs e)
    {
        if (_closing || !IsVisible)
        {
            return;
        }

        if (!NativeMethods.GetCursorPos(out var cursor))
        {
            return;
        }

        var localX = cursor.X * 96.0 / _dpiX - Left;
        var localY = cursor.Y * 96.0 / _dpiY - Top;
        UpdateSelection(new Point(localX, localY));
    }

    private void CaptureBlurredBackdrop(double dpiX, double dpiY)
    {
        try
        {
            var margin = (int)Math.Round(BlurMargin * dpiX / 96);
            var left = (int)Math.Round(Left * dpiX / 96) - margin;
            var top = (int)Math.Round(Top * dpiY / 96) - margin;
            var width = (int)Math.Round(Width * dpiX / 96) + margin * 2;
            var height = (int)Math.Round(Height * dpiY / 96) + margin * 2;

            var source = CaptureScreenRegion(left, top, width, height);
            if (source != null)
            {
                source.Freeze();
                BackdropImage.Source = source;
            }
        }
        catch
        {
            // blur is decorative; the ring still renders without a backdrop
        }
    }

    private static BitmapSource? CaptureScreenRegion(int left, int top, int width, int height)
    {
        if (width <= 0 || height <= 0)
        {
            return null;
        }

        var hdcSrc = NativeMethods.GetDC(IntPtr.Zero);
        var hdcMem = NativeMethods.CreateCompatibleDC(hdcSrc);
        var hbmp = NativeMethods.CreateCompatibleBitmap(hdcSrc, width, height);
        if (hbmp == IntPtr.Zero)
        {
            NativeMethods.DeleteDC(hdcMem);
            NativeMethods.ReleaseDC(IntPtr.Zero, hdcSrc);
            return null;
        }

        var previous = NativeMethods.SelectObject(hdcMem, hbmp);
        NativeMethods.BitBlt(hdcMem, 0, 0, width, height, hdcSrc, left, top, NativeMethods.SrcCopy);
        NativeMethods.SelectObject(hdcMem, previous);
        var source = Imaging.CreateBitmapSourceFromHBitmap(hbmp, IntPtr.Zero, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
        NativeMethods.DeleteObject(hbmp);
        NativeMethods.DeleteDC(hdcMem);
        NativeMethods.ReleaseDC(IntPtr.Zero, hdcSrc);
        return source;
    }

    /// <summary>
    /// Called when the trigger key is released: commit the current selection,
    /// or close without committing if nothing was chosen.
    /// </summary>
    internal void CommitOrClose()
    {
        if (_closing)
        {
            return;
        }

        if (_selected.HasValue)
        {
            Commit(_selected.Value);
        }
        else
        {
            CloseOverlay();
        }
    }

    private void Overlay_MouseMove(object sender, MouseEventArgs e)
    {
        UpdateSelection(e.GetPosition(this));
    }

    private void UpdateSelection(Point point)
    {
        var dx = point.X - Center;
        var dy = point.Y - Center;

        if (Math.Sqrt(dx * dx + dy * dy) < DeadZoneRadius)
        {
            SetSelection(null);
            return;
        }

        var selection = Math.Abs(dx) > Math.Abs(dy)
            ? (dx < 0 ? WindowHalf.Left : WindowHalf.Right)
            : (dy < 0 ? WindowHalf.Top : WindowHalf.Bottom);

        SetSelection(selection);
    }

    private void Overlay_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (_selected.HasValue)
        {
            Commit(_selected.Value);
        }
        else
        {
            CloseOverlay();
        }
    }

    private void Overlay_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            CloseOverlay();
            e.Handled = true;
            return;
        }

        var selection = e.Key switch
        {
            Key.Left => WindowHalf.Left,
            Key.Right => WindowHalf.Right,
            Key.Up => WindowHalf.Top,
            Key.Down => WindowHalf.Bottom,
            _ => (WindowHalf?)null
        };

        if (selection.HasValue)
        {
            Commit(selection.Value);
            e.Handled = true;
        }
    }

    private void SetSelection(WindowHalf? selection)
    {
        _selected = selection;
        HighlightWedge(TopWedge, selection == WindowHalf.Top);
        HighlightWedge(RightWedge, selection == WindowHalf.Right);
        HighlightWedge(BottomWedge, selection == WindowHalf.Bottom);
        HighlightWedge(LeftWedge, selection == WindowHalf.Left);

        if (selection.HasValue && WindowActionService.TryGetHalfFrame(_targetWindow, selection.Value, out var frame, out _))
        {
            _preview.ShowFrame(frame, selection.Value);
            _preview.Topmost = true;
            RaiseAbovePreview();
        }
        else if (_preview.IsVisible)
        {
            _preview.Hide();
        }
    }

    private void RaiseAbovePreview()
    {
        var hwnd = new WindowInteropHelper(this).Handle;
        if (hwnd != IntPtr.Zero)
        {
            NativeMethods.SetWindowPos(hwnd, NativeMethods.HwndTopmost, 0, 0, 0, 0,
                NativeMethods.SwpNoMove | NativeMethods.SwpNoSize | NativeMethods.SwpNoActivate);
        }
    }

    private static void HighlightWedge(Path wedge, bool on)
    {
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(130));
        wedge.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(wedge.Opacity, on ? 1 : 0, duration) { EasingFunction = ease });
    }

    private void Commit(WindowHalf action)
    {
        if (_closing)
        {
            return;
        }

        _commit(action);
        CloseOverlay();
    }

    private void CloseOverlay()
    {
        if (_closing)
        {
            return;
        }

        _closing = true;
        _pollTimer.Stop();
        _preview.HideAnimated(destroy: true);

        var ease = new CubicEase { EasingMode = EasingMode.EaseIn };
        var duration = new Duration(TimeSpan.FromMilliseconds(120));
        var scale = (ScaleTransform)OverlaySurface.RenderTransform;
        var opacity = new DoubleAnimation(OverlaySurface.Opacity, 0, duration) { EasingFunction = ease };
        opacity.Completed += (_, _) => Close();
        OverlaySurface.BeginAnimation(UIElement.OpacityProperty, opacity);
        scale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(scale.ScaleX, 0.94, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(scale.ScaleY, 0.94, duration) { EasingFunction = ease });
    }
}
