using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Threading;
using ThreadTimer = System.Threading.Timer;

namespace LoopW;

/// <summary>
/// Global hotkey service built on a WH_KEYBOARD_LL hook so that essentially any
/// key can be bound — including Caps Lock and other single keys that
/// RegisterHotKey refuses. The bound key is swallowed system-wide, which also
/// stops Caps Lock from ever toggling while it is the trigger.
/// </summary>
public sealed class GlobalHotkey : IDisposable
{
    private const int WhKeyboardLl = 13;
    private const int WhMouseLl = 14;
    private const int WmKeyDown = 0x0100;
    private const int WmKeyUp = 0x0101;
    private const int WmSysKeyDown = 0x0104;
    private const int WmSysKeyUp = 0x0105;

    private const uint VkShift = 0x10;
    private const uint VkControl = 0x11;
    private const uint VkMenu = 0x12;
    private const uint VkLWin = 0x5B;
    private const uint VkRWin = 0x5C;
    private const uint VkEscape = 0x1B;

    private readonly NativeMethods.KeyboardHookProc _hookProc;
    private readonly NativeMethods.MouseHookProc _mouseHookProc;
    private readonly object _stateGate = new();
    private IntPtr _hookHandle;
    private IntPtr _mouseHookHandle;
    private uint _triggerVk;
    private uint _triggerModifiers;
    private TriggerModifierSide _triggerModifierSide;
    private int _triggerDelayMilliseconds;
    private int _triggerTimeoutMilliseconds;
    private bool _doubleClickToTrigger;
    private bool _middleClickToTrigger;
    private bool _triggerDown;
    private bool _middleTriggerDown;
    private bool _triggerActivated;
    private bool _triggerTimedOut;
    private bool _capturing;
    private long _lastTriggerReleaseAt = -1;
    private long _stateVersion;
    private ThreadTimer? _activationTimer;
    private ThreadTimer? _timeoutTimer;
    private readonly HashSet<uint> _pressedKeybinds = new();
    private List<Keybind> _keybinds = new();
    private Action<uint, uint>? _captureTarget;
    private Action? _captureCancelled;
    private Action? _captureRejected;

    public GlobalHotkey()
    {
        _hookProc = HookProc;
        _mouseHookProc = MouseHookProc;
    }

    public uint TriggerVk => _triggerVk;

    public uint TriggerModifiers => _triggerModifiers;

    public TriggerModifierSide TriggerModifierSide => _triggerModifierSide;

    public bool IsActive => _hookHandle != IntPtr.Zero;

    public event Action? TriggerPressed;

    public event Action? TriggerReleased;

    public event Action? TriggerCancelled;

    public event Action? TriggerTimedOut;

    public event Action<uint, uint>? KeyCaptured;

    public event Action? CaptureCancelled;

    public event Action? CaptureRejected;

    /// <summary>
    /// Raised when a bound key goes down while the trigger is held. The key is
    /// swallowed system-wide so it never reaches the focused app.
    /// </summary>
    public event Action<Keybind>? KeybindPressed;

    public void Start()
    {
        lock (_stateGate)
        {
            if (_hookHandle == IntPtr.Zero)
            {
                _hookHandle = NativeMethods.SetWindowsHookEx(WhKeyboardLl, _hookProc, IntPtr.Zero, 0);
            }

            UpdateMouseHookLocked();
        }
    }

    public void SetBinding(uint modifiers, uint vk)
    {
        lock (_stateGate)
        {
            _triggerModifiers = modifiers;
            _triggerVk = vk;
            ResetInputStateLocked(notify: true);
        }
    }

    public void SetTriggerBehavior(
        TriggerModifierSide modifierSide,
        int delayMilliseconds,
        int timeoutMilliseconds,
        bool doubleClickToTrigger,
        bool middleClickToTrigger)
    {
        lock (_stateGate)
        {
            _triggerModifierSide = Enum.IsDefined(modifierSide)
                ? modifierSide
                : TriggerModifierSide.Any;
            _triggerDelayMilliseconds = Math.Clamp(delayMilliseconds, 0, 1000);
            _triggerTimeoutMilliseconds = Math.Clamp(timeoutMilliseconds, 0, 10000);
            _doubleClickToTrigger = doubleClickToTrigger;
            _middleClickToTrigger = middleClickToTrigger;
            ResetInputStateLocked(notify: true);
            UpdateMouseHookLocked();
        }
    }

    public void SetKeybinds(IReadOnlyList<Keybind> keybinds)
    {
        lock (_stateGate)
        {
            _keybinds = keybinds == null ? new List<Keybind>() : new List<Keybind>(keybinds);
        }
    }

    /// <summary>
    /// Begins capture, routing the result to this instance's Capture events.
    /// </summary>
    public void BeginCapture()
    {
        lock (_stateGate)
        {
            _captureTarget = null;
            _captureCancelled = null;
            _captureRejected = null;
            _capturing = true;
            ResetInputStateLocked(notify: true);
        }
    }

    /// <summary>
    /// Begins capture, routing the result to the supplied one-shot callbacks instead
    /// of this instance's Capture events. Used by the settings window so it can
    /// capture keybinds without tripping the main window's trigger-rebind handlers.
    /// </summary>
    public void BeginCapture(Action<uint, uint> captured, Action cancelled, Action rejected)
    {
        lock (_stateGate)
        {
            _captureTarget = captured;
            _captureCancelled = cancelled;
            _captureRejected = rejected;
            _capturing = true;
            ResetInputStateLocked(notify: true);
        }
    }

    public void Dispose()
    {
        lock (_stateGate)
        {
            if (_hookHandle != IntPtr.Zero)
            {
                NativeMethods.UnhookWindowsHookEx(_hookHandle);
                _hookHandle = IntPtr.Zero;
            }

            if (_mouseHookHandle != IntPtr.Zero)
            {
                NativeMethods.UnhookWindowsHookEx(_mouseHookHandle);
                _mouseHookHandle = IntPtr.Zero;
            }

            ResetInputStateLocked(notify: false);
        }

        GC.SuppressFinalize(this);
    }

    private IntPtr HookProc(int nCode, IntPtr wParam, IntPtr lParam)
    {
        using var performance = PerformanceDiagnostics.Measure(PerformanceMetric.HotkeyHook);
        if (nCode >= 0)
        {
            var message = wParam.ToInt32();
            var isDown = message == WmKeyDown || message == WmSysKeyDown;
            var isUp = message == WmKeyUp || message == WmSysKeyUp;

            if (isDown || isUp)
            {
                var data = Marshal.PtrToStructure<NativeMethods.KbdLlHookStruct>(lParam);
                lock (_stateGate)
                {
                    var suppress = isDown ? HandleKeyDownLocked(data.VkCode) : HandleKeyUpLocked(data.VkCode);
                    if (suppress)
                    {
                        return (IntPtr)1;
                    }
                }
            }
        }

        return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
    }

    private IntPtr MouseHookProc(int nCode, IntPtr wParam, IntPtr lParam)
    {
        using var performance = PerformanceDiagnostics.Measure(PerformanceMetric.MouseHook);
        if (nCode >= 0)
        {
            var message = wParam.ToInt32();
            if (message == NativeMethods.WmMButtonDown || message == NativeMethods.WmMButtonUp)
            {
                lock (_stateGate)
                {
                    var suppress = message == NativeMethods.WmMButtonDown
                        ? HandleMiddleButtonDownLocked()
                        : HandleMiddleButtonUpLocked();
                    if (suppress)
                    {
                        return (IntPtr)1;
                    }
                }
            }
        }

        return NativeMethods.CallNextHookEx(_mouseHookHandle, nCode, wParam, lParam);
    }

    private bool HandleKeyDownLocked(uint vk)
    {
        if (_capturing)
        {
            return HandleCaptureLocked(vk);
        }

        if (vk == _triggerVk && ModifiersMatch(_triggerModifiers, _triggerModifierSide))
        {
            if (!_triggerDown)
            {
                _triggerDown = true;
                if (!_middleTriggerDown)
                {
                    BeginTriggerPressLocked();
                }
            }

            return true;
        }

        if (AnyTriggerHeld && !_triggerTimedOut && TryMatchKeybindLocked(vk, bypassTrigger: false))
        {
            return true;
        }

        if (!AnyTriggerHeld && TryMatchKeybindLocked(vk, bypassTrigger: true))
        {
            return true;
        }

        return false;
    }

    private bool TryMatchKeybindLocked(uint vk, bool bypassTrigger)
    {
        // Low-level hooks repeat WM_KEYDOWN while a key is held. A cycle advances
        // per physical press, not per repeat tick.
        if (_pressedKeybinds.Contains(vk))
        {
            return true;
        }

        for (var i = 0; i < _keybinds.Count; i++)
        {
            var keybind = _keybinds[i];
            if (keybind.Vk != vk || keybind.Vk == _triggerVk ||
                keybind.BypassTrigger != bypassTrigger || !ModifiersMatch(keybind.Modifiers, TriggerModifierSide.Any))
            {
                continue;
            }

            var captured = keybind;
            _pressedKeybinds.Add(vk);
            Dispatch(() => KeybindPressed?.Invoke(captured));
            return true;
        }

        return false;
    }

    private bool HandleKeyUpLocked(uint vk)
    {
        _pressedKeybinds.Remove(vk);

        if (_capturing)
        {
            return false;
        }

        if (vk != _triggerVk || !_triggerDown)
        {
            return false;
        }

        _triggerDown = false;
        if (!AnyTriggerHeld)
        {
            CompleteTriggerReleaseLocked();
        }

        return true;
    }

    private bool HandleMiddleButtonDownLocked()
    {
        if (_capturing || !_middleClickToTrigger)
        {
            return false;
        }

        if (!_middleTriggerDown)
        {
            _middleTriggerDown = true;
            if (!_triggerDown)
            {
                BeginTriggerPressLocked();
            }
        }

        return true;
    }

    private bool HandleMiddleButtonUpLocked()
    {
        if (!_middleTriggerDown)
        {
            return false;
        }

        _middleTriggerDown = false;
        if (!AnyTriggerHeld)
        {
            CompleteTriggerReleaseLocked();
        }

        return true;
    }

    private bool HandleCaptureLocked(uint vk)
    {
        if (IsModifierKey(vk))
        {
            return false;
        }

        if (vk == VkEscape)
        {
            _capturing = false;
            if (_captureCancelled != null)
            {
                Dispatch(_captureCancelled);
            }
            else
            {
                Dispatch(() => CaptureCancelled?.Invoke());
            }

            return true;
        }

        var mods = CurrentModifiers(TriggerModifierSide.Any);
        if ((mods & NativeMethods.ModWin) != 0)
        {
            _capturing = false;
            if (_captureRejected != null)
            {
                Dispatch(_captureRejected);
            }
            else
            {
                Dispatch(() => CaptureRejected?.Invoke());
            }

            return true;
        }

        _capturing = false;
        if (_captureTarget != null)
        {
            var capturedMods = mods;
            var capturedVk = vk;
            Dispatch(() => _captureTarget(capturedMods, capturedVk));
        }
        else
        {
            Dispatch(() => KeyCaptured?.Invoke(mods, vk));
        }

        return true;
    }

    private void BeginTriggerPressLocked()
    {
        if (_doubleClickToTrigger)
        {
            var now = Environment.TickCount64;
            var isSecondClick = _lastTriggerReleaseAt >= 0 &&
                now - _lastTriggerReleaseAt <= NativeMethods.GetDoubleClickTime();
            _lastTriggerReleaseAt = -1;
            if (!isSecondClick)
            {
                return;
            }
        }

        ScheduleActivationLocked();
    }

    private void ScheduleActivationLocked()
    {
        _activationTimer?.Dispose();
        _activationTimer = null;
        var version = ++_stateVersion;
        if (_triggerDelayMilliseconds == 0)
        {
            ActivateTriggerLocked(version);
            return;
        }

        _activationTimer = new ThreadTimer(
            _ =>
            {
                lock (_stateGate)
                {
                    if (version == _stateVersion && AnyTriggerHeld)
                    {
                        _activationTimer?.Dispose();
                        _activationTimer = null;
                        ActivateTriggerLocked(version);
                    }
                }
            },
            null,
            _triggerDelayMilliseconds,
            Timeout.Infinite);
    }

    private void ActivateTriggerLocked(long version)
    {
        if (version != _stateVersion || !AnyTriggerHeld || _triggerActivated || _triggerTimedOut)
        {
            return;
        }

        _triggerActivated = true;
        Dispatch(() => TriggerPressed?.Invoke());
        if (_triggerTimeoutMilliseconds > 0)
        {
            _timeoutTimer?.Dispose();
            _timeoutTimer = new ThreadTimer(
                _ =>
                {
                    lock (_stateGate)
                    {
                        if (version != _stateVersion || !AnyTriggerHeld || !_triggerActivated)
                        {
                            return;
                        }

                        _triggerActivated = false;
                        _triggerTimedOut = true;
                        _timeoutTimer?.Dispose();
                        _timeoutTimer = null;
                        _stateVersion++;
                        _pressedKeybinds.Clear();
                        Dispatch(() => TriggerTimedOut?.Invoke());
                    }
                },
                null,
                _triggerTimeoutMilliseconds,
                Timeout.Infinite);
        }
    }

    private void CompleteTriggerReleaseLocked()
    {
        _activationTimer?.Dispose();
        _activationTimer = null;
        _timeoutTimer?.Dispose();
        _timeoutTimer = null;
        ++_stateVersion;

        var wasActivated = _triggerActivated;
        _triggerActivated = false;
        _triggerTimedOut = false;
        _pressedKeybinds.Clear();
        _lastTriggerReleaseAt = _doubleClickToTrigger ? Environment.TickCount64 : -1;
        if (wasActivated)
        {
            Dispatch(() => TriggerReleased?.Invoke());
        }
    }

    private bool AnyTriggerHeld => _triggerDown || _middleTriggerDown;

    private void UpdateMouseHookLocked()
    {
        if (_hookHandle == IntPtr.Zero || !_middleClickToTrigger)
        {
            if (_mouseHookHandle != IntPtr.Zero)
            {
                NativeMethods.UnhookWindowsHookEx(_mouseHookHandle);
                _mouseHookHandle = IntPtr.Zero;
            }

            return;
        }

        if (_mouseHookHandle == IntPtr.Zero)
        {
            _mouseHookHandle = NativeMethods.SetWindowsHookEx(WhMouseLl, _mouseHookProc, IntPtr.Zero, 0);
        }
    }

    private void ResetInputStateLocked(bool notify)
    {
        var wasActive = _triggerActivated;
        _activationTimer?.Dispose();
        _activationTimer = null;
        _timeoutTimer?.Dispose();
        _timeoutTimer = null;
        ++_stateVersion;
        _triggerDown = false;
        _middleTriggerDown = false;
        _triggerActivated = false;
        _triggerTimedOut = false;
        _lastTriggerReleaseAt = -1;
        _pressedKeybinds.Clear();
        if (notify && wasActive)
        {
            Dispatch(() => TriggerCancelled?.Invoke());
        }
    }

    private static bool IsModifierKey(uint vk) =>
        vk is VkShift or VkControl or VkMenu or VkLWin or VkRWin or 0xA0 or 0xA1 or 0xA2 or 0xA3 or 0xA4 or 0xA5;

    private static uint CurrentModifiers(TriggerModifierSide side)
    {
        var mods = 0u;
        var shiftDown = side switch
        {
            TriggerModifierSide.Left => IsDown(0xA0),
            TriggerModifierSide.Right => IsDown(0xA1),
            _ => IsDown(VkShift)
        };
        var controlDown = side switch
        {
            TriggerModifierSide.Left => IsDown(0xA2),
            TriggerModifierSide.Right => IsDown(0xA3),
            _ => IsDown(VkControl)
        };
        var altDown = side switch
        {
            TriggerModifierSide.Left => IsDown(0xA4),
            TriggerModifierSide.Right => IsDown(0xA5),
            _ => IsDown(VkMenu)
        };
        var winDown = side switch
        {
            TriggerModifierSide.Left => IsDown(VkLWin),
            TriggerModifierSide.Right => IsDown(VkRWin),
            _ => IsDown(VkLWin) || IsDown(VkRWin)
        };

        if (shiftDown)
        {
            mods |= NativeMethods.ModShift;
        }

        if (controlDown)
        {
            mods |= NativeMethods.ModControl;
        }

        if (altDown)
        {
            mods |= NativeMethods.ModAlt;
        }

        if (winDown)
        {
            mods |= NativeMethods.ModWin;
        }

        return mods;
    }

    private static bool IsDown(uint vk) => vk != 0 && (NativeMethods.GetAsyncKeyState((int)vk) & 0x8000) != 0;

    private static bool ModifiersMatch(uint expected, TriggerModifierSide side) =>
        CurrentModifiers(side) == expected;

    private static void Dispatch(Action action)
    {
        System.Windows.Application.Current?.Dispatcher?.BeginInvoke(action, DispatcherPriority.Input);
    }
}

public static class HotkeyNames
{
    public static string For(uint modifiers, uint vk, TriggerModifierSide side = TriggerModifierSide.Any)
    {
        var parts = new List<string>(4);
        var sideLabel = side switch
        {
            TriggerModifierSide.Left => "Left",
            TriggerModifierSide.Right => "Right",
            _ => string.Empty
        };
        if ((modifiers & NativeMethods.ModControl) != 0)
        {
            parts.Add(string.IsNullOrEmpty(sideLabel) ? "Ctrl" : $"{sideLabel} Ctrl");
        }

        if ((modifiers & NativeMethods.ModAlt) != 0)
        {
            parts.Add(string.IsNullOrEmpty(sideLabel) ? "Alt" : $"{sideLabel} Alt");
        }

        if ((modifiers & NativeMethods.ModShift) != 0)
        {
            parts.Add(string.IsNullOrEmpty(sideLabel) ? "Shift" : $"{sideLabel} Shift");
        }

        if ((modifiers & NativeMethods.ModWin) != 0)
        {
            parts.Add(string.IsNullOrEmpty(sideLabel) ? "Win" : $"{sideLabel} Win");
        }

        parts.Add(KeyName(vk));
        return string.Join(" + ", parts);
    }

    public static string KeyName(uint vk)
    {
        return vk switch
        {
            0x08 => "Backspace",
            0x09 => "Tab",
            0x0D => "Enter",
            0x13 => "Pause",
            0x1B => "Esc",
            0x14 => "Caps Lock",
            0x20 => "Space",
            0x21 => "PgUp",
            0x22 => "PgDn",
            0x23 => "End",
            0x24 => "Home",
            0x25 => "Left",
            0x26 => "Up",
            0x27 => "Right",
            0x28 => "Down",
            0x2C => "PrtScr",
            0x2D => "Ins",
            0x2E => "Del",
            0x5D => "Menu",
            0x90 => "Num Lock",
            0x91 => "Scroll Lock",
            _ when vk >= 0x30 && vk <= 0x39 => ((char)vk).ToString(),
            _ when vk >= 0x41 && vk <= 0x5A => ((char)vk).ToString(),
            _ when vk >= 0x60 && vk <= 0x69 => $"Num {vk - 0x60}",
            0x6A => "Num *",
            0x6B => "Num +",
            0x6D => "Num -",
            0x6E => "Num .",
            0x6F => "Num /",
            _ when vk >= 0x70 && vk <= 0x87 => $"F{vk - 0x6F}",
            0xBA => ";",
            0xBB => "=",
            0xBC => ",",
            0xBD => "-",
            0xBE => ".",
            0xBF => "/",
            0xC0 => "`",
            0xDB => "[",
            0xDC => "\\",
            0xDD => "]",
            0xDE => "'",
            _ => System.Windows.Input.KeyInterop.KeyFromVirtualKey((int)vk).ToString()
        };
    }
}
