using System.Drawing;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Media.Imaging;
using TrayNotifyIcon = Wpf.Ui.Tray.Controls.NotifyIcon;

namespace LoopW;

internal sealed class TrayIcon : IDisposable
{
    private readonly Action _showMainWindow;
    private readonly TrayNotifyIcon _notifyIcon;
    private readonly ContextMenu _menu;

    public TrayIcon(Action showMainWindow, Action openSettings, Action quit)
    {
        _showMainWindow = showMainWindow;
        _menu = new ContextMenu();
        _menu.Items.Add(CreateMenuItem("Open LoopW", showMainWindow));
        _menu.Items.Add(CreateMenuItem("Open settings", openSettings));
        _menu.Items.Add(new Separator());
        _menu.Items.Add(CreateMenuItem("Quit", quit));

        _notifyIcon = new TrayNotifyIcon
        {
            Icon = CreateTrayImage(),
            TooltipText = "LoopW",
            Menu = _menu,
            MenuOnRightClick = true,
            FocusOnLeftClick = true
        };
#pragma warning disable CS8622 // WPF UI's tray delegate annotations differ between target frameworks.
        _notifyIcon.LeftDoubleClick += NotifyIcon_LeftDoubleClick;
#pragma warning restore CS8622
        _notifyIcon.Register();
    }

    public void Dispose()
    {
        _notifyIcon.Dispose();
        _menu.Items.Clear();
    }

    private static MenuItem CreateMenuItem(string header, Action action)
    {
        var item = new MenuItem { Header = header };
        item.Click += (_, _) => action();
        return item;
    }

    private void NotifyIcon_LeftDoubleClick(TrayNotifyIcon? sender, RoutedEventArgs e) => _showMainWindow();

    private static BitmapSource CreateTrayImage()
    {
        var source = Imaging.CreateBitmapSourceFromHIcon(
            SystemIcons.Application.Handle,
            Int32Rect.Empty,
            BitmapSizeOptions.FromEmptyOptions());
        source.Freeze();
        return source;
    }
}
