using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace LoopW;

public partial class SettingsWindow : Window
{
    public static IReadOnlyList<KeyValuePair<WindowAction, string>> ActionChoices { get; } =
        Enum.GetValues<WindowAction>()
            .Select(a => new KeyValuePair<WindowAction, string>(a, WindowActionService.ActionName(a)))
            .ToArray();

    private readonly GlobalHotkey _hotkey;
    private readonly ObservableCollection<KeybindRow> _rows = new();
    private bool _capturingUi;

    public SettingsWindow(GlobalHotkey hotkey)
    {
        InitializeComponent();
        _hotkey = hotkey;
        KeybindList.ItemsSource = _rows;
    }

    private void Window_SourceInitialized(object? sender, EventArgs e)
    {
        var hwnd = new System.Windows.Interop.WindowInteropHelper(this).Handle;
        if (hwnd == IntPtr.Zero)
        {
            return;
        }

        foreach (var attribute in new[] { NativeMethods.DwmwaUseImmersiveDarkMode, NativeMethods.DwmwaUseImmersiveDarkModeBefore20h1 })
        {
            var enabled = 1;
            if (NativeMethods.DwmSetWindowAttribute(hwnd, attribute, ref enabled, sizeof(int)) == 0)
            {
                break;
            }
        }
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        var settings = AppSettings.Load();
        foreach (var keybind in settings.Keybinds)
        {
            _rows.Add(new KeybindRow(keybind));
        }

        _hotkey.SetKeybinds(settings.Keybinds);
    }

    private void Add_Click(object sender, RoutedEventArgs e)
    {
        if (_capturingUi)
        {
            return;
        }

        SetCapturingUi(true);
        _hotkey.BeginCapture(
            (mods, vk) =>
            {
                SetCapturingUi(false);
                _rows.Add(new KeybindRow(new Keybind(mods, vk, WindowAction.RightHalf)));
                SaveAll();
                StatusText.Text = $"  ·  Added {HotkeyNames.For(mods, vk)} — pick an action from the list";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "  ·  Keybind capture cancelled";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "  ·  That key is reserved by the OS — try another";
            });
    }

    private void KeyChip_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (_capturingUi || sender is not TextBlock { DataContext: KeybindRow row })
        {
            return;
        }

        SetCapturingUi(true);
        _hotkey.BeginCapture(
            (mods, vk) =>
            {
                SetCapturingUi(false);
                row.Keybind.Modifiers = mods;
                row.Keybind.Vk = vk;
                row.Refresh();
                SaveAll();
                StatusText.Text = $"  ·  Rebound to {HotkeyNames.For(mods, vk)}";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "  ·  Rebind cancelled";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "  ·  That key is reserved by the OS — try another";
            });
    }

    private void Action_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (sender is ComboBox { DataContext: KeybindRow row } combo)
        {
            row.Keybind.Action = (WindowAction)combo.SelectedValue;
            SaveAll();
        }
    }

    private void Delete_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: KeybindRow row })
        {
            _rows.Remove(row);
            SaveAll();
        }
    }

    private void SetCapturingUi(bool capturing)
    {
        _capturingUi = capturing;
        AddButton.IsEnabled = !capturing;
        StatusText.Text = capturing ? "Press a key or combo — Esc to cancel" : " ";
    }

    private void SaveAll()
    {
        var settings = AppSettings.Load();
        settings.Keybinds = _rows.Select(r => r.Keybind).ToList();
        settings.Save();
        _hotkey.SetKeybinds(settings.Keybinds);
    }
}

/// <summary>
/// A single keybind row in the settings window. Wraps the persisted Keybind and
/// raises change notifications so the key chip re-renders after a rebind.
/// </summary>
public sealed class KeybindRow : INotifyPropertyChanged
{
    public KeybindRow(Keybind keybind)
    {
        Keybind = keybind;
    }

    public Keybind Keybind { get; }

    public string Display => HotkeyNames.For(Keybind.Modifiers, Keybind.Vk);

    public void Refresh() => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Display)));

    public event PropertyChangedEventHandler? PropertyChanged;
}
