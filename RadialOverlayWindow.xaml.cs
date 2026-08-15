using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Shapes;
using System.Windows.Threading;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using MouseEventArgs = System.Windows.Input.MouseEventArgs;
using Point = System.Windows.Point;

namespace LoopW;

public partial class RadialOverlayWindow : Window
{
    private const double OverlayScale = 0.7333333333;

    private readonly IntPtr _targetWindow;
    private readonly Action<WindowAction> _commit;
    private readonly AppSettings _settings;
    private readonly PreviewOverlayWindow _preview;
    private readonly DispatcherTimer _pollTimer;
    private readonly double _center;
    private readonly double _outerRadius;
    private readonly double _innerRadius;
    private readonly double _blurMargin;
    private readonly Path[] _wedgePaths;
    private readonly TextBlock[] _labels;
    private WindowAction? _selected;
    private bool _closing;
    private double _dpiX = 96;
    private double _dpiY = 96;

    internal RadialOverlayWindow(IntPtr targetWindow, Action<WindowAction> commit, AppSettings settings)
    {
        InitializeComponent();
        _targetWindow = targetWindow;
        _commit = commit;
        _settings = settings;
        _preview = new PreviewOverlayWindow(settings);
        _outerRadius = settings.RadialOuterRadius * OverlayScale;
        _innerRadius = settings.RadialInnerRadius * OverlayScale;
        _center = _outerRadius * 1.1;
        _blurMargin = _outerRadius * 0.27;
        _wedgePaths = new[]
        {
            RightWedge, BottomRightWedge, BottomWedge, BottomLeftWedge,
            LeftWedge, TopLeftWedge, TopWedge, TopRightWedge
        };
        _labels = new[]
        {
            RightLabel, BottomRightLabel, BottomLabel, BottomLeftLabel,
            LeftLabel, TopLeftLabel, TopLabel, TopRightLabel
        };
        Width = _center * 2;
        Height = _center * 2;
        Resources["SectorFillBrush"] = CreateBrush(settings.RadialSectorFill, "#7A007AFF");
        Resources["SectorStrokeBrush"] = CreateBrush(settings.RadialSectorStroke, "#F0007AFF");
        Ring.Fill = CreateBrush(settings.RadialRingFill, "#B61E1E1E");
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
        Ring.Data = RadialGeometry.BuildAnnulus(_center, _outerRadius, _innerRadius);
        BackdropImage.Clip = RadialGeometry.BuildAnnulus(_center + _blurMargin, _outerRadius, _innerRadius);

        var labelDistance = (_innerRadius + _outerRadius) / 2;
        for (var i = 0; i < RadialActionCatalog.Slots.Count; i++)
        {
            var slot = RadialActionCatalog.Slots[i];
            _wedgePaths[i].Data = RadialGeometry.BuildWedge(
                _center,
                _outerRadius,
                _innerRadius,
                slot.FromDegrees,
                slot.ToDegrees);
            _labels[i].Text = slot.Label;

            var angle = slot.CenterDegrees * Math.PI / 180;
            Canvas.SetLeft(_labels[i], _center + Math.Cos(angle) * labelDistance - _labels[i].Width / 2);
            Canvas.SetTop(_labels[i], _center + Math.Sin(angle) * labelDistance - 8);
        }
    }

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

        if (!_settings.CursorInteractionEnabled)
        {
            SetSelection(null);
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
            var margin = (int)Math.Round(_blurMargin * dpiX / 96);
            var left = (int)Math.Round(Left * dpiX / 96) - margin;
            var top = (int)Math.Round(Top * dpiY / 96) - margin;
            var width = (int)Math.Round(Width * dpiX / 96) + margin * 2;
            var height = (int)Math.Round(Height * dpiY / 96) + margin * 2;

            var source = ScreenCapture.CaptureRegion(left, top, width, height);
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

    /// <summary>
    /// Closes the overlay without committing a selection. Used when a keybind
    /// fires while the overlay is open so releasing the trigger cannot apply a
    /// second (unintended) wedge action.
    /// </summary>
    internal void Dismiss()
    {
        CloseOverlay();
    }

    private void Overlay_MouseMove(object sender, MouseEventArgs e)
    {
        if (!_settings.CursorInteractionEnabled)
        {
            return;
        }

        UpdateSelection(e.GetPosition(this));
    }

    private void UpdateSelection(Point point)
    {
        var dx = point.X - _center;
        var dy = point.Y - _center;

        if (Math.Sqrt(dx * dx + dy * dy) < _innerRadius)
        {
            SetSelection(null);
            return;
        }

        var selection = RadialActionCatalog.ActionAt(Math.Atan2(dy, dx) * 180 / Math.PI);

        SetSelection(selection);
    }

    private void Overlay_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (!_settings.CursorInteractionEnabled)
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
            Key.Left => WindowAction.LeftHalf,
            Key.Right => WindowAction.RightHalf,
            Key.Up => WindowAction.TopHalf,
            Key.Down => WindowAction.BottomHalf,
            _ => (WindowAction?)null
        };

        if (selection.HasValue)
        {
            Commit(selection.Value);
            e.Handled = true;
        }
    }

    private void SetSelection(WindowAction? selection)
    {
        _selected = selection;
        for (var i = 0; i < RadialActionCatalog.Slots.Count; i++)
        {
            HighlightWedge(_wedgePaths[i], RadialActionCatalog.Slots[i].Action == selection);
        }

        CenterLabel.Text = selection.HasValue
            ? WindowActionService.ActionName(selection.Value)
            : "Choose an action";

        if (_settings.PreviewEnabled && selection.HasValue && WindowActionService.TryGetTargetFrame(_targetWindow, selection.Value, out var frame, out _))
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

    private void Commit(WindowAction action)
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

    private static System.Windows.Media.Brush CreateBrush(string value, string fallback)
    {
        try
        {
            if (new System.Windows.Media.BrushConverter().ConvertFromString(value) is System.Windows.Media.Brush brush)
            {
                brush.Freeze();
                return brush;
            }
        }
        catch
        {
            // Invalid in-memory values use a safe fallback.
        }

        return new System.Windows.Media.BrushConverter().ConvertFromString(fallback) as System.Windows.Media.Brush
            ?? System.Windows.Media.Brushes.Transparent;
    }
}
