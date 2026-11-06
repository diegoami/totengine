# Target only the window belonging to the process this script starts.
param(
    [string]$Binary = "$PSScriptRoot\target\x86_64-pc-windows-gnullvm\debug\tot-desktop-spike.exe",
    [string]$Report = "$PSScriptRoot\target\windows-input.json"
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class SpikeWindow {
    [DllImport("kernel32.dll", SetLastError=true)] public static extern bool GetExitCodeProcess(IntPtr h, out uint code);
    [StructLayout(LayoutKind.Sequential)] private struct KEYBDINPUT { public ushort vk, scan; public uint flags, time; public UIntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] private struct MOUSEINPUT { public int x, y; public uint data, flags, time; public UIntPtr extra; }
    [StructLayout(LayoutKind.Explicit)] private struct INPUTUNION { [FieldOffset(0)] public KEYBDINPUT key; [FieldOffset(0)] public MOUSEINPUT mouse; }
    [StructLayout(LayoutKind.Sequential)] private struct INPUT { public uint type; public INPUTUNION value; }
    [DllImport("user32.dll", SetLastError=true)] private static extern uint SendInput(uint n, INPUT[] input, int size);
    [DllImport("user32.dll")] private static extern uint MapVirtualKey(uint key, uint mode);
    public static void Key(uint vk, bool up) {
        var i = new INPUT(); i.type = 1;
        i.value.key.scan = (ushort)MapVirtualKey(vk, 0);
        i.value.key.flags = 8u | (up ? 2u : 0u);
        if (SendInput(1, new[] { i }, Marshal.SizeOf(typeof(INPUT))) != 1) throw new Exception("SendInput failed: " + Marshal.GetLastWin32Error());
    }
    private delegate bool EnumProc(IntPtr h, IntPtr p);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumProc cb, IntPtr p);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    public static IntPtr ForProcess(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr p) {
            uint owner; GetWindowThreadProcessId(h, out owner);
            if (owner == pid) {
                var title = new StringBuilder(256); GetWindowText(h, title, title.Capacity);
                if (title.ToString().StartsWith("ToTEngine")) { found = h; return false; }
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int height, bool repaint);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
}
'@
$app = $null
$helper = $null
try {
    $env:SDL_VIDEODRIVER = 'windows'
    # Avoid ShellExecute/UNC publisher prompts for this locally built program.
    $app = Start-Process -FilePath $Binary -ArgumentList @('--external-test', "`"$Report`"") -NoNewWindow -PassThru
    $nativeHandle = $app.Handle
    $window = [IntPtr]::Zero
    for ($i = 0; $i -lt 600; $i++) {
        $window = [SpikeWindow]::ForProcess($app.Id)
        if ($window -ne [IntPtr]::Zero) { break }
        if ($app.HasExited) { throw 'spike exited before opening a window' }
        Start-Sleep -Milliseconds 50
    }
    [uint32]$owner = 0
    [void][SpikeWindow]::GetWindowThreadProcessId($window, [ref]$owner)
    if ($window -eq [IntPtr]::Zero -or $owner -ne $app.Id) { throw 'our SDL window did not open' }
    function Focus-Spike {
        [void][SpikeWindow]::SetForegroundWindow($window)
        Start-Sleep -Milliseconds 250
        if ([SpikeWindow]::GetForegroundWindow() -ne $window) { throw 'cannot focus our own window; no keys sent' }
    }
    function Send-Key([uint32]$Key) {
        if ([SpikeWindow]::GetForegroundWindow() -ne $window) { throw 'our window lost focus; no keys sent' }
        [SpikeWindow]::Key($Key, $false)
        [SpikeWindow]::Key($Key, $true)
        Start-Sleep -Milliseconds 80
    }
    function Send-Text([string]$Text) {
        foreach ($ch in $Text.ToCharArray()) { Send-Key ([uint32][char]([string]$ch).ToUpperInvariant()) }
    }
    Focus-Spike
    Send-Text 'n'
    Start-Sleep -Milliseconds 200
    Send-Text 'athenx'
    Send-Key 8
    Send-Text 'a'
    Send-Key 9
    Send-Key 13
    Send-Text 'n'
    Start-Sleep -Milliseconds 200
    Send-Text 'discard'
    Send-Key 27
    $helper = New-Object System.Windows.Forms.Form
    $helper.Text = 'ToTEngine feasibility focus helper'
    $helper.Show()
    [void][SpikeWindow]::SetForegroundWindow($helper.Handle)
    [System.Windows.Forms.Application]::DoEvents()
    Start-Sleep -Milliseconds 300
    Focus-Spike
    $helper.Close()
    [void][SpikeWindow]::MoveWindow($window, 100, 100, 1200, 800, $true)
    Start-Sleep -Milliseconds 300
    [void][SpikeWindow]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    if (-not $app.WaitForExit(15000)) { throw 'spike did not close' }
    [uint32]$exitCode = 0
    if (-not [SpikeWindow]::GetExitCodeProcess($nativeHandle, [ref]$exitCode)) { throw 'cannot read our process exit code' }
    if ($exitCode -ne 0) { throw "external input check failed: $exitCode" }
    Get-Content -Encoding UTF8 $Report
} finally {
    if ($helper) { $helper.Dispose() }
    if ($app -and -not $app.HasExited) { $app.Kill(); $app.WaitForExit() }
}
