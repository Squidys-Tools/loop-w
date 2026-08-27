using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Win32;
using System.Windows.Media;
using MediaColor = System.Windows.Media.Color;
using MediaColorConverter = System.Windows.Media.ColorConverter;

namespace LoopW;

public sealed class AppSettings
{
    private static readonly JsonSerializerOptions SaveOptions = new() { WriteIndented = true };
    private static readonly string SettingsPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "LoopW",
        "settings.json");

    public uint TriggerVk { get; set; } = NativeMethods.VkCapital;

    public uint TriggerModifiers { get; set; }

    public TriggerModifierSide TriggerModifierSide { get; set; } = TriggerModifierSide.Any;

    public int TriggerDelayMilliseconds { get; set; }

    public int TriggerTimeoutMilliseconds { get; set; }

    public bool DoubleClickToTrigger { get; set; }

    public bool MiddleClickToTrigger { get; set; }

    public List<Keybind> Keybinds { get; set; } = new();

    public List<RadialTargetSettings> RadialSlots { get; set; } = RadialConfiguration.CreateDefaultSlots();

    public RadialTargetSettings CenterTarget { get; set; } = RadialConfiguration.CreateDefaultCenter();

    public bool LaunchAtLogin { get; set; }

    public string AppearanceMode { get; set; } = "Dark";

    [JsonIgnore]
    public bool IsLightAppearance => AppearanceMode == "Light" ||
        (AppearanceMode == "FollowWindows" && IsWindowsLightMode());

    public bool RadialEnabled { get; set; } = true;

    public bool CursorInteractionEnabled { get; set; } = true;

    public double RadialOuterRadius { get; set; } = 91.2;

    public double RadialInnerRadius { get; set; } = 57.76;

    public bool PreviewEnabled { get; set; } = true;

    // Temporary proof-of-concept switch. The live compositor path falls back
    // to the existing bitmap renderer if initialization is unavailable.
    public bool LiveBackdropPreviewEnabled { get; set; } = true;

    public bool DragSnapEnabled { get; set; } = true;

    public int DragSnapThreshold { get; set; } = 24;

    public bool RestorePreDragFrameOnSnapCancel { get; set; } = true;

    public bool StashPersistenceEnabled { get; set; } = true;

    public MonitorMoveSizePolicy MonitorMoveSizePolicy { get; set; } = MonitorMoveSizePolicy.PreservePixels;

    public int GlobalScreenPadding { get; set; }

    public int ScreenPaddingLeft { get; set; }

    public int ScreenPaddingTop { get; set; }

    public int ScreenPaddingRight { get; set; }

    public int ScreenPaddingBottom { get; set; }

    public List<string> ExcludedExecutablePaths { get; set; } = new();

    public List<string> ExcludedProcessNames { get; set; } = new();

    public int StashEdgePeek { get; set; } = 8;

    public int StashHitZone { get; set; } = 14;

    public int StashRevealDelayMilliseconds { get; set; } = 80;

    public List<StashRecord> StashRecords { get; set; } = new();

    public double PreviewPadding { get; set; } = 21;

    public double PreviewCornerRadius { get; set; } = 14;

    public double PreviewBorderWidth { get; set; } = 2;

    public string AccentColor { get; set; } = "#3D9BFF";

    public string RadialSectorFill { get; set; } = "#7A3D9BFF";

    public string RadialSectorStroke { get; set; } = "#F03D9BFF";

    public string RadialRingFill { get; set; } = "#B61B212B";

    public string PreviewBorderColor { get; set; } = "#B83D9BFF";

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

    public bool Save()
    {
        using var performance = PerformanceDiagnostics.Measure(PerformanceMetric.SettingsSave);
        string? temporaryPath = null;
        try
        {
            Normalize();
            var directory = Path.GetDirectoryName(SettingsPath)!;
            Directory.CreateDirectory(directory);

            var serialized = JsonSerializer.Serialize(this, SaveOptions);
            temporaryPath = Path.Combine(
                directory,
                $"{Path.GetFileName(SettingsPath)}.{Guid.NewGuid():N}.tmp");

            using (var stream = new FileStream(
                       temporaryPath,
                       FileMode.CreateNew,
                       FileAccess.Write,
                       FileShare.None,
                       bufferSize: 4096,
                       options: FileOptions.WriteThrough))
            using (var writer = new StreamWriter(stream, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false)))
            {
                writer.Write(serialized);
            }

            File.Move(temporaryPath, SettingsPath, overwrite: true);
            return true;
        }
        catch
        {
            // The previous complete settings file remains in place when the
            // temporary write or replacement fails.
            return false;
        }
        finally
        {
            if (temporaryPath is not null)
            {
                try
                {
                    File.Delete(temporaryPath);
                }
                catch
                {
                    // A failed cleanup does not change the save result.
                }
            }
        }
    }

    internal void Normalize()
    {
        TriggerVk = TriggerVk == 0 ? NativeMethods.VkCapital : TriggerVk;
        TriggerModifiers &= NativeMethods.ModControl | NativeMethods.ModAlt | NativeMethods.ModShift | NativeMethods.ModWin;
        if (!Enum.IsDefined(TriggerModifierSide))
        {
            TriggerModifierSide = TriggerModifierSide.Any;
        }

        TriggerDelayMilliseconds = Math.Clamp(TriggerDelayMilliseconds, 0, 1000);
        TriggerTimeoutMilliseconds = Math.Clamp(TriggerTimeoutMilliseconds, 0, 10000);
        Keybinds ??= new List<Keybind>();
        NormalizeKeybinds();
        NormalizeRadialTargets();
        AppearanceMode = AppearanceMode is "Dark" or "FollowWindows" or "Light"
            ? AppearanceMode
            : "Dark";

        RadialOuterRadius = Clamp(RadialOuterRadius, 64, 140);
        RadialInnerRadius = Clamp(RadialInnerRadius, 24, RadialOuterRadius - 8);
        PreviewPadding = Clamp(PreviewPadding, 4, 48);
        PreviewCornerRadius = Clamp(PreviewCornerRadius, 4, 32);
        PreviewBorderWidth = Clamp(PreviewBorderWidth, 0, 6);
        DragSnapThreshold = Math.Clamp(DragSnapThreshold, 4, 96);
        if (!Enum.IsDefined(MonitorMoveSizePolicy))
        {
            MonitorMoveSizePolicy = MonitorMoveSizePolicy.PreservePixels;
        }

        GlobalScreenPadding = Math.Clamp(GlobalScreenPadding, 0, 128);
        ScreenPaddingLeft = Math.Clamp(ScreenPaddingLeft, 0, 128);
        ScreenPaddingTop = Math.Clamp(ScreenPaddingTop, 0, 128);
        ScreenPaddingRight = Math.Clamp(ScreenPaddingRight, 0, 128);
        ScreenPaddingBottom = Math.Clamp(ScreenPaddingBottom, 0, 128);
        NormalizeExclusions();
        StashEdgePeek = Math.Clamp(StashEdgePeek, 1, 48);
        StashHitZone = Math.Clamp(StashHitZone, 1, 96);
        StashRevealDelayMilliseconds = Math.Clamp(StashRevealDelayMilliseconds, 0, 2000);
        NormalizeStashRecords();

        AccentColor = NormalizeColor(AccentColor, "#007AFF");
        RadialSectorFill = NormalizeColor(RadialSectorFill, "#7A007AFF");
        RadialSectorStroke = NormalizeColor(RadialSectorStroke, "#F0007AFF");
        RadialRingFill = NormalizeColor(RadialRingFill, "#B61E1E1E");
        PreviewBorderColor = NormalizeColor(PreviewBorderColor, "#B8007AFF");
    }

    public void ResetToDefaults()
    {
        var defaults = new AppSettings();
        TriggerVk = defaults.TriggerVk;
        TriggerModifiers = defaults.TriggerModifiers;
        TriggerModifierSide = defaults.TriggerModifierSide;
        TriggerDelayMilliseconds = defaults.TriggerDelayMilliseconds;
        TriggerTimeoutMilliseconds = defaults.TriggerTimeoutMilliseconds;
        DoubleClickToTrigger = defaults.DoubleClickToTrigger;
        MiddleClickToTrigger = defaults.MiddleClickToTrigger;
        Keybinds = new List<Keybind>();
        RadialSlots = RadialConfiguration.CreateDefaultSlots();
        CenterTarget = RadialConfiguration.CreateDefaultCenter();
        LaunchAtLogin = defaults.LaunchAtLogin;
        AppearanceMode = defaults.AppearanceMode;
        RadialEnabled = defaults.RadialEnabled;
        CursorInteractionEnabled = defaults.CursorInteractionEnabled;
        RadialOuterRadius = defaults.RadialOuterRadius;
        RadialInnerRadius = defaults.RadialInnerRadius;
        PreviewEnabled = defaults.PreviewEnabled;
        LiveBackdropPreviewEnabled = defaults.LiveBackdropPreviewEnabled;
        DragSnapEnabled = defaults.DragSnapEnabled;
        DragSnapThreshold = defaults.DragSnapThreshold;
        RestorePreDragFrameOnSnapCancel = defaults.RestorePreDragFrameOnSnapCancel;
        StashPersistenceEnabled = defaults.StashPersistenceEnabled;
        MonitorMoveSizePolicy = defaults.MonitorMoveSizePolicy;
        GlobalScreenPadding = defaults.GlobalScreenPadding;
        ScreenPaddingLeft = defaults.ScreenPaddingLeft;
        ScreenPaddingTop = defaults.ScreenPaddingTop;
        ScreenPaddingRight = defaults.ScreenPaddingRight;
        ScreenPaddingBottom = defaults.ScreenPaddingBottom;
        ExcludedExecutablePaths = new List<string>();
        ExcludedProcessNames = new List<string>();
        StashEdgePeek = defaults.StashEdgePeek;
        StashHitZone = defaults.StashHitZone;
        StashRevealDelayMilliseconds = defaults.StashRevealDelayMilliseconds;
        StashRecords = new List<StashRecord>();
        PreviewPadding = defaults.PreviewPadding;
        PreviewCornerRadius = defaults.PreviewCornerRadius;
        PreviewBorderWidth = defaults.PreviewBorderWidth;
        AccentColor = defaults.AccentColor;
        RadialSectorFill = defaults.RadialSectorFill;
        RadialSectorStroke = defaults.RadialSectorStroke;
        RadialRingFill = defaults.RadialRingFill;
        PreviewBorderColor = defaults.PreviewBorderColor;
    }

    private static bool IsWindowsLightMode()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(
                @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
            return key?.GetValue("AppsUseLightTheme") is int value && value != 0;
        }
        catch
        {
            return false;
        }
    }

    private static double Clamp(double value, double min, double max) =>
        double.IsFinite(value) ? Math.Max(min, Math.Min(max, value)) : min;

    private void NormalizeKeybinds()
    {
        var ids = new HashSet<string>(StringComparer.Ordinal);
        for (var i = 0; i < Keybinds.Count; i++)
        {
            var keybind = Keybinds[i];
            if (keybind is null)
            {
                keybind = new Keybind(0, NativeMethods.VkSpace, WindowAction.RightHalf);
                Keybinds[i] = keybind;
            }

            if (string.IsNullOrWhiteSpace(keybind.Id) || !ids.Add(keybind.Id))
            {
                do
                {
                    keybind.Id = Guid.NewGuid().ToString("N");
                }
                while (!ids.Add(keybind.Id));
            }

            keybind.Modifiers &= NativeMethods.ModControl | NativeMethods.ModAlt | NativeMethods.ModShift | NativeMethods.ModWin;
            if (!Enum.IsDefined(keybind.Action))
            {
                keybind.Action = WindowAction.RightHalf;
            }
        }
    }

    private void NormalizeStashRecords()
    {
        StashRecords ??= new List<StashRecord>();
        const int maxStashRecords = 64;
        if (StashRecords.Count > maxStashRecords)
        {
            // New records are appended. Keep the newest records so a buildup
            // of old unmatched entries cannot evict a newly stashed window.
            StashRecords.RemoveRange(0, StashRecords.Count - maxStashRecords);
        }

        var ids = new HashSet<string>(StringComparer.Ordinal);
        for (var i = StashRecords.Count - 1; i >= 0; i--)
        {
            var record = StashRecords[i];
            if (record is null)
            {
                StashRecords.RemoveAt(i);
                continue;
            }

            if (string.IsNullOrWhiteSpace(record.Id) || !ids.Add(record.Id))
            {
                do
                {
                    record.Id = Guid.NewGuid().ToString("N");
                }
                while (!ids.Add(record.Id));
            }

            if (!Enum.IsDefined(record.Edge))
            {
                record.Edge = StashEdge.Left;
            }

            record.OriginalPlacement ??= new StashPlacement();
            record.OriginalPlacement.MinPosition ??= new StashPoint();
            record.OriginalPlacement.MaxPosition ??= new StashPoint();
            record.OriginalPlacement.NormalPosition ??= new StashRect();
            record.OriginalMonitor ??= new StashMonitor();
            record.OriginalMonitor.Monitor ??= new StashRect();
            record.OriginalMonitor.Work ??= new StashRect();
            record.StashedFrame ??= new StashRect();
        }
    }

    private void NormalizeExclusions()
    {
        ExcludedExecutablePaths = NormalizeExclusionList(ExcludedExecutablePaths, path => path.Trim());
        ExcludedProcessNames = NormalizeExclusionList(
            ExcludedProcessNames,
            name => Path.GetFileNameWithoutExtension(name.Trim()));
    }

    private static List<string> NormalizeExclusionList(
        List<string>? values,
        Func<string, string> normalize)
    {
        var normalized = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var value in values ?? new List<string>())
        {
            if (string.IsNullOrWhiteSpace(value))
            {
                continue;
            }

            var item = normalize(value);
            if (!string.IsNullOrWhiteSpace(item) && seen.Add(item))
            {
                normalized.Add(item);
            }
        }

        return normalized;
    }

    private void NormalizeRadialTargets()
    {
        RadialSlots ??= RadialConfiguration.CreateDefaultSlots();
        while (RadialSlots.Count < RadialConfiguration.SlotCount)
        {
            RadialSlots.Add(new RadialTargetSettings { Kind = RadialTargetKind.None });
        }

        if (RadialSlots.Count > RadialConfiguration.SlotCount)
        {
            RadialSlots.RemoveRange(RadialConfiguration.SlotCount, RadialSlots.Count - RadialConfiguration.SlotCount);
        }

        var keybindIds = new HashSet<string>(Keybinds.Select(keybind => keybind.Id), StringComparer.Ordinal);
        for (var i = 0; i < RadialSlots.Count; i++)
        {
            RadialSlots[i] ??= new RadialTargetSettings { Kind = RadialTargetKind.None };
            NormalizeTarget(RadialSlots[i], keybindIds);
        }

        CenterTarget ??= RadialConfiguration.CreateDefaultCenter();
        NormalizeTarget(CenterTarget, keybindIds);
    }

    private static void NormalizeTarget(RadialTargetSettings target, IReadOnlySet<string> keybindIds)
    {
        if (!Enum.IsDefined(target.Kind))
        {
            target.Kind = RadialTargetKind.None;
        }

        switch (target.Kind)
        {
            case RadialTargetKind.None:
                target.Action = WindowAction.RightHalf;
                target.KeybindId = string.Empty;
                target.CycleEnabled = false;
                break;
            case RadialTargetKind.Action:
                if (!Enum.IsDefined(target.Action))
                {
                    target.Kind = RadialTargetKind.None;
                    target.Action = WindowAction.RightHalf;
                    target.KeybindId = string.Empty;
                    target.CycleEnabled = false;
                }

                break;
            case RadialTargetKind.Keybind:
                if (string.IsNullOrWhiteSpace(target.KeybindId) || !keybindIds.Contains(target.KeybindId))
                {
                    target.Kind = RadialTargetKind.None;
                    target.Action = WindowAction.RightHalf;
                    target.KeybindId = string.Empty;
                    target.CycleEnabled = false;
                }

                break;
        }
    }

    private static string NormalizeColor(string? value, string fallback)
    {
        try
        {
            if (MediaColorConverter.ConvertFromString(value) is MediaColor color)
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
