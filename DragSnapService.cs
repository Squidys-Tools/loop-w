using System;
using System.Runtime.InteropServices;
using System.Windows.Threading;

namespace LoopW;

internal enum DragSnapEndReason
{
    Released,
    CaptureLost,
    WindowUnavailable,
    Disabled
}

internal readonly record struct DragSnapTarget(WindowAction Action, NativeMethods.Rect Frame);

internal readonly record struct DragSnapGesture(
    IntPtr Window,
    NativeMethods.Rect OriginalFrame,
    DragSnapTarget? Target,
    DragSnapEndReason Reason);

/// <summary>
/// Observes ordinary title-bar drags without taking mouse capture or suppressing
/// any input. It owns only the gesture state; placement and preview rendering stay
/// in the UI shell and the action service.
/// </summary>
internal sealed class DragSnapService : IDisposable
{
    private const int WhMouseLl = 14;
    private const int DragStartDistance = 4;

    private readonly Dispatcher _dispatcher;
    private readonly AppSettings _settings;
    private readonly NativeMethods.MouseHookProc _hookProc;
    private readonly DispatcherTimer _captureTimer;
    private IntPtr _hookHandle;
    private bool _leftButtonDown;
    private bool _dragStarted;
    private IntPtr _window;
    private NativeMethods.Rect _originalFrame;
    private NativeMethods.Point _startPoint;
    private DragSnapTarget? _target;
    private bool _hadCandidate;

    public DragSnapService(Dispatcher dispatcher, AppSettings settings)
    {
        _dispatcher = dispatcher;
        _settings = settings;
        _hookProc = HookProc;
        _captureTimer = new DispatcherTimer(DispatcherPriority.Input, dispatcher)
        {
            Interval = TimeSpan.FromMilliseconds(45)
        };
        _captureTimer.Tick += CaptureTimer_Tick;
    }

    public bool IsActive => _hookHandle != IntPtr.Zero;

    public event Action<DragSnapTarget>? TargetChanged;

    public event Action? TargetCleared;

    public event Action<DragSnapGesture>? GestureEnded;

    public void Start()
    {
        if (_hookHandle == IntPtr.Zero)
        {
            _hookHandle = NativeMethods.SetWindowsHookEx(WhMouseLl, _hookProc, IntPtr.Zero, 0);
        }
    }

    public void UpdateSettings()
    {
        if (!_settings.DragSnapEnabled && _leftButtonDown)
        {
            EndGesture(DragSnapEndReason.Disabled);
        }
        else if (_target is { } target)
        {
            TargetChanged?.Invoke(target);
        }
    }

    public void Dispose()
    {
        _captureTimer.Stop();
        ResetGesture();
        if (_hookHandle != IntPtr.Zero)
        {
            NativeMethods.UnhookWindowsHookEx(_hookHandle);
            _hookHandle = IntPtr.Zero;
        }

        GC.SuppressFinalize(this);
    }

    private IntPtr HookProc(int nCode, IntPtr wParam, IntPtr lParam)
    {
        if (nCode >= 0)
        {
            var message = wParam.ToInt32();
            if (message is NativeMethods.WmLButtonDown or NativeMethods.WmLButtonUp or NativeMethods.WmMouseMove)
            {
                var data = Marshal.PtrToStructure<NativeMethods.MouseLlHookStruct>(lParam);
                Dispatch(message, data.Point);
            }
        }

        return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
    }

    private void Dispatch(int message, NativeMethods.Point point)
    {
        void Handle()
        {
            switch (message)
            {
                case NativeMethods.WmLButtonDown:
                    HandleButtonDown(point);
                    break;
                case NativeMethods.WmLButtonUp:
                    HandleButtonUp();
                    break;
                case NativeMethods.WmMouseMove:
                    HandleMouseMove(point);
                    break;
            }
        }

        if (_dispatcher.CheckAccess())
        {
            Handle();
        }
        else
        {
            _dispatcher.BeginInvoke(DispatcherPriority.Input, Handle);
        }
    }

    private void HandleButtonDown(NativeMethods.Point point)
    {
        if (_leftButtonDown || !_settings.DragSnapEnabled)
        {
            return;
        }

        var source = NativeMethods.WindowFromPoint(point);
        var window = NativeMethods.GetAncestor(source, NativeMethods.GaRoot);
        if (!CanBeginDrag(window, point) || !NativeMethods.GetWindowRect(window, out var frame))
        {
            return;
        }

        _leftButtonDown = true;
        _dragStarted = false;
        _window = window;
        _originalFrame = frame;
        _startPoint = point;
        _target = null;
        _hadCandidate = false;
        _captureTimer.Start();
    }

    private void HandleButtonUp()
    {
        if (_leftButtonDown)
        {
            EndGesture(DragSnapEndReason.Released);
        }
    }

    private void HandleMouseMove(NativeMethods.Point point)
    {
        if (!_leftButtonDown)
        {
            return;
        }

        if (!IsLeftButtonDown() || !NativeMethods.IsWindow(_window))
        {
            EndGesture(!NativeMethods.IsWindow(_window)
                ? DragSnapEndReason.WindowUnavailable
                : DragSnapEndReason.CaptureLost);
            return;
        }

        if (!_dragStarted)
        {
            var dx = point.X - _startPoint.X;
            var dy = point.Y - _startPoint.Y;
            if (dx * dx + dy * dy < DragStartDistance * DragStartDistance)
            {
                return;
            }

            _dragStarted = true;
        }

        if (!TryGetTarget(point, out var target))
        {
            if (_target.HasValue)
            {
                _target = null;
                TargetCleared?.Invoke();
            }

            return;
        }

        if (_target == target)
        {
            return;
        }

        _target = target;
        _hadCandidate = true;
        TargetChanged?.Invoke(target);
    }

    private void CaptureTimer_Tick(object? sender, EventArgs e)
    {
        if (!_leftButtonDown)
        {
            return;
        }

        if (!IsLeftButtonDown())
        {
            EndGesture(DragSnapEndReason.CaptureLost);
        }
        else if (!NativeMethods.IsWindow(_window))
        {
            EndGesture(DragSnapEndReason.WindowUnavailable);
        }
    }

    private void EndGesture(DragSnapEndReason reason)
    {
        if (!_leftButtonDown)
        {
            return;
        }

        var gesture = new DragSnapGesture(_window, _originalFrame, _target, reason);
        var hadTarget = _target.HasValue;
        var hadCandidate = _hadCandidate;
        ResetGesture();

        if (hadTarget)
        {
            TargetCleared?.Invoke();
        }

        if (hadCandidate)
        {
            GestureEnded?.Invoke(gesture);
        }
    }

    private void ResetGesture()
    {
        _captureTimer.Stop();
        _leftButtonDown = false;
        _dragStarted = false;
        _window = IntPtr.Zero;
        _originalFrame = default;
        _startPoint = default;
        _target = null;
        _hadCandidate = false;
    }

    private bool TryGetTarget(NativeMethods.Point point, out DragSnapTarget target)
    {
        target = default;
        var monitor = NativeMethods.MonitorFromPoint(point, NativeMethods.MonitorDefaultToNearest);
        var info = new NativeMethods.MonitorInfo
        {
            Size = Marshal.SizeOf<NativeMethods.MonitorInfo>()
        };
        if (monitor == IntPtr.Zero || !NativeMethods.GetMonitorInfo(monitor, ref info) ||
            !DragSnapGeometry.TryResolve(info.Monitor, info.Work, point, _settings.DragSnapThreshold, out var zone))
        {
            return false;
        }

        var action = DragSnapGeometry.ActionOf(zone);
        target = new DragSnapTarget(action, WindowFrameMath.ZoneFrame(info.Work, action));
        return true;
    }

    private static bool CanBeginDrag(IntPtr window, NativeMethods.Point point)
    {
        if (!WindowQuery.IsEligibleForSnap(window) || NativeMethods.IsZoomed(window))
        {
            return false;
        }

        var style = NativeMethods.GetWindowLongPtr(window, NativeMethods.GwlStyle).ToInt64();
        if ((style & NativeMethods.WsCaption) == 0)
        {
            return false;
        }

        var result = NativeMethods.SendMessageTimeout(
            window,
            NativeMethods.WmNcHitTest,
            IntPtr.Zero,
            PackPoint(point),
            NativeMethods.SmtoAbortIfHung,
            100,
            out var hitTest);
        return result != IntPtr.Zero && hitTest.ToUInt64() == NativeMethods.HtCaption;
    }

    private static IntPtr PackPoint(NativeMethods.Point point) =>
        new((long)(uint)point.X | ((long)(uint)point.Y << 32));

    private static bool IsLeftButtonDown() =>
        (NativeMethods.GetAsyncKeyState(NativeMethods.VkLButton) & unchecked((short)0x8000)) != 0;
}
