using System;
using System.IO;
using System.Text.Json;

namespace LoopW;

public sealed class AppSettings
{
    private static readonly string SettingsPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "LoopW",
        "settings.json");

    public uint TriggerVk { get; set; } = NativeMethods.VkCapital;

    public uint TriggerModifiers { get; set; }

    public static AppSettings Load()
    {
        try
        {
            if (File.Exists(SettingsPath))
            {
                var loaded = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(SettingsPath));
                if (loaded != null)
                {
                    return loaded;
                }
            }
        }
        catch
        {
            // corrupted or unreadable settings fall back to the default trigger
        }

        return new AppSettings();
    }

    public void Save()
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(SettingsPath)!);
            File.WriteAllText(SettingsPath, JsonSerializer.Serialize(this, new JsonSerializerOptions { WriteIndented = true }));
        }
        catch
        {
            // persistence is best-effort; the binding still works for this session
        }
    }
}
