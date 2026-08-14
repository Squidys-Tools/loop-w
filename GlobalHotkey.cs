using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Windows.Threading;

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
    private IntPtr _hookHandle;
    private uint _triggerVk;
    private uint _triggerModifiers;
    private bool _triggerDown;
    private bool _capturing;

    public GlobalHotkey()
    {
        _hookProc = HookProc;
    }

    public uint TriggerVk => _triggerVk;

    public uint TriggerModifiers => _triggerModifiers;

    public bool IsActive => _hookHandle != IntPtr.Zero;

    public event Action? TriggerPressed;

    public event Action? TriggerReleased;

    public event Action<uint, uint>? KeyCaptured;

    public event Action? CaptureCancelled;

    public event Action? CaptureRejected;

    public void Start()
    {
        if (_hookHandle != IntPtr.Zero)
        {
            return;
        }

        _hookHandle = NativeMethods.SetWindowsHookEx(WhKeyboardLl, _hookProc, IntPtr.Zero, 0);
    }

    public void SetBinding(uint modifiers, uint vk)
    {
        _triggerModifiers = modifiers;
        _triggerVk = vk;
        _triggerDown = false;
    }

    public void BeginCapture()
    {
        _capturing = true;
    }

    public void Dispose()
    {
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
            int message = wParam.ToInt32();
            bool isDown = message == WmKeyDown || message == WmSysKeyDown;
            bool isUp = message == WmKeyUp || message == WmSysKeyUp;

            if (isDown || isUp)
            {
                var data = Marshal.PtrToStructure<NativeMethods.KbdLlHookStruct>(lParam);
                bool suppress = isDown ? HandleKeyDown(data.VkCode) : HandleKeyUp(data.VkCode);
                if (suppress)
                {
                    return (IntPtr)1;
                }
            }
        }

        return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
    }

    private bool HandleKeyDown(uint vk)
    {
        if (_capturing)
        {
            return HandleCapture(vk);
        }

        if (vk != _triggerVk || !ModifiersMatch(_triggerModifiers))
        {
            return false;
        }

        if (!_triggerDown)
        {
            _triggerDown = true;
            Dispatch(() => TriggerPressed?.Invoke());
        }

        return true;
    }

    private bool HandleKeyUp(uint vk)
    {
        if (_capturing)
        {
            return false;
        }

        if (vk != _triggerVk || !_triggerDown)
        {
            return false;
        }

        _triggerDown = false;
        Dispatch(() => TriggerReleased?.Invoke());
        return true;
    }

    private bool HandleCapture(uint vk)
    {
        if (IsModifierKey(vk))
        {
            return false;
        }

        if (vk == VkEscape)
        {
            _capturing = false;
            Dispatch(() => CaptureCancelled?.Invoke());
            return true;
        }

        var mods = CurrentModifiers();
        if ((mods & NativeMethods.ModWin) != 0)
        {
            _capturing = false;
            Dispatch(() => CaptureRejected?.Invoke());
            return true;
        }

        _capturing = false;
        Dispatch(() => KeyCaptured?.Invoke(mods, vk));
        return true;
    }

    private static bool IsModifierKey(uint vk) =>
        vk is VkShift or VkControl or VkMenu or VkLWin or VkRWin;

    private static uint CurrentModifiers()
    {
        var mods = 0u;
        if ((NativeMethods.GetAsyncKeyState((int)VkShift) & 0x8000) != 0)
        {
            mods |= NativeMethods.ModShift;
        }

        if ((NativeMethods.GetAsyncKeyState((int)VkControl) & 0x8000) != 0)
        {
            mods |= NativeMethods.ModControl;
        }

        if ((NativeMethods.GetAsyncKeyState((int)VkMenu) & 0x8000) != 0)
        {
            mods |= NativeMethods.ModAlt;
        }

        if ((NativeMethods.GetAsyncKeyState((int)VkLWin) & 0x8000) != 0 ||
            (NativeMethods.GetAsyncKeyState((int)VkRWin) & 0x8000) != 0)
        {
            mods |= NativeMethods.ModWin;
        }

        return mods;
    }

    private static bool ModifiersMatch(uint expected) => CurrentModifiers() == expected;

    private static void Dispatch(Action action)
    {
        System.Windows.Application.Current?.Dispatcher?.BeginInvoke(action, DispatcherPriority.Input);
    }
}

public static class HotkeyNames
{
    public static string For(uint modifiers, uint vk)
    {
        var parts = new List<string>(4);
        if ((modifiers & NativeMethods.ModControl) != 0)
        {
            parts.Add("Ctrl");
        }

        if ((modifiers & NativeMethods.ModAlt) != 0)
        {
            parts.Add("Alt");
        }

        if ((modifiers & NativeMethods.ModShift) != 0)
        {
            parts.Add("Shift");
        }

        if ((modifiers & NativeMethods.ModWin) != 0)
        {
            parts.Add("Win");
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
