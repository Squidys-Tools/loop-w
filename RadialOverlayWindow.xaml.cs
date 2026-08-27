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

    private readonly IntPtr _targetWindow;
    private readonly Action<RadialTarget> _commit;
    private readonly AppSettings _settings;
    private readonly IReadOnlyList<RadialTarget> _slotTargets;
    private readonly RadialTarget _centerTarget;
    private readonly PreviewOverlayWindow _preview;
    private readonly DispatcherTimer _pollTimer;
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
            ShowPreviewFor(selection);
        }
    }

    private void Overlay_MouseMove(object sender, MouseEventArgs e) =>
        UpdateSelection(e.GetPosition(this));

    private void UpdateSelection(Point point)
    {
        var slot = MenuSurface.SlotAt(point);
        if (slot is int index && index < _slotTargets.Count && _slotTargets[index] is not RadialTarget.None)
        {
            SetSelection(new RadialSelection.Wedge(index, _slotTargets[index]));
            return;
        }

        var dx = point.X - MenuSurface.Center;
        var dy = point.Y - MenuSurface.Center;
        if (Math.Sqrt(dx * dx + dy * dy) < MenuSurface.InnerRadius && _centerTarget is not RadialTarget.None)
        {
            SetSelection(new RadialSelection.Center(_centerTarget));
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

        ShowPreviewFor(selection);
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
        _preview.HideImmediately(destroy: true);
        Close();
    }
}
