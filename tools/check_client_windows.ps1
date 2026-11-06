# Real OS input only; traces are private test instrumentation, never endpoints.
param([string]$Binary = "$PSScriptRoot\..\target\x86_64-pc-windows-gnullvm\debug\totdesktop.exe", [string]$Work = "$PSScriptRoot\..\target\client-windows-check")
$ErrorActionPreference='Stop'
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
        uint scan = MapVirtualKey(vk, 4);
        i.value.key.vk=(ushort)vk;
        i.value.key.scan = (ushort)(scan & 255u);
        i.value.key.flags = (up ? 2u : 0u) | ((vk >= 33u && vk <= 46u) ? 1u : 0u);
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
    [DllImport("kernel32.dll")] private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] private static extern bool AttachThreadInput(uint a,uint b,bool attach);
    [DllImport("user32.dll")] private static extern IntPtr SetActiveWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr SetFocus(IntPtr window);
    public static bool ActivateOwn(IntPtr window,uint pid) {
        uint owner; GetWindowThreadProcessId(window,out owner);
        if(owner!=pid)return false;
        uint other; uint foreground=GetWindowThreadProcessId(GetForegroundWindow(),out other);
        uint current=GetCurrentThreadId(); bool attached=false;
        try {
            if(foreground!=0 && foreground!=current) attached=AttachThreadInput(current,foreground,true);
            SetForegroundWindow(window);SetActiveWindow(window);SetFocus(window);
            return GetForegroundWindow()==window;
        } finally {if(attached)AttachThreadInput(current,foreground,false);}
    }
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int height, bool repaint);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
}
'@

$run = Join-Path $Work ([Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $run -Force)
$env:TOTENGINE_SAVES = Join-Path $run 'saves'
$env:TOTENGINE_CLIENT_TRACE = Join-Path $run 'trace.jsonl'
$env:SDL_VIDEODRIVER = 'windows'
$app=$null
try {
    $app=Start-Process -FilePath $Binary -NoNewWindow -PassThru
    $handle=$app.Handle
    $window=[IntPtr]::Zero
    for($i=0;$i -lt 600;$i++) {
        $window=[SpikeWindow]::ForProcess($app.Id)
        if($window -ne [IntPtr]::Zero){break}
        if($app.HasExited){throw 'client exited before opening'}
        Start-Sleep -Milliseconds 50
    }
    if($window -eq [IntPtr]::Zero){throw 'our client window did not open'}
    if(-not [SpikeWindow]::ActivateOwn($window,$app.Id)){throw "our new client window could not acquire initial focus"}
    Start-Sleep -Milliseconds 300
    function Read-Scene {
        try {return (Get-Content -Encoding UTF8 (Join-Path $run 'trace.painted.json') -Raw | ConvertFrom-Json)} catch {return $null}
    }
    function Wait-Scene([scriptblock]$Predicate) {
        for($i=0;$i -lt 400;$i++) {
            $scene=Read-Scene
            if($scene -and (& $Predicate $scene)){return $scene}
            if($app.HasExited){throw 'client exited during route'}
            Start-Sleep -Milliseconds 50
        }
        $s=Read-Scene
        throw ('UI wait timed out: '+$s.status)
    }
    function Send-Key([uint32]$Key) {
        if([SpikeWindow]::GetForegroundWindow() -ne $window){throw 'our client lost foreground; no input sent'}
        $oldRevision=(Read-Scene).revision
        [SpikeWindow]::Key($Key,$false);[SpikeWindow]::Key($Key,$true)
        if($Key -ne 13 -or -not (Has-Text (Read-Scene) 'Close client? Save first if you want to resume.')) {
            [void](Wait-Scene {param($s) $s.revision -gt $oldRevision})
        }
        Start-Sleep -Milliseconds 50
    }
    function Send-Text([string]$Text) {
        foreach($ch in $Text.ToCharArray()) {Send-Key ([uint32][char]([string]$ch).ToUpperInvariant())}
    }
    function Has-Text($s,[string]$Text) {return @($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text -eq $Text}).Count -gt 0}
    function Focus-Button([string]$Label) {
        for($n=0;$n -lt 30;$n++) {
            $scene=Read-Scene
            $labelDraw=@($scene.draws | Where-Object {$_.kind -eq 'text' -and $_.text -eq $Label})[0]
            if(-not $labelDraw){throw "Button absent: $Label"}
            $selected=@($scene.draws | Where-Object {$_.kind -eq 'rect' -and $_.x -eq ($labelDraw.x-7) -and $_.y -eq ($labelDraw.y-7) -and $_.color -eq '#6b7057'}).Count -gt 0
            if($selected){Send-Key 13;return}
            Send-Key 9
        }
        throw "Button did not focus: $Label"
    }
    function Research {
        $scene=Read-Scene
        if(Has-Text $scene 'Choose research explicitly') {
            Send-Key 40;Send-Key 13
            [void](Wait-Scene {param($s) $s.status -eq 'research-set'})
        }
    }
    function Save-Dialog([string]$Name) {
        Send-Text $Name
        [void](Wait-Scene {param($s) $s.entry -and $s.entry.value -eq $Name})
        Send-Key 13
    }
    [void](Wait-Scene {param($s) $s.phase -eq 'Setup'})
    Focus-Button 'Start Original surface game'
    [void](Wait-Scene {param($s) Has-Text $s 'Choose research explicitly'})
    Focus-Button 'Save pending choice'
    Save-Dialog 'research'
    [void](Wait-Scene {param($s) Has-Text $s 'Choose research explicitly'})
    Research
    Send-Text 'f'
    [void](Wait-Scene {param($s) Has-Text $s 'Name your city'})
    Focus-Button 'Save pending choice'
    Save-Dialog 'founding'
    [void](Wait-Scene {param($s) Has-Text $s 'Name your city'})
    Send-Text 'athenx';Send-Key 8;Send-Text 'a'
    [void](Wait-Scene {param($s) $s.entry -and $s.entry.value -eq 'athena'})
    Send-Key 13
    [void](Wait-Scene {param($s) $s.status -eq 'founded 0'})
    Focus-Button 'Choose production'
    for($n=0;$n -lt 80;$n++) {
        Send-Key 40
        $s=Read-Scene
        if(@($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text.StartsWith([string][char]0x25cf) -and $_.text.Contains('Warriors')}).Count -gt 0){break}
    }
    Send-Key 13
    [void](Wait-Scene {param($s) $s.status -eq 'prod-set'})
    Send-Text 's';Save-Dialog 'normal'
    for($round=0;$round -lt 10;$round++) {
        Research
        $s=Read-Scene
        $turnText=@($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text -match '^Turn ([0-9]+) '})[0].text
        [void]($turnText -match '^Turn ([0-9]+) ');$turn=[int]$Matches[1]
        Send-Text 'e'
        Start-Sleep -Milliseconds 150
        if(Has-Text (Read-Scene) 'End your turn with movable units?'){Send-Key 13}
        [void](Wait-Scene {param($s) $s.phase -eq 'Human' -and @($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text -match ('^Turn '+($turn+1)+' ')}).Count -gt 0})
    }
    Research
    Send-Text 's';Save-Dialog 'tenrounds'
    [void](Wait-Scene {param($s) $s.status -eq 'Engine checkpoint saved: tenrounds'})
    function Load-Slot([string]$Name) { Send-Text 'l'; Save-Dialog $Name }
    Load-Slot 'research';Research;Send-Text 'f';Focus-Button 'Save pending choice';Save-Dialog 'researchnext'
    Send-Key 27;Load-Slot 'founding';Send-Text 'athena';Send-Key 13
    Focus-Button 'Choose production'
    for($n=0;$n -lt 80;$n++) {
        Send-Key 40
        $s=Read-Scene
        if(@($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text.StartsWith([string][char]0x25cf) -and $_.text.Contains('Warriors')}).Count -gt 0){break}
    }
    Send-Key 13;Send-Text 's';Save-Dialog 'foundingnext'
    Load-Slot 'normal';Send-Text 'e'
    if(Has-Text (Read-Scene) 'End your turn with movable units?'){Send-Key 13}
    [void](Wait-Scene {param($s) $s.phase -eq 'Human' -and @($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text.StartsWith('Turn 2 ')}).Count -gt 0})
    Research;Send-Text 's';Save-Dialog 'normalnext'
    if($env:TOTENGINE_CITIZENS_FIXTURES) {
        $citizens=$env:TOTENGINE_CITIZENS_FIXTURES
        Copy-Item (Join-Path $citizens 'mature.cth') (Join-Path $env:TOTENGINE_SAVES 'mature.cth')
        $meta=Get-Content -Encoding UTF8 (Join-Path $citizens 'route.json') -Raw | ConvertFrom-Json
        function Compare-Citizens([string]$Name,[string]$Expected) {
            Send-Text 's';Save-Dialog $Name
            $actual=(Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($Name+'.cth')) -Algorithm SHA256).Hash
            $wanted=(Get-FileHash (Join-Path $citizens ($Expected+'.cth')) -Algorithm SHA256).Hash
            if($actual -ne $wanted){throw "Citizen checkpoint differs: $Name"}
        }
        Load-Slot 'mature';Send-Text 'o'
        for($n=0;$n -lt 21;$n++) {if(Has-Text (Read-Scene) ('Tile '+$meta.worker_tile)){break};Send-Key 39}
        Send-Key 32;Compare-Citizens 'cityentertainer' 'after-entertainer'
        Focus-Button '1: Entertainer';Compare-Citizens 'citytax' 'after-tax'
        Focus-Button '1: Tax collector';Compare-Citizens 'cityscience' 'after-science'
        Focus-Button '1: Scientist';Compare-Citizens 'citycycle' 'after-cycle'
        Focus-Button 'Choose production';Send-Key 40;Send-Key 27
        Focus-Button 'Auto workers (centre tile)';Load-Slot 'citycycle'
        for($n=0;$n -lt 21;$n++) {if(Has-Text (Read-Scene) ('Tile '+$meta.worker_tile)){break};Send-Key 39}
        Send-Key 32;Compare-Citizens 'cityresume' 'after-resume'
        Send-Key 27;Send-Text 'o'
        if($env:TOTENGINE_NAVIGATION_FIXTURE) {
            Copy-Item $env:TOTENGINE_NAVIGATION_FIXTURE (Join-Path $env:TOTENGINE_SAVES 'navigation.cth')
            Load-Slot 'navigation'
            $first=@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text.Contains('population')})[0].text
            Send-Key 34
            $next=@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text.Contains('population')})[0].text
            if($first -eq $next){throw 'city navigation did not change city'}
            Send-Key 33;Send-Text 's';Save-Dialog 'citynavigation'
            if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES 'citynavigation.cth')).Hash -ne (Get-FileHash $env:TOTENGINE_NAVIGATION_FIXTURE).Hash){throw 'city navigation changed gameplay'}
        }
        Send-Key 27
    }
    if($env:TOTENGINE_ENCOUNTER_FIXTURES) {
        foreach($name in @('battle','capture')) {
            $actor=if($name -eq 'battle'){1}else{7}
            Copy-Item (Join-Path $env:TOTENGINE_ENCOUNTER_FIXTURES "before-$name.cth") (Join-Path $env:TOTENGINE_SAVES "$name.cth")
            Load-Slot $name
            for($n=0;$n -lt 80;$n++) {
                if(@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text.EndsWith(" #$actor")}).Count -gt 0){break}
                Send-Text 'w'
            }
            if($n -eq 80){throw 'combat actor absent'}
            if(Has-Text (Read-Scene) 'Viewing [M]'){Send-Text 'm'}
            Send-Text '8';Send-Key 27;Send-Text '8';Send-Key 13
            Send-Text 's';Save-Dialog ($name+'after')
            $actual=(Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($name+'after.cth')) -Algorithm SHA256).Hash
            $expected=(Get-FileHash (Join-Path $env:TOTENGINE_ENCOUNTER_FIXTURES "after-$name.cth") -Algorithm SHA256).Hash
            if($actual -ne $expected){throw "Encounter checkpoint differs: $name"}
        }
    }
    Focus-Button 'Close client'
    Send-Key 13
    if(-not $app.WaitForExit(15000)){throw 'client did not close'}
    [uint32]$code=0
    if(-not [SpikeWindow]::GetExitCodeProcess($handle,[ref]$code) -or $code -ne 0){throw "client exit $code"}
    Write-Output $run
} finally {if($app -and -not $app.HasExited){$app.Kill();$app.WaitForExit()}}
