using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Effects;

namespace LoopW;

public partial class WindowPreviewSurface : UserControl
{
    private Effect? _surfaceEffect;
    private Effect? _backdropEffect;

    public WindowPreviewSurface()
    {
        InitializeComponent();

        var transforms = (TransformGroup)RenderTransform;
        SurfaceScaleTransform = (ScaleTransform)transforms.Children[0];
        SurfaceTranslateTransform = (TranslateTransform)transforms.Children[1];
    }

    public Image BackdropImageElement => BackdropImage;

    internal ScaleTransform SurfaceScaleTransform { get; }

    internal TranslateTransform SurfaceTranslateTransform { get; }

    public double BlurMargin { get; private set; }

    public double BackdropClipRadius => SurfaceTint.CornerRadius.TopLeft;

    public void ApplySettings(AppSettings settings)
    {
        BlurMargin = settings.PreviewPadding;
        SurfaceElement.CornerRadius = new CornerRadius(settings.PreviewCornerRadius);
        SurfaceTint.CornerRadius = new CornerRadius(Math.Max(1, settings.PreviewCornerRadius - 3));
        SurfaceElement.BorderBrush = CreateBrush(settings.PreviewBorderColor, "#B8007AFF");
        SurfaceElement.BorderThickness = new Thickness(Math.Max(0, settings.PreviewBorderWidth));
        _surfaceEffect = settings.IsLightAppearance
            ? new DropShadowEffect
            {
                BlurRadius = 18,
                Direction = 270,
                ShadowDepth = 4,
                Opacity = 0.2,
                Color = Color.FromRgb(0x40, 0x51, 0x5A)
            }
            : new DropShadowEffect
            {
                BlurRadius = 18,
                Direction = 270,
                ShadowDepth = 4,
                Opacity = 0.32,
                Color = Colors.Black
            };
        _surfaceEffect.Freeze();
        SurfaceElement.Effect = _surfaceEffect;

        _backdropEffect = new BlurEffect { Radius = 20 };
        _backdropEffect.Freeze();
        BackdropImage.Effect = _backdropEffect;
        SurfaceTint.Background = CreateBrush(
            settings.IsLightAppearance ? "#30FFFFFF" : "#30101827",
            settings.IsLightAppearance ? "#30FFFFFF" : "#30101827");
    }

    internal void SetTransitionRendering(bool transitioning)
    {
        SurfaceElement.Effect = transitioning ? null : _surfaceEffect;
        BackdropImage.Effect = transitioning ? null : _backdropEffect;
    }

    private static Brush CreateBrush(string? value, string fallback)
    {
        try
        {
            if (new BrushConverter().ConvertFromString(value ?? string.Empty) is Brush brush)
            {
                brush.Freeze();
                return brush;
            }
        }
        catch
        {
            // Invalid in-memory values use a safe fallback.
        }

        return new BrushConverter().ConvertFromString(fallback) as Brush ?? Brushes.Transparent;
    }
}
