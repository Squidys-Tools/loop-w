using System;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media.Imaging;

namespace LoopW;

internal static class ScreenCapture
{
    public static BitmapSource? CaptureRegion(int left, int top, int width, int height)
    {
        if (width <= 0 || height <= 0)
        {
            return null;
        }

        var hdcSrc = NativeMethods.GetDC(IntPtr.Zero);
        var hdcMem = NativeMethods.CreateCompatibleDC(hdcSrc);
        var hbmp = NativeMethods.CreateCompatibleBitmap(hdcSrc, width, height);
        if (hbmp == IntPtr.Zero)
        {
            NativeMethods.DeleteDC(hdcMem);
            NativeMethods.ReleaseDC(IntPtr.Zero, hdcSrc);
            return null;
        }

        var previous = NativeMethods.SelectObject(hdcMem, hbmp);
        NativeMethods.BitBlt(hdcMem, 0, 0, width, height, hdcSrc, left, top, NativeMethods.SrcCopy);
        NativeMethods.SelectObject(hdcMem, previous);
        var source = Imaging.CreateBitmapSourceFromHBitmap(hbmp, IntPtr.Zero, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
        NativeMethods.DeleteObject(hbmp);
        NativeMethods.DeleteDC(hdcMem);
        NativeMethods.ReleaseDC(IntPtr.Zero, hdcSrc);
        return source;
    }
}
