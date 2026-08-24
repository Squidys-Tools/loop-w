using System;
using System.Text;
using System.Windows;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Cursors = System.Windows.Input.Cursors;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;
using Wpf.Ui.Controls;

namespace LoopW;

public partial class MainWindow : FluentWindow
{
    private readonly GlobalHotkey _hotkey = new();
    private readonly AppSettings _settings;
    private readonly DragSnapService _dragSnap;
    private IReadOnlyList<RadialTarget> _radialTargets = Array.Empty<RadialTarget>();
    private IntPtr _targetWindow;
    private RadialOverlayWindow? _activeOverlay;
    private readonly DispatcherTimer _stashTimer;
    private PreviewOverlayWindow? _dragPreview;
    private System.Windows.Interop.HwndSource? _windowSource;
    private bool _capturing;
    private bool _allowClose;

    public MainWindow()
    {
        InitializeComponent();
        _settings = AppSettings.Load();
        MonitorService.Configure(_settings);
        WindowPolicy.Configure(_settings);
        WindowStashService.Configure(_settings);
        _radialTargets = RadialActionCatalog.LoadTargets(_settings);

        var settingsPanel = new SettingsWindow(_hotkey, _settings);
        settingsPanel.SettingsChanged += ApplySettings;
        SettingsHost.Content = settingsPanel;

        _hotkey.SetBinding(_settings.TriggerModifiers, _settings.TriggerVk);
        _hotkey.SetTriggerBehavior(
            _settings.TriggerModifierSide,
            _settings.TriggerDelayMilliseconds,
            _settings.TriggerTimeoutMilliseconds,
            _settings.DoubleClickToTrigger,
            _settings.MiddleClickToTrigger);
        _hotkey.SetKeybinds(_settings.Keybinds);
        _dragSnap = new DragSnapService(Dispatcher, _settings);
        _stashTimer = new DispatcherTimer(DispatcherPriority.Input)
        {
            Interval = TimeSpan.FromMilliseconds(80)
        };
        _stashTimer.Tick += StashTimer_Tick;
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        _hotkey.TriggerPressed += Hotkey_TriggerPressed;
        _hotkey.TriggerReleased += Hotkey_TriggerReleased;
        _hotkey.TriggerCancelled += Hotkey_TriggerCancelled;
        _hotkey.TriggerTimedOut += Hotkey_TriggerTimedOut;
        _hotkey.KeyCaptured += Hotkey_KeyCaptured;
        _hotkey.CaptureCancelled += Hotkey_CaptureCancelled;
        _hotkey.CaptureRejected += Hotkey_CaptureRejected;
        _hotkey.KeybindPressed += Hotkey_KeybindPressed;
        _dragSnap.TargetChanged += DragSnap_TargetChanged;
        _dragSnap.TargetCleared += DragSnap_TargetCleared;
        _dragSnap.GestureEnded += DragSnap_GestureEnded;
        _hotkey.Start();
        _dragSnap.Start();
        _stashTimer.Start();

        UpdateTriggerLabel();
        TargetStatus.Text = $"  ·  No target captured — hold {TriggerLabel.Text} while another app is focused";

        if (!_hotkey.IsActive)
        {
            TargetStatus.Text = "  ·  Could not install the global keyboard hook";
            RebindHint.Text = "unavailable";
            RebindHint.Cursor = Cursors.Arrow;
        }
    }

    private void Window_Closed(object? sender, EventArgs e)
    {
        _stashTimer.Stop();
        _dragSnap.TargetChanged -= DragSnap_TargetChanged;
        _dragSnap.TargetCleared -= DragSnap_TargetCleared;
        _dragSnap.GestureEnded -= DragSnap_GestureEnded;
        _windowSource?.RemoveHook(WindowMessageHook);
        _dragSnap.Dispose();
        _dragPreview?.Close();
        _hotkey.Dispose();
    }

    private void StashTimer_Tick(object? sender, EventArgs e)
    {
        WindowStashService.Poll();
        if (NativeMethods.GetCursorPos(out var cursor) &&
            WindowStashService.TryRevealAtCursor(cursor, out var message))
        {
            TargetStatus.Text = $"  ·  {message}";
        }
    }

    private void Window_SourceInitialized(object? sender, EventArgs e)
    {
        var hwnd = new System.Windows.Interop.WindowInteropHelper(this).Handle;
        if (hwnd == IntPtr.Zero)
        {
            return;
        }

        _windowSource = System.Windows.Interop.HwndSource.FromHwnd(hwnd);
        _windowSource?.AddHook(WindowMessageHook);
    }

    private IntPtr WindowMessageHook(
        IntPtr hwnd,
        int message,
        IntPtr wParam,
        IntPtr lParam,
        ref bool handled)
    {
        if (message is NativeMethods.WmDisplayChange or
            NativeMethods.WmDpiChanged or
            NativeMethods.WmSettingChange or
            NativeMethods.WmDeviceChange)
        {
            MonitorService.Invalidate();
            _dragSnap.RefreshTarget();
            _activeOverlay?.RefreshTargetFrame();
            _dragPreview?.HideImmediately();
        }

        return IntPtr.Zero;
    }

    private void Window_Closing(object? sender, System.ComponentModel.CancelEventArgs e)
    {
        if (_allowClose)
        {
            return;
        }

        e.Cancel = true;
        Hide();
    }

    internal void AllowClose() => _allowClose = true;

    internal void ShowFromTray()
    {
        Show();
        if (WindowState == WindowState.Minimized)
        {
            WindowState = WindowState.Normal;
        }

        Activate();
    }

    internal void OpenSettingsFromTray() => ShowFromTray();

    internal string ExecuteExternalAction(WindowAction action)
    {
        var ownWindow = new System.Windows.Interop.WindowInteropHelper(this).Handle;
        if (action == WindowAction.RevealStashed)
        {
            var revealed = WindowActionService.TryApply(IntPtr.Zero, action, out var revealMessage);
            TargetStatus.Text = $"  ·  {revealMessage}";
            return revealMessage;
        }

        var foreground = NativeMethods.GetForegroundWindow();
        if (foreground == IntPtr.Zero || foreground == ownWindow)
        {
            return "No external foreground window is available.";
        }

        var applied = WindowActionService.TryApply(foreground, action, out var message);
        if (applied)
        {
            SelectedAction.Text = WindowActionService.ActionName(action);
        }

        TargetStatus.Text = $"  ·  {message}";
        return message;
    }

    internal string DescribeKeybinds() =>
        LoopCommandFormatter.Keybinds(
            _settings.Keybinds,
            _settings.TriggerModifiers,
            _settings.TriggerVk,
            _settings.TriggerModifierSide);

    internal string DescribeAll() =>
        LoopCommandFormatter.All(
            _settings.Keybinds,
            _settings.TriggerModifiers,
            _settings.TriggerVk,
            _settings.TriggerModifierSide);

    private void Hotkey_TriggerPressed()
    {
        CaptureTargetWindow();
    }

    private void Hotkey_TriggerReleased()
    {
        if (_activeOverlay is { IsVisible: true } overlay)
        {
            overlay.CommitOrClose();
        }
    }

    private void Hotkey_TriggerTimedOut()
    {
        _activeOverlay?.Dismiss();
        _targetWindow = IntPtr.Zero;
        TargetStatus.Text = "  ·  Trigger timed out — no action committed";
    }

    private void Hotkey_TriggerCancelled()
    {
        _activeOverlay?.Dismiss();
        _targetWindow = IntPtr.Zero;
        TargetStatus.Text = "  ·  Trigger cancelled";
    }

    private void Hotkey_KeyCaptured(uint modifiers, uint vk)
    {
        _capturing = false;
        _hotkey.SetBinding(modifiers, vk);

        // Load the existing settings and update only the trigger fields, so a
        // rebind never wipes out persisted keybinds.
        _settings.TriggerModifiers = modifiers;
        _settings.TriggerVk = vk;
        var saved = _settings.Save();

        SetCapturingUi(false);
        TargetStatus.Text = saved
            ? $"  ·  Trigger set to {HotkeyNames.For(modifiers, vk, _settings.TriggerModifierSide)}"
            : "  ·  Trigger updated for this session, but could not be saved";
        Keyboard.ClearFocus();
    }

    private void Hotkey_CaptureCancelled()
    {
        _capturing = false;
        SetCapturingUi(false);
        TargetStatus.Text = "  ·  Rebind cancelled";
    }

    private void Hotkey_CaptureRejected()
    {
        _capturing = false;
        SetCapturingUi(false);
        TargetStatus.Text = "  ·  That key is reserved by the OS — try another (Caps Lock is a good one)";
    }

    private void TriggerCap_MouseUp(object sender, MouseButtonEventArgs e) => BeginRebind();

    private void RebindHint_MouseUp(object sender, MouseButtonEventArgs e) => BeginRebind();

    private void BeginRebind()
    {
        if (_capturing || !_hotkey.IsActive)
        {
            return;
        }

        _capturing = true;
        _hotkey.BeginCapture();
        SetCapturingUi(true);
        TargetStatus.Text = "  ·  Press any key, or a combo like Ctrl + B — Esc to cancel";
        Keyboard.ClearFocus();
    }

    private void SetCapturingUi(bool capturing)
    {
        TriggerLabel.Text = capturing
            ? "Press a key…"
            : HotkeyNames.For(_hotkey.TriggerModifiers, _hotkey.TriggerVk, _settings.TriggerModifierSide);
        RebindHint.Text = capturing ? "Esc to cancel" : "rebind";
        TriggerCap.BorderBrush = capturing
            ? (Brush)FindResource("SystemAccentColorPrimaryBrush")
            : (Brush)FindResource("ControlStrokeColorDefaultBrush");
    }

    private void UpdateTriggerLabel()
    {
        TriggerLabel.Text = HotkeyNames.For(
            _hotkey.TriggerModifiers,
            _hotkey.TriggerVk,
            _settings.TriggerModifierSide);
    }

    private void CaptureTargetWindow()
    {
        if (_activeOverlay is { IsVisible: true })
        {
            return;
        }

        var foreground = NativeMethods.GetForegroundWindow();
        var ownWindow = new System.Windows.Interop.WindowInteropHelper(this).Handle;

        if (foreground == IntPtr.Zero || foreground == ownWindow)
        {
            TargetStatus.Text = $"  ·  Focus another app, then press {TriggerLabel.Text}";
            return;
        }

        _targetWindow = foreground;
        var title = new StringBuilder(256);
        NativeMethods.GetWindowText(_targetWindow, title, title.Capacity);
        TargetStatus.Text = $"  ·  Target: {(title.Length > 0 ? title.ToString() : "active window")}";
        if (_settings.RadialEnabled)
        {
            var overlay = new RadialOverlayWindow(_targetWindow, CommitOverlayTarget, _settings);
            _activeOverlay = overlay;
            overlay.Closed += (_, _) => _activeOverlay = null;
            overlay.ShowAtCursor();
        }
    }

    private void Hotkey_KeybindPressed(Keybind keybind)
    {
        if (keybind.Action == WindowAction.RevealStashed)
        {
            _activeOverlay?.Dismiss();
            ApplyWindowAction(keybind.Action, cycleEnabled: false);
            return;
        }

        if (_targetWindow == IntPtr.Zero)
        {
            TargetStatus.Text = $"  ·  Capture a target first with {TriggerLabel.Text}";
            return;
        }

        // A keybind applies the action directly, so dismiss any open radial
        // overlay; otherwise releasing the trigger would commit a second wedge.
        _activeOverlay?.Dismiss();

        ApplyWindowAction(keybind.Action, keybind.CycleEnabled);
    }

    private void Settings_MouseUp(object sender, MouseButtonEventArgs e) => OpenSettings();

    private void OpenSettings() => ShowFromTray();

    private void CommitOverlayTarget(RadialTarget target)
    {
        ApplyRadialTarget(target);
    }

    private void ApplyRadialTarget(RadialTarget target)
    {
        var action = RadialTargetResolver.ActionOf(target);
        if (!action.HasValue)
        {
            SelectedAction.Text = "No action";
            TargetStatus.Text = "  ·  No radial action is configured";
            return;
        }

        ApplyWindowAction(action.Value, RadialTargetResolver.CycleEnabledOf(target));
    }

    private void Window_KeyDown(object sender, KeyEventArgs e)
    {
        var action = e.Key switch
        {
            Key.Left => WindowAction.LeftHalf,
            Key.Right => WindowAction.RightHalf,
            Key.Up => WindowAction.TopHalf,
            Key.Down => WindowAction.BottomHalf,
            _ => (WindowAction?)null
        };

        if (action.HasValue)
        {
            ApplyWindowAction(action.Value, WindowCycleService.CanCycle(action.Value));
            e.Handled = true;
        }
    }

    private void ApplyWindowAction(WindowAction requestedAction, bool cycleEnabled)
    {
        if (requestedAction == WindowAction.RevealStashed)
        {
            SelectedAction.Text = WindowActionService.ActionName(requestedAction);
            WindowActionService.TryApply(IntPtr.Zero, requestedAction, out var revealMessage);
            TargetStatus.Text = $"  ·  {revealMessage}";
            return;
        }

        if (_targetWindow == IntPtr.Zero)
        {
            TargetStatus.Text = $"  ·  Capture a target first with {TriggerLabel.Text}";
            return;
        }

        var selection = WindowCycleService.Select(_targetWindow, requestedAction, cycleEnabled);
        SelectedAction.Text = WindowActionService.ActionName(selection.EffectiveAction);

        var applied = WindowActionService.TryApply(_targetWindow, selection.EffectiveAction, out var message);
        if (applied)
        {
            WindowCycleService.Commit(_targetWindow, selection);
        }

        TargetStatus.Text = $"  ·  {message}{(applied ? selection.StatusSuffix : string.Empty)}";
    }

    private void ApplySettings(AppSettings settings)
    {
        WindowStashService.UpdateSettings(settings);
        MonitorService.UpdateSettings(settings);
        WindowPolicy.UpdateSettings(settings);
        _radialTargets = RadialActionCatalog.LoadTargets(settings);
        _hotkey.SetBinding(settings.TriggerModifiers, settings.TriggerVk);
        _hotkey.SetTriggerBehavior(
            settings.TriggerModifierSide,
            settings.TriggerDelayMilliseconds,
            settings.TriggerTimeoutMilliseconds,
            settings.DoubleClickToTrigger,
            settings.MiddleClickToTrigger);
        _hotkey.SetKeybinds(settings.Keybinds);
        _dragPreview?.HideImmediately();
        _dragSnap.UpdateSettings();
        UpdateTriggerLabel();
    }

    private void DragSnap_TargetChanged(DragSnapTarget target)
    {
        if (!_settings.PreviewEnabled)
        {
            return;
        }

        _dragPreview ??= new PreviewOverlayWindow(_settings);
        _dragPreview.ShowFrame(target.Frame, target.Action);
        _dragPreview.Topmost = true;
    }

    private void DragSnap_TargetCleared()
    {
        _dragPreview?.HideImmediately();
    }

    private void DragSnap_GestureEnded(DragSnapGesture gesture)
    {
        Dispatcher.BeginInvoke(DispatcherPriority.Input, () =>
        {
            if (gesture.Reason == DragSnapEndReason.Released && gesture.Target is { } target)
            {
                var applied = WindowActionService.TryApplySnap(gesture.Window, target, out var message);
                if (applied)
                {
                    SelectedAction.Text = WindowActionService.ActionName(target.Action);
                }

                TargetStatus.Text = $"  ·  {message}";
                return;
            }

            if (_settings.RestorePreDragFrameOnSnapCancel)
            {
                WindowActionService.TryRestoreFrame(gesture.Window, gesture.OriginalFrame, out var message);
                TargetStatus.Text = $"  ·  {message}";
            }
        });
    }
}
