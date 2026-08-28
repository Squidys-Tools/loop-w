using System;
using System.Numerics;
using System.Runtime.Versioning;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using Microsoft.Graphics.Canvas.Effects;
using Windows.UI.Composition;
using MediaColor = System.Windows.Media.Color;

#pragma warning disable CA1416

namespace LoopW;

/// <summary>
/// Live desktop backdrop preview rendered by Windows Composition and Win2D.
/// The bitmap preview remains available in <see cref="PreviewOverlayWindow"/>
/// when the native composition path cannot start.
/// </summary>
[SupportedOSPlatform("windows10.0.17134")]
public partial class HostBackdropPreviewWindow : Window
{
    // A low radius keeps the desktop readable while still removing sharp detail.
    // The old prototype used 20, which made the material look like a flat fill.
    private const float LiveBlurAmount = 7f;

    private readonly CompositionHost _compositionHost;
    private readonly ShapeVisual _surfaceVisual = null!;
    private readonly CompositionRoundedRectangleGeometry _backdropGeometry = null!;
    private readonly CompositionRoundedRectangleGeometry _tintGeometry = null!;
    private readonly CompositionRoundedRectangleGeometry _borderGeometry = null!;
    private readonly CompositionSpriteShape _backdropShape = null!;
    private readonly CompositionSpriteShape _tintShape = null!;
    private readonly CompositionSpriteShape _borderShape = null!;
    private readonly CompositionColorBrush _tintBrush = null!;
    private readonly CompositionColorBrush _borderBrush = null!;
    private readonly CompositionBackdropBrush _hostBackdropBrush = null!;
    private readonly CompositionEffectBrush _backdropEffectBrush = null!;
    private readonly AppSettings _settings;
    private bool _initialized;
    private bool _initializationFailed;

    internal HostBackdropPreviewWindow(AppSettings settings)
    {
        InitializeComponent();
        _settings = settings;
        _compositionHost = CompositionHostElement;

        SourceInitialized += OnSourceInitialized;
        Loaded += OnLoaded;

        try
        {
            var compositor = _compositionHost.Compositor;
            _surfaceVisual = compositor.CreateShapeVisual();
            _hostBackdropBrush = compositor.CreateHostBackdropBrush();
            var blurEffect = new GaussianBlurEffect
            {
                Name = "BackdropBlur",
                BlurAmount = LiveBlurAmount,
                BorderMode = EffectBorderMode.Hard,
                Source = new CompositionEffectSourceParameter("Backdrop")
            };
            _backdropEffectBrush = compositor
                .CreateEffectFactory(blurEffect)
                .CreateBrush();
            _backdropEffectBrush.SetSourceParameter("Backdrop", _hostBackdropBrush);
            _tintBrush = compositor.CreateColorBrush(ToCompositionColor(
                settings.IsLightAppearance ? "#30FFFFFF" : "#30101827",
                "#30101827"));
            _borderBrush = compositor.CreateColorBrush(ToCompositionColor(
                settings.PreviewBorderColor,
                "#B8007AFF"));

            _backdropGeometry = compositor.CreateRoundedRectangleGeometry();
            _tintGeometry = compositor.CreateRoundedRectangleGeometry();
            _borderGeometry = compositor.CreateRoundedRectangleGeometry();

            _backdropShape = compositor.CreateSpriteShape(_backdropGeometry);
            _backdropShape.FillBrush = _backdropEffectBrush;

            _tintShape = compositor.CreateSpriteShape(_tintGeometry);
            _tintShape.FillBrush = _tintBrush;

            _borderShape = compositor.CreateSpriteShape(_borderGeometry);
            _borderShape.StrokeBrush = _borderBrush;
            _borderShape.IsStrokeNonScaling = true;

            _surfaceVisual.Shapes.Add(_backdropShape);
            _surfaceVisual.Shapes.Add(_tintShape);
            _surfaceVisual.Shapes.Add(_borderShape);
        }
        catch
        {
            _initializationFailed = true;
        }
    }

    internal bool TryShowFrame(NativeMethods.Rect frame)
    {
        if (_initializationFailed || !OperatingSystem.IsWindowsVersionAtLeast(10, 0, 22000))
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
        var left = frame.Left * scaleX + 8;
        var top = frame.Top * scaleY + 8;
        var width = Math.Max(frame.Width * scaleX - 16, 1);
        var height = Math.Max(frame.Height * scaleY - 16, 1);

        Left = left;
        Top = top;
        Width = width;
        Height = height;

        if (!IsVisible)
        {
            try
            {
                Show();
            }
            catch
            {
                _initializationFailed = true;
                return false;
            }
        }

        if (!_initialized && !TryInitializeComposition())
        {
            Hide();
            return false;
        }

        UpdateSurface(width * dpiX / 96.0, height * dpiY / 96.0, dpiX / 96.0, dpiY / 96.0);
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
    }

    private void OnLoaded(object sender, RoutedEventArgs e) => TryInitializeComposition();

    private bool TryInitializeComposition()
    {
        if (_initialized)
        {
            return true;
        }

        if (_initializationFailed || !_compositionHost.IsReady)
        {
            return false;
        }

        try
        {
            var root = _compositionHost.CreateRoot();
            root.Children.InsertAtTop(_surfaceVisual);
            _compositionHost.SetRoot(root);
            _initialized = true;
            return true;
        }
        catch
        {
            _initializationFailed = true;
            return false;
        }
    }

    private void UpdateSurface(double width, double height, double scaleX, double scaleY)
    {
        var size = new Vector2((float)Math.Max(width, 1), (float)Math.Max(height, 1));
        var radius = (float)Math.Min(
            _settings.PreviewCornerRadius * Math.Min(scaleX, scaleY),
            Math.Min(width, height) / 2);

        _surfaceVisual.Size = size;
        _backdropGeometry.Size = size;
        _backdropGeometry.CornerRadius = new Vector2(radius, radius);
        _tintGeometry.Size = size;
        _tintGeometry.CornerRadius = new Vector2(radius, radius);
        _borderGeometry.Size = size;
        _borderGeometry.CornerRadius = new Vector2(radius, radius);
        _borderShape.StrokeThickness = (float)Math.Max(
            _settings.PreviewBorderWidth * Math.Min(scaleX, scaleY),
            0);
    }

    private static Windows.UI.Color ToCompositionColor(string? value, string fallback)
    {
        var color = ParseColor(value, fallback);
        return Windows.UI.Color.FromArgb(color.A, color.R, color.G, color.B);
    }

    private static MediaColor ParseColor(string? value, string fallback)
    {
        try
        {
            if (ColorConverter.ConvertFromString(value ?? string.Empty) is MediaColor color)
            {
                return color;
            }
        }
        catch
        {
            // Invalid settings use the fallback color.
        }

        return (MediaColor)ColorConverter.ConvertFromString(fallback)!;
    }
}
