using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Windows.Media;

namespace LoopW;

public sealed class AppSettings
{
    private static readonly string SettingsPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "LoopW",
        "settings.json");

    public uint TriggerVk { get; set; } = NativeMethods.VkCapital;

    public uint TriggerModifiers { get; set; }

    public List<Keybind> Keybinds { get; set; } = new();

    public bool LaunchAtLogin { get; set; }

    public bool RadialEnabled { get; set; } = true;

    public bool CursorInteractionEnabled { get; set; } = true;

    public double RadialOuterRadius { get; set; } = 91.2;

    public double RadialInnerRadius { get; set; } = 57.76;

    public bool PreviewEnabled { get; set; } = true;

    public double PreviewPadding { get; set; } = 21;

    public double PreviewCornerRadius { get; set; } = 14;

    public double PreviewBorderWidth { get; set; }

    public string AccentColor { get; set; } = "#007AFF";

    public string RadialSectorFill { get; set; } = "#7A007AFF";

    public string RadialSectorStroke { get; set; } = "#F0007AFF";

    public string RadialRingFill { get; set; } = "#B61E1E1E";

    public string PreviewBorderColor { get; set; } = "#FFFFFFFF";

    public static AppSettings Load()
    {
        try
        {
            if (File.Exists(SettingsPath))
            {
                var loaded = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(SettingsPath));
                if (loaded != null)
                {
                    loaded.Normalize();
                    return loaded;
                }
            }
        }
        catch
        {
            // corrupted or unreadable settings fall back to the default trigger
        }

        var defaults = new AppSettings();
        defaults.Normalize();
        return defaults;
    }

    public void Save()
    {
        try
        {
            Normalize();
            Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!);
            File.WriteAllText(SettingsPath, JsonSerializer.Serialize(this, new JsonSerializerOptions { WriteIndented = true }));
        }
        catch
        {
            // persistence is best-effort; the binding still works for this session
        }
    }

    private void Normalize()
    {
        TriggerVk = TriggerVk == 0 ? NativeMethods.VkCapital : TriggerVk;
        TriggerModifiers &= NativeMethods.ModControl | NativeMethods.ModAlt | NativeMethods.ModShift | NativeMethods.ModWin;
        Keybinds ??= new List<Keybind>();

        RadialOuterRadius = Clamp(RadialOuterRadius, 64, 140);
        RadialInnerRadius = Clamp(RadialInnerRadius, 24, RadialOuterRadius - 8);
        PreviewPadding = Clamp(PreviewPadding, 4, 48);
        PreviewCornerRadius = Clamp(PreviewCornerRadius, 4, 32);
        PreviewBorderWidth = Clamp(PreviewBorderWidth, 0, 6);

        AccentColor = NormalizeColor(AccentColor, "#007AFF");
        RadialSectorFill = NormalizeColor(RadialSectorFill, "#7A007AFF");
        RadialSectorStroke = NormalizeColor(RadialSectorStroke, "#F0007AFF");
        RadialRingFill = NormalizeColor(RadialRingFill, "#B61E1E1E");
        PreviewBorderColor = NormalizeColor(PreviewBorderColor, "#FFFFFFFF");
    }

    private static double Clamp(double value, double min, double max) =>
        double.IsFinite(value) ? Math.Max(min, Math.Min(max, value)) : min;

    private static string NormalizeColor(string? value, string fallback)
    {
        try
        {
            if (ColorConverter.ConvertFromString(value) is Color color)
            {
                return color.ToString();
            }
        }
        catch
        {
            // Invalid values from settings fall back at the config boundary.
        }

        return fallback;
    }
}
