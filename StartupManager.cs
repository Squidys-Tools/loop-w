using Microsoft.Win32;

namespace LoopW;

internal static class StartupManager
{
    private const string RunKeyPath = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    private const string ValueName = "LoopW";

    public static bool TrySetEnabled(bool enabled)
    {
        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(RunKeyPath);
            if (key == null)
            {
                return false;
            }

            if (enabled)
            {
                var processPath = Environment.ProcessPath;
                if (string.IsNullOrWhiteSpace(processPath))
                {
                    return false;
                }

                key.SetValue(ValueName, $"\"{processPath}\"");
            }
            else
            {
                key.DeleteValue(ValueName, throwOnMissingValue: false);
            }

            return true;
        }
        catch
        {
            return false;
        }
    }
}
