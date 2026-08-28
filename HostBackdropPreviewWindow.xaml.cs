using System;
using System.Runtime.Versioning;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;

#pragma warning disable CA1416

namespace LoopW;

/// <summary>
/// Live desktop preview backed by Windows 11's native DWM Desktop Acrylic
/// backdrop. It has no Win2D or Visual C++ runtime dependency. The bitmap
/// preview remains available in <see cref="PreviewOverlayWindow"/> when the
/// system backdrop is unavailable or disabled.
/// </summary>
[SupportedOSPlatform("windows10.0.22621")]
public partial class HostBackdropPreviewWindow : Window
{
    private readonly AppSettings _settings;
    private bool _backdropEnabled;
    private bool _initializationFailed;

    internal HostBackdropPreviewWindow(AppSettings settings)
    {
        InitializeComponent();
        _settings = settings;
        ApplySurfaceSettings();
        SourceInitialized += OnSourceInitialized;
    }

    internal bool TryShowFrame(NativeMethods.Rect frame)
    {
        if (_initializationFailed ||
            !OperatingSystem.IsWindowsVersionAtLeast(10, 0, 22621))
        {
            return false;
        }

        var monitor = NativeMethods.MonitorFromRect(ref frame, NativeMethods.MonitorDefaultToNearest);
        if (!NativeMethods.TryGetMonitorDpi(monitor, out var dpiX, out var dpiY))
        {
            dpiX = 96;
            dpiY = 96;
        }

        var scaleX = 96.0 / dpiX;
        var scaleY = 96.0 / dpiY;
        Left = frame.Left * scaleX + 8;
        Top = frame.Top * scaleY + 8;
        Width = Math.Max(frame.Width * scaleX - 16, 1);
        Height = Math.Max(frame.Height * scaleY - 16, 1);

        if (!IsVisible)
        {
            try
            {
                // Opacity starts at zero, so the native backdrop can be
                // configured before the preview becomes visible.
                Show();
            }
            catch
            {
                _initializationFailed = true;
                return false;
            }
        }

        if (!_backdropEnabled)
        {
            Hide();
            return false;
        }

        ApplySurfaceSettings();
        Opacity = 1;
        return true;
    }

    internal void HidePreview(bool destroy = false)
    {
        Opacity = 0;
        if (destroy)
        {
            Close();
        }
        else
        {
            Hide();
        }
    }

    private void OnSourceInitialized(object? sender, EventArgs e)
    {
        var hwnd = new WindowInteropHelper(this).Handle;
        NativeMethods.MakeMouseClickThrough(hwnd);

        // WPF otherwise clears the client surface to opaque black before DWM
        // can draw the system backdrop behind it. Keep the composition target
        // transparent so the live desktop material remains visible.
        if (HwndSource.FromHwnd(hwnd)?.CompositionTarget is { } compositionTarget)
        {
            compositionTarget.BackgroundColor = Colors.Transparent;
        }

        // Extend the DWM frame through the client area. This lets the system
        // backdrop occupy the same pixels as the WPF preview surface.
        var margins = new NativeMethods.Margins
        {
            Left = -1,
            Right = -1,
            Top = -1,
            Bottom = -1
        };
        if (NativeMethods.DwmExtendFrameIntoClientArea(hwnd, ref margins) != 0)
        {
            _initializationFailed = true;
            return;
        }

        var backdropType = NativeMethods.DwmSystemBackdropTransientWindow;
        if (NativeMethods.DwmSetWindowAttribute(
                hwnd,
                NativeMethods.DwmwaSystemBackdropType,
                ref backdropType,
                sizeof(int)) != 0)
        {
            _initializationFailed = true;
            return;
        }

        var cornerPreference = NativeMethods.DwmWindowCornerRound;
        NativeMethods.DwmSetWindowAttribute(
            hwnd,
            NativeMethods.DwmwaWindowCornerPreference,
            ref cornerPreference,
            sizeof(int));

        var darkMode = _settings.IsLightAppearance ? 0 : 1;
        NativeMethods.DwmSetWindowAttribute(
            hwnd,
            NativeMethods.DwmwaUseImmersiveDarkMode,
            ref darkMode,
            sizeof(int));

        _backdropEnabled = true;
    }

    private void ApplySurfaceSettings()
    {
        SurfaceElement.CornerRadius = new CornerRadius(_settings.PreviewCornerRadius);
        SurfaceElement.BorderThickness = new Thickness(_settings.PreviewBorderWidth);
        SurfaceElement.Background = new SolidColorBrush(ParseColor(
            _settings.IsLightAppearance ? "#30FFFFFF" : "#30101827",
            "#30101827"));
        SurfaceElement.BorderBrush = new SolidColorBrush(ParseColor(
            _settings.PreviewBorderColor,
            "#B8007AFF"));
    }

    private static Color ParseColor(string? value, string fallback)
    {
        try
        {
            if (ColorConverter.ConvertFromString(value ?? string.Empty) is Color color)
            {
                return color;
            }
        }
        catch
        {
            // Invalid settings use the fallback color.
        }

        return (Color)ColorConverter.ConvertFromString(fallback)!;
    }
}
