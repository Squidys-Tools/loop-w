using System;
using System.Collections.Generic;
using System.Windows;
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
    private readonly Action<RadialTarget> _commit;
    private readonly AppSettings _settings;
    private readonly IReadOnlyList<RadialTarget> _slotTargets;
    private readonly RadialTarget _centerTarget;
    private readonly PreviewOverlayWindow _preview;
    private readonly DispatcherTimer _pollTimer;
    private readonly double _center;
    private readonly double _outerRadius;
    private readonly double _innerRadius;
    private readonly double _blurMargin;
    private readonly Path[] _wedgePaths;
    private RadialSelection? _selected;
    private bool _closing;
    private double _dpiX = 96;
    private double _dpiY = 96;

    internal RadialOverlayWindow(IntPtr targetWindow, Action<RadialTarget> commit, AppSettings settings)
    {
        InitializeComponent();
        _targetWindow = targetWindow;
        _commit = commit;
        _settings = settings;
        _slotTargets = RadialActionCatalog.LoadTargets(settings);
        _centerTarget = RadialTargetResolver.Resolve(settings.CenterTarget, settings.Keybinds);
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
        Width = _center * 2;
        Height = _center * 2;
        Resources["SectorFillBrush"] = CreateBrush(settings.RadialSectorFill, "#7A007AFF");
        Resources["SectorStrokeBrush"] = CreateBrush(settings.RadialSectorStroke, "#F0007AFF");
        Ring.Fill = CreateBrush(settings.RadialRingFill, "#B61E1E1E");
        BuildGeometry();

        if (_settings.IsLightAppearance)
        {
            ApplyLightAppearance();
        }

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
        CenterHighlight.Data = new EllipseGeometry(new Point(_center, _center), _innerRadius, _innerRadius);
        BackdropImage.Clip = RadialGeometry.BuildAnnulus(_center + _blurMargin, _outerRadius, _innerRadius);

        for (var i = 0; i < RadialActionCatalog.Geometry.Count; i++)
        {
            var slot = RadialActionCatalog.Geometry[i];
            _wedgePaths[i].Data = RadialGeometry.BuildWedge(
                _center,
                _outerRadius,
                _innerRadius,
                slot.FromDegrees,
                slot.ToDegrees);
        }
    }

    private void ApplyLightAppearance()
    {
        Resources["SectorFillBrush"] = CreateBrush("#D9007AFF", "#D9007AFF");
        Resources["SectorStrokeBrush"] = CreateBrush("#F0007AFF", "#F0007AFF");
        Ring.Fill = CreateBrush("#E6F0F4F8", "#E6F0F4F8");
        Ring.Effect = new System.Windows.Media.Effects.DropShadowEffect
        {
            BlurRadius = 14,
            Direction = 270,
            ShadowDepth = 3,
            Opacity = 0.22,
            Color = System.Windows.Media.Color.FromRgb(0x52, 0x61, 0x6B)
        };

        foreach (var wedge in _wedgePaths)
        {
            wedge.StrokeThickness = 0.8;
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

        if (_selected is { } selected)
        {
            Commit(selected.Target);
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

    internal void RefreshTargetFrame()
    {
        if (_selected is { } selection)
        {
            ShowPreviewFor(selection);
        }
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
            SetSelection(CreateCenterSelection());
            return;
        }

        var index = RadialActionCatalog.IndexAt(Math.Atan2(dy, dx) * 180 / Math.PI);

        SetSelection(CreateWedgeSelection(index));
    }

    private void Overlay_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (!_settings.CursorInteractionEnabled)
        {
            return;
        }

        if (_selected is { } selected)
        {
            Commit(selected.Target);
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

        var index = e.Key switch
        {
            Key.Right => 0,
            Key.Down => 2,
            Key.Left => 4,
            Key.Up => 6,
            _ => -1
        };

        if (index >= 0)
        {
            var selection = CreateWedgeSelection(index);
            if (selection is { } resolved)
            {
                Commit(resolved.Target);
            }
            else
            {
                CloseOverlay();
            }

            e.Handled = true;
        }
    }

    private void SetSelection(RadialSelection? selection)
    {
        if (_selected == selection)
        {
            return;
        }

        var previous = _selected;
        _selected = selection;

        // Cursor polling runs every 20 ms, so only animate the paths whose state
        // changed. Reapplying all eight animations on every tick creates and
        // replaces hundreds of animation clocks per second while hovering.
        var previousIndex = previous is RadialSelection.Wedge previousWedge ? previousWedge.Index : -1;
        var selectedIndex = selection is RadialSelection.Wedge selectedWedge ? selectedWedge.Index : -1;
        for (var i = 0; i < RadialActionCatalog.Geometry.Count; i++)
        {
            if (i == previousIndex || i == selectedIndex)
            {
                HighlightWedge(_wedgePaths[i], i == selectedIndex);
            }
        }

        HighlightCenter(selection is RadialSelection.Center);

        ShowPreviewFor(selection);
    }

    private void ShowPreviewFor(RadialSelection? selection)
    {
        var action = selection is { } resolvedSelection
            ? RadialTargetResolver.ActionOf(resolvedSelection.Target)
            : null;
        if (_settings.PreviewEnabled && action.HasValue && WindowActionService.TryGetTargetFrame(_targetWindow, action.Value, out var frame, out _))
        {
            _preview.ShowFrame(frame, action.Value);
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
        var target = on ? 1 : 0;
        var from = wedge.Opacity;

        // Remove the previous clock before setting the base value. This prevents
        // finished/replaced hover animations from accumulating on the path.
        wedge.BeginAnimation(UIElement.OpacityProperty, null);
        wedge.Opacity = target;

        if (Math.Abs(from - target) < 0.001)
        {
            return;
        }

        var animation = new DoubleAnimation(from, target, new Duration(TimeSpan.FromMilliseconds(130)))
        {
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
            FillBehavior = FillBehavior.Stop
        };
        wedge.BeginAnimation(UIElement.OpacityProperty, animation, HandoffBehavior.SnapshotAndReplace);
    }

    private RadialSelection? CreateWedgeSelection(int index)
    {
        if (index < 0 || index >= _slotTargets.Count || _slotTargets[index] is RadialTarget.None)
        {
            return null;
        }

        return new RadialSelection.Wedge(index, _slotTargets[index]);
    }

    private RadialSelection? CreateCenterSelection() => _centerTarget is RadialTarget.None
        ? null
        : new RadialSelection.Center(_centerTarget);

    private void Commit(RadialTarget target)
    {
        if (_closing)
        {
            return;
        }

        _commit(target);
        CloseOverlay();
    }

    private void HighlightCenter(bool on)
    {
        var target = on ? 1 : 0;
        var from = CenterHighlight.Opacity;
        CenterHighlight.BeginAnimation(UIElement.OpacityProperty, null);
        CenterHighlight.Opacity = target;

        if (Math.Abs(from - target) < 0.001)
        {
            return;
        }

        CenterHighlight.BeginAnimation(
            UIElement.OpacityProperty,
            new DoubleAnimation(from, target, new Duration(TimeSpan.FromMilliseconds(130)))
            {
                EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
                FillBehavior = FillBehavior.Stop
            },
            HandoffBehavior.SnapshotAndReplace);
    }

    private void CloseOverlay()
    {
        if (_closing)
        {
            return;
        }

        _closing = true;
        _pollTimer.Stop();
        _preview.HideImmediately(destroy: true);
        Close();
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
