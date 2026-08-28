using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using System.Windows.Interop;
using Windows.UI.Composition;

#pragma warning disable CA1416

namespace LoopW;

/// <summary>
/// Hosts a Windows.UI.Composition visual tree inside a WPF window.
/// The child HWND is also the DWM host-backdrop surface. Keeping that
/// boundary explicit prevents WPF from replacing the live material with an
/// opaque client surface.
/// </summary>
[SupportedOSPlatform("windows10.0.17134")]
internal sealed class CompositionHost : HwndHost
{
    private readonly object _dispatcherQueue;
    private readonly List<IDisposable> _disposables = new();
    private readonly ICompositorDesktopInterop _compositorDesktopInterop;
    private ICompositionTarget? _compositionTarget;
    private IntPtr _hwnd;
    private ContainerVisual? _root;

    public CompositionHost()
    {
        PreloadNativeDependencies();
        _dispatcherQueue = InitializeDispatcherQueue();
        Compositor = new Compositor();
        _compositorDesktopInterop = (ICompositorDesktopInterop)(object)Compositor;
    }

    public Compositor Compositor { get; }

    internal bool IsReady => _compositionTarget is not null;

    internal void SetRoot(Visual visual)
    {
        if (_compositionTarget is null)
        {
            throw new InvalidOperationException("The composition host window has not been created.");
        }

        _compositionTarget.Root = visual;
    }

    internal ContainerVisual CreateRoot()
    {
        if (_root is null)
        {
            _root = Compositor.CreateContainerVisual();
            SetRoot(_root);
        }

        return _root;
    }

    protected override HandleRef BuildWindowCore(HandleRef hwndParent)
    {
        _hwnd = CreateWindowEx(
            WsExTransparent | WsExNoActivate,
            "Static",
            "LoopW Composition Host",
            WsChild | WsVisible,
            0,
            0,
            0,
            0,
            hwndParent.Handle,
            IntPtr.Zero,
            IntPtr.Zero,
            IntPtr.Zero);

        if (_hwnd == IntPtr.Zero)
        {
            throw new InvalidOperationException("Unable to create the composition host window.");
        }

        _compositorDesktopInterop.CreateDesktopWindowTarget(_hwnd, isTopmost: true, out var target);
        _compositionTarget = target;
        return new HandleRef(this, _hwnd);
    }

    protected override void DestroyWindowCore(HandleRef hwnd)
    {
        _compositionTarget?.Root?.Dispose();
        _compositionTarget = null;
        _root = null;

        if (hwnd.Handle != IntPtr.Zero)
        {
            DestroyWindow(hwnd.Handle);
        }

        foreach (var disposable in _disposables)
        {
            disposable.Dispose();
        }

        _disposables.Clear();
        _hwnd = IntPtr.Zero;
    }

    internal void RegisterForDispose(IDisposable disposable) => _disposables.Add(disposable);

    private static void PreloadNativeDependencies()
    {
        var baseDirectory = AppContext.BaseDirectory;
        var nativeFiles = new[]
        {
            "concrt140_app.dll",
            "msvcp140_1_app.dll",
            "msvcp140_2_app.dll",
            "msvcp140_app.dll",
            "msvcp140_atomic_wait_app.dll",
            "vcamp140_app.dll",
            "vccorlib140_app.dll",
            "vcomp140_app.dll",
            "vcruntime140_1_app.dll",
            "vcruntime140_app.dll",
            "Microsoft.Graphics.Canvas.dll"
        };

        foreach (var nativeFile in nativeFiles)
        {
            var path = Path.Combine(baseDirectory, nativeFile);
            if (!File.Exists(path))
            {
                continue;
            }

            try
            {
                NativeLibrary.Load(path);
            }
            catch (Exception exception)
            {
                LivePreviewDiagnostics.Record("native-load", nativeFile, exception);
            }
        }
    }

    private static object InitializeDispatcherQueue()
    {
        var options = new DispatcherQueueOptions
        {
            Size = Marshal.SizeOf<DispatcherQueueOptions>(),
            ThreadType = DispatcherQueueThreadType.Current,
            ApartmentType = DispatcherQueueApartmentType.Sta
        };

        var hresult = CreateDispatcherQueueController(options, out object dispatcherQueue);
        if (hresult != 0)
        {
            Marshal.ThrowExceptionForHR(hresult);
        }

        return dispatcherQueue;
    }

    private const int WsChild = 0x40000000;
    private const int WsVisible = 0x10000000;
    private const int WsExTransparent = 0x00000020;
    private const int WsExNoActivate = unchecked((int)0x08000000);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowEx(
        int exStyle,
        string className,
        string windowName,
        int style,
        int x,
        int y,
        int width,
        int height,
        IntPtr parent,
        IntPtr menu,
        IntPtr instance,
        IntPtr param);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr hwnd);

    [DllImport("coremessaging.dll")]
    private static extern int CreateDispatcherQueueController(
        DispatcherQueueOptions options,
        [MarshalAs(UnmanagedType.IUnknown)] out object dispatcherQueueController);

    [StructLayout(LayoutKind.Sequential)]
    private struct DispatcherQueueOptions
    {
        public int Size;

        [MarshalAs(UnmanagedType.I4)]
        public DispatcherQueueThreadType ThreadType;

        [MarshalAs(UnmanagedType.I4)]
        public DispatcherQueueApartmentType ApartmentType;
    }

    private enum DispatcherQueueThreadType
    {
        Dedicated = 1,
        Current = 2
    }

    private enum DispatcherQueueApartmentType
    {
        None = 0,
        Asta = 1,
        Sta = 2
    }

    [ComImport]
    [Guid("29E691FA-4567-4DCA-B319-D0F207EB6807")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface ICompositorDesktopInterop
    {
        void CreateDesktopWindowTarget(
            IntPtr hwndTarget,
            [MarshalAs(UnmanagedType.Bool)] bool isTopmost,
            out ICompositionTarget target);
    }

    [ComImport]
    [Guid("A1BEA8BA-D726-4663-8129-6B5E7927FFA6")]
    [InterfaceType(ComInterfaceType.InterfaceIsIInspectable)]
    private interface ICompositionTarget
    {
        Visual Root { get; set; }
    }
}
