using System.Drawing;
using Forms = System.Windows.Forms;

namespace LoopW;

internal sealed class TrayIcon : IDisposable
{
    private readonly Forms.NotifyIcon _notifyIcon;

    public TrayIcon(Action showMainWindow, Action openSettings, Action quit)
    {
        var menu = new Forms.ContextMenuStrip();
        menu.Items.Add("Open LoopW", null, (_, _) => showMainWindow());
        menu.Items.Add("Open settings", null, (_, _) => openSettings());
        menu.Items.Add(new Forms.ToolStripSeparator());
        menu.Items.Add("Quit", null, (_, _) => quit());

        _notifyIcon = new Forms.NotifyIcon
        {
            Icon = SystemIcons.Application,
            Text = "LoopW",
            ContextMenuStrip = menu,
            Visible = true
        };
        _notifyIcon.DoubleClick += (_, _) => showMainWindow();
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
    }
}
