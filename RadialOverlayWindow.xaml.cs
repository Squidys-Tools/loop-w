using System;
using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using MouseEventArgs = System.Windows.Input.MouseEventArgs;
using Point = System.Windows.Point;

namespace LoopW;

public partial class RadialOverlayWindow : Window
{
    private const double OverlayScale = 0.7333333333;
    private static readonly TimeSpan PreviewUpdateDelay = TimeSpan.FromMilliseconds(50);

    private readonly IntPtr _targetWindow;
    private readonly Action<RadialTarget> _commit;
    private readonly AppSettings _settings;
    private readonly IReadOnlyList<RadialTarget> _slotTargets;
    private readonly RadialTarget _centerTarget;
    private readonly PreviewOverlayWindow _preview;
    private readonly DispatcherTimer _pollTimer;
    private readonly DispatcherTimer _previewTimer;
    private RadialSelection? _selected;
    private RadialSelection? _pendingPreview;
    private bool _closing;
    private NativeMethods.Point _activationCursor;
    private Point? _lastPointerPoint;
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
        MenuSurface.ApplySettings(settings, OverlayScale);

        var enabled = new bool[_slotTargets.Count];
        for (var i = 0; i < enabled.Length; i++)
        {
            enabled[i] = _slotTargets[i] is not RadialTarget.None;
        }

        MenuSurface.SetSelectableSlots(enabled);
        MenuSurface.SetCenterEnabled(_centerTarget is not RadialTarget.None);
        Width = MenuSurface.Width;
        Height = MenuSurface.Height;

        _pollTimer = new DispatcherTimer(DispatcherPriority.Input)
        {
            Interval = TimeSpan.FromMilliseconds(20)
        };
        _pollTimer.Tick += PollTimer_Tick;

        _previewTimer = new DispatcherTimer(DispatcherPriority.Background)
        {
            Interval = PreviewUpdateDelay
        };
        _previewTimer.Tick += PreviewTimer_Tick;
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

        _activationCursor = cursor;

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

        UpdateSelection(CursorToLocal(cursor));
    }

    private Point CursorToLocal(NativeMethods.Point cursor)
    {
        var scaleX = 96.0 / _dpiX;
        var scaleY = 96.0 / _dpiY;
        return new Point(
            MenuSurface.Center + (cursor.X - _activationCursor.X) * scaleX,
            MenuSurface.Center + (cursor.Y - _activationCursor.Y) * scaleY);
    }

    private void CaptureBlurredBackdrop(double dpiX, double dpiY)
    {
        using var performance = PerformanceDiagnostics.Measure(PerformanceMetric.OverlayCapture);
        try
        {
            var margin = (int)Math.Round(MenuSurface.BlurMargin * dpiX / 96);
            var left = (int)Math.Round(Left * dpiX / 96) - margin;
            var top = (int)Math.Round(Top * dpiY / 96) - margin;
            var width = (int)Math.Round(Width * dpiX / 96) + margin * 2;
            var height = (int)Math.Round(Height * dpiY / 96) + margin * 2;
            var source = ScreenCapture.CaptureRegion(left, top, width, height);
            if (source != null)
            {
                source.Freeze();
                MenuSurface.BackdropImageElement.Source = source;
            }
        }
        catch
        {
            // The backdrop is decorative; the ring still renders without it.
        }
    }

    internal void CommitOrClose()
    {
        if (_closing)
        {
            return;
        }

        RefreshSelectionAtCursor();

        if (_selected is { } selected)
        {
            Commit(selected.Target);
        }
        else
        {
            CloseOverlay();
        }
    }

    internal void Dismiss() => CloseOverlay();

    internal void RefreshTargetFrame()
    {
        if (_selected is { } selection)
        {
            SchedulePreviewUpdate(selection);
        }
    }

    private void Overlay_MouseMove(object sender, MouseEventArgs e) =>
        UpdateSelection(e.GetPosition(this));

    private void UpdateSelection(Point point)
    {
        var pointerMoved = !_lastPointerPoint.HasValue || _lastPointerPoint.Value != point;
        _lastPointerPoint = point;
        if (pointerMoved && _selected is not null && _settings.PreviewEnabled)
        {
            _pendingPreview = _selected;
            _previewTimer.Stop();
            _previewTimer.Start();
        }

        var dx = point.X - MenuSurface.Center;
        var dy = point.Y - MenuSurface.Center;
        if (dx * dx + dy * dy < MenuSurface.InnerRadius * MenuSurface.InnerRadius)
        {
            SetSelection(_centerTarget is not RadialTarget.None
                ? new RadialSelection.Center(_centerTarget)
                : null);
            return;
        }

        var slot = MenuSurface.SlotAtDirection(point);
        if (slot is int index && index < _slotTargets.Count && _slotTargets[index] is not RadialTarget.None)
        {
            SetSelection(new RadialSelection.Wedge(index, _slotTargets[index]));
            return;
        }

        SetSelection(null);
    }

    private void Overlay_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (!_settings.CursorInteractionEnabled)
        {
            return;
        }

        UpdateSelection(e.GetPosition(this));

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

        if (index >= 0 && index < _slotTargets.Count)
        {
            var target = _slotTargets[index];
            if (target is not RadialTarget.None)
            {
                Commit(target);
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

        _selected = selection;
        MenuSurface.SetSelectedSlot(selection is RadialSelection.Wedge wedge ? wedge.Index : null);
        MenuSurface.SetSelectedCenter(selection is RadialSelection.Center);

        SchedulePreviewUpdate(selection);
    }

    private void SchedulePreviewUpdate(RadialSelection? selection)
    {
        _pendingPreview = selection;
        _previewTimer.Stop();

        if (selection is null)
        {
            if (_preview.IsVisible)
            {
                _preview.Hide();
            }

            return;
        }

        if (_settings.PreviewEnabled)
        {
            _previewTimer.Start();
        }
    }

    private void RefreshSelectionAtCursor()
    {
        if (!_settings.CursorInteractionEnabled ||
            !NativeMethods.GetCursorPos(out var cursor))
        {
            return;
        }

        UpdateSelection(CursorToLocal(cursor));
    }

    private void PreviewTimer_Tick(object? sender, EventArgs e)
    {
        _previewTimer.Stop();
        if (!_closing)
        {
            ShowPreviewFor(_pendingPreview);
        }
    }

    private void ShowPreviewFor(RadialSelection? selection)
    {
        var action = selection is { } resolvedSelection
            ? RadialTargetResolver.ActionOf(resolvedSelection.Target)
            : null;
        if (_settings.PreviewEnabled && action.HasValue &&
            WindowActionService.TryGetTargetFrame(_targetWindow, action.Value, out var frame, out _))
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

    private void Commit(RadialTarget target)
    {
        if (_closing)
        {
            return;
        }

        _commit(target);
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
        _previewTimer.Stop();
        _pendingPreview = null;
        _preview.HideImmediately(destroy: true);
        Close();
    }
}
