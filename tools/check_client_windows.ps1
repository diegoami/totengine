# Real OS input only; traces are private test instrumentation, never endpoints.
param([string]$Binary = "$PSScriptRoot\..\target\x86_64-pc-windows-gnullvm\debug\totdesktop.exe", [string]$Work = "$PSScriptRoot\..\target\client-windows-check", [switch]$MovementOnly, [switch]$SelectionOnly)
$openDemo = $env:TOTENGINE_OPEN_DEMO -eq '1'
$startLabel = if ($openDemo) { 'Start open demo' } else { 'Start Original surface game' }
$unitChoice = if ($openDemo) { 'Guards' } else { 'Warriors' }
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
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x,y; }
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int left,top,right,bottom; }
    [DllImport("user32.dll")] private static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern short GetKeyState(int vk);
    public static void Click(IntPtr h,uint pid,double x,double y) {
        uint owner; GetWindowThreadProcessId(h,out owner);
        if(owner!=pid || GetForegroundWindow()!=h)throw new Exception("No mouse input: owned window is not foreground");
        RECT r; if(!GetClientRect(h,out r))throw new Exception("GetClientRect failed");
        double scale=Math.Min((r.right-r.left)/1024.0,(r.bottom-r.top)/768.0);
        POINT p=new POINT();p.x=(int)(((r.right-r.left)-1024*scale)/2+x*scale);p.y=(int)(((r.bottom-r.top)-768*scale)/2+y*scale);
        if(!ClientToScreen(h,ref p) || !SetCursorPos(p.x,p.y))throw new Exception("Mouse positioning failed");
        // Let SDL consume the cursor motion before delivering the button event.
        System.Threading.Thread.Sleep(80);
        var down=new INPUT();down.type=0;down.value.mouse.flags=2;
        var up=new INPUT();up.type=0;up.value.mouse.flags=4;
        if(SendInput(2,new[]{down,up},Marshal.SizeOf(typeof(INPUT)))!=2)throw new Exception("Mouse SendInput failed");
    }
    public static void DoubleClick(IntPtr h,uint pid,double x,double y) {
        Click(h,pid,x,y);System.Threading.Thread.Sleep(60);Click(h,pid,x,y);
    }
    [DllImport("user32.dll")] private static extern IntPtr GetDC(IntPtr h);
    [DllImport("user32.dll")] private static extern int ReleaseDC(IntPtr h,IntPtr dc);
    [DllImport("gdi32.dll")] private static extern uint GetPixel(IntPtr dc,int x,int y);
    public static uint MarkerPixel(IntPtr h,uint pid) {
        uint owner;GetWindowThreadProcessId(h,out owner);
        if(owner!=pid || GetForegroundWindow()!=h)throw new Exception("Only read our foreground marker");
        RECT r;if(!GetClientRect(h,out r))throw new Exception("GetClientRect failed");
        double scale=Math.Min((r.right-r.left)/1024.0,(r.bottom-r.top)/768.0);
        var dc=GetDC(h);
        try {return GetPixel(dc,(int)(((r.right-r.left)-1024*scale)/2+380*scale),(int)(((r.bottom-r.top)-768*scale)/2+374*scale));}
        finally {ReleaseDC(h,dc);}
    }
    public static string Pixel(IntPtr h,uint pid,double x,double y) {
        uint owner;GetWindowThreadProcessId(h,out owner);
        if(owner!=pid || GetForegroundWindow()!=h)throw new Exception("Only read our own foreground unit");
        RECT r;if(!GetClientRect(h,out r))throw new Exception("GetClientRect failed");
        double scale=Math.Min((r.right-r.left)/1024.0,(r.bottom-r.top)/768.0);
        var dc=GetDC(h);
        try {uint c=GetPixel(dc,(int)(((r.right-r.left)-1024*scale)/2+x*scale),(int)(((r.bottom-r.top)-768*scale)/2+y*scale));
            return String.Format("#{0:x2}{1:x2}{2:x2}",c&255,(c>>8)&255,(c>>16)&255);}
        finally {ReleaseDC(h,dc);}
    }
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr h,out RECT r);
    public static void ResizeClient(IntPtr h,uint pid,int w,int height) {
        uint owner;GetWindowThreadProcessId(h,out owner);
        if(owner!=pid || GetForegroundWindow()!=h)throw new Exception("Only resize our own foreground window");
        RECT outer,inner;if(!GetWindowRect(h,out outer)||!GetClientRect(h,out inner))throw new Exception("Window geometry failed");
        if(!MoveWindow(h,outer.left,outer.top,w+outer.right-outer.left-inner.right,height+outer.bottom-outer.top-inner.bottom,true))throw new Exception("Resize failed");
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
    function Send-Chord([uint32]$Key) {
        if([SpikeWindow]::GetForegroundWindow() -ne $window){throw 'our client lost foreground; no input sent'}
        $oldRevision=(Read-Scene).revision
        [SpikeWindow]::Key(17,$false)
        try {[SpikeWindow]::Key($Key,$false);[SpikeWindow]::Key($Key,$true)}
        finally {[SpikeWindow]::Key(17,$true)}
        [void](Wait-Scene {param($s) $s.revision -gt $oldRevision})
    }
    function Click-At([double]$X,[double]$Y) {
        $oldRevision=(Read-Scene).revision
        [SpikeWindow]::Click($window,$app.Id,$X,$Y)
        [void](Wait-Scene {param($s) $s.revision -gt $oldRevision})
    }
    function Click-Button([string]$Label) {
        $t=@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text -eq $Label})[0]
        if(-not $t){throw "Button absent: $Label"}
        Click-At ($t.x+12) ($t.y+8)
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
    if(-not $MovementOnly -and -not $SelectionOnly) {
    Focus-Button $startLabel
    [void](Wait-Scene {param($s) Has-Text $s 'Choose research explicitly'})
    Focus-Button 'Save pending choice'
    Save-Dialog 'research'
    [void](Wait-Scene {param($s) Has-Text $s 'Choose research explicitly'})
    Research
    Send-Text 'b'
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
        if(@($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text.StartsWith([string][char]0x25cf) -and $_.text.Contains($unitChoice)}).Count -gt 0){break}
    }
    Send-Key 13
    [void](Wait-Scene {param($s) $s.status -eq 'prod-set'})
    Send-Chord 83;Save-Dialog 'normal'
    for($round=0;$round -lt 10;$round++) {
        Research
        $s=Read-Scene
        $turnText=@($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text -match '^Turn ([0-9]+) '})[0].text
        [void]($turnText -match '^Turn ([0-9]+) ');$turn=[int]$Matches[1]
        Send-Chord 78
        Start-Sleep -Milliseconds 150
        if(Has-Text (Read-Scene) 'End your turn with movable units?'){Send-Key 13}
        [void](Wait-Scene {param($s) $s.phase -eq 'Human' -and @($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text -match ('^Turn '+($turn+1)+' ')}).Count -gt 0})
    }
    Research
    Send-Chord 83;Save-Dialog 'tenrounds'
    [void](Wait-Scene {param($s) $s.status -eq 'Engine checkpoint saved: tenrounds'})
    function Load-Slot([string]$Name) { Send-Chord 76; Save-Dialog $Name }
    Load-Slot 'research';Research;Send-Text 'b';Focus-Button 'Save pending choice';Save-Dialog 'researchnext'
    Send-Key 27;Load-Slot 'founding';Send-Text 'athena';Send-Key 13
    Focus-Button 'Choose production'
    for($n=0;$n -lt 80;$n++) {
        Send-Key 40
        $s=Read-Scene
        if(@($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text.StartsWith([string][char]0x25cf) -and $_.text.Contains($unitChoice)}).Count -gt 0){break}
    }
    Send-Key 13;Send-Chord 83;Save-Dialog 'foundingnext'
    Load-Slot 'normal';Send-Chord 78
    if(Has-Text (Read-Scene) 'End your turn with movable units?'){Send-Key 13}
    [void](Wait-Scene {param($s) $s.phase -eq 'Human' -and @($s.draws | Where-Object {$_.kind -eq 'text' -and $_.text.StartsWith('Turn 2 ')}).Count -gt 0})
    Research;Send-Chord 83;Save-Dialog 'normalnext'
    if($env:TOTENGINE_CITIZENS_FIXTURES) {
        $citizens=$env:TOTENGINE_CITIZENS_FIXTURES
        Copy-Item (Join-Path $citizens 'mature.cth') (Join-Path $env:TOTENGINE_SAVES 'mature.cth')
        $meta=Get-Content -Encoding UTF8 (Join-Path $citizens 'route.json') -Raw | ConvertFrom-Json
        function Compare-Citizens([string]$Name,[string]$Expected) {
            Send-Chord 83;Save-Dialog $Name
            $actual=(Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($Name+'.cth')) -Algorithm SHA256).Hash
            $wanted=(Get-FileHash (Join-Path $citizens ($Expected+'.cth')) -Algorithm SHA256).Hash
            if($actual -ne $wanted){throw "Citizen checkpoint differs: $Name"}
        }
        Load-Slot 'mature';Focus-Button 'Manage city'
        if(-not (Has-Text (Read-Scene) 'Currently producing:')){throw 'current production label missing'}
        for($n=0;$n -lt 21;$n++) {if(Has-Text (Read-Scene) ('Tile '+$meta.worker_tile)){break};Send-Key 39}
        Send-Key 32;Compare-Citizens 'cityentertainer' 'after-entertainer'
        Focus-Button '1: Entertainer';Compare-Citizens 'citytax' 'after-tax'
        Focus-Button '1: Tax collector';Compare-Citizens 'cityscience' 'after-science'
        Focus-Button '1: Scientist';Compare-Citizens 'citycycle' 'after-cycle'
        Focus-Button 'Choose production';Send-Key 40;Send-Key 27
        Focus-Button 'Auto workers (centre tile)';Load-Slot 'citycycle'
        for($n=0;$n -lt 21;$n++) {if(Has-Text (Read-Scene) ('Tile '+$meta.worker_tile)){break};Send-Key 39}
        Send-Key 32;Compare-Citizens 'cityresume' 'after-resume'
        Send-Key 27;Focus-Button 'Manage city'
        if($env:TOTENGINE_NAVIGATION_FIXTURE) {
            Copy-Item $env:TOTENGINE_NAVIGATION_FIXTURE (Join-Path $env:TOTENGINE_SAVES 'navigation.cth')
            Load-Slot 'navigation'
            $first=@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text.Contains('population')})[0].text
            Send-Key 34
            $next=@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text.Contains('population')})[0].text
            if($first -eq $next){throw 'city navigation did not change city'}
            Send-Key 33;Send-Chord 83;Save-Dialog 'citynavigation'
            if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES 'citynavigation.cth')).Hash -ne (Get-FileHash $env:TOTENGINE_NAVIGATION_FIXTURE).Hash){throw 'city navigation changed gameplay'}
        }
        Send-Key 27
    }
    }
    if($MovementOnly) {
        [void](New-Item -ItemType Directory -Path $env:TOTENGINE_SAVES -Force)
        Copy-Item (Join-Path $env:TOTENGINE_MOVEMENT_FIXTURES 'before.cth') (Join-Path $env:TOTENGINE_SAVES 'movement.cth')
        Focus-Button 'Load engine checkpoint';Save-Dialog 'movement'
        function Load-Slot([string]$Name) { Send-Chord 76; Save-Dialog $Name }
    }
    if($SelectionOnly) {
        [void](New-Item -ItemType Directory -Path $env:TOTENGINE_SAVES -Force)
        Copy-Item (Join-Path $env:TOTENGINE_SELECTION_FIXTURES 'before.cth') (Join-Path $env:TOTENGINE_SAVES 'selectionbefore.cth')
        Focus-Button 'Load engine checkpoint';Save-Dialog 'selectionbefore'
        function Load-Slot([string]$Name) {Send-Chord 76;Save-Dialog $Name}
    }
    if($env:TOTENGINE_SELECTION_FIXTURES) { . (Join-Path $PSScriptRoot 'check_client_selection.ps1') }
    if($env:TOTENGINE_MOVEMENT_FIXTURES) {
        $movement=$env:TOTENGINE_MOVEMENT_FIXTURES
        Copy-Item (Join-Path $movement 'before.cth') (Join-Path $env:TOTENGINE_SAVES 'movement.cth')
        $route=Get-Content -Encoding UTF8 (Join-Path $movement 'route.json') -Raw | ConvertFrom-Json
        function Select-Actor {
            for($n=0;$n -lt 100;$n++) {
                if(@((Read-Scene).draws | Where-Object {$_.text -and $_.text.EndsWith(' #'+$route.actor)}).Count -gt 0){return}
                Send-Text 'w'
            }
            throw 'movement actor absent'
        }
        Load-Slot 'movement';Select-Actor
        if(Has-Text (Read-Scene) 'Moving [V]'){Send-Text 'v'}
        Send-Text 'c';Click-At 380 365;Click-Button ('Activate unit '+$route.actor)
        if(-not (Has-Text (Read-Scene) 'Moving [V]')){throw 'Activate did not enter Moving'}
        $revision=(Read-Scene).revision;$pixels=@{}
        for($n=0;$n -lt 8;$n++) {$pixels[[string][SpikeWindow]::MarkerPixel($window,$app.Id)]=$true;Start-Sleep -Milliseconds 120}
        if($pixels.Count -lt 2){throw 'native selection square did not blink'}
        if((Read-Scene).revision -ne $revision){throw 'native blink generated input'}
        Click-At (380+$route.dx*32) (390+$route.dy*16)
        if((Read-Scene).status -ne 'moved'){throw ('mouse movement failed: '+(Read-Scene).status)}
        Send-Chord 83;Save-Dialog 'movementafter'
        $expected=(Get-FileHash (Join-Path $movement 'after.cth')).Hash
        if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES 'movementafter.cth')).Hash -ne $expected){throw 'mouse movement checkpoint differs'}
        $digit=@{'-1,-1'=7;'0,-2'=8;'1,-1'=9;'-2,0'=4;'2,0'=6;'-1,1'=1;'0,2'=2;'1,1'=3}[([string]$route.dx+','+$route.dy)]
        $originalNumLock=([SpikeWindow]::GetKeyState(144) -band 1) -ne 0
        try {
            foreach($mode in @('on','off','arrows')) {
                Load-Slot 'movement';Select-Actor
                if(Has-Text (Read-Scene) 'Viewing [V]'){Send-Text 'v'}
                $numLock=([SpikeWindow]::GetKeyState(144) -band 1) -ne 0
                if($numLock -ne ($mode -eq 'on')) {
                    if([SpikeWindow]::GetForegroundWindow() -ne $window){throw 'no Num Lock input: client lost foreground'}
                    [SpikeWindow]::Key(144,$false);[SpikeWindow]::Key(144,$true);Start-Sleep -Milliseconds 80
                }
                if($mode -eq 'arrows') {Send-Key (@{2=40;4=37;6=39;8=38}[[int]$digit])}
                else {Send-Key (96+$digit)}
                Send-Chord 83;Save-Dialog ('movementkeypad'+$mode)
                if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES ('movementkeypad'+$mode+'.cth'))).Hash -ne $expected){throw ('keypad checkpoint differs '+$mode)}
            }
        } finally {
            if((([SpikeWindow]::GetKeyState(144) -band 1) -ne 0) -ne $originalNumLock -and [SpikeWindow]::GetForegroundWindow() -eq $window) {
                [SpikeWindow]::Key(144,$false);[SpikeWindow]::Key(144,$true)
            }
        }
    }
    if(-not $MovementOnly -and -not $SelectionOnly -and $env:TOTENGINE_PRODUCTION_FIXTURES) {
        $routes=Get-Content (Join-Path $env:TOTENGINE_PRODUCTION_FIXTURES 'route.json') -Raw | ConvertFrom-Json
        foreach($route in $routes) {
            $name='production'+$route.name
            Copy-Item (Join-Path $env:TOTENGINE_PRODUCTION_FIXTURES ($route.name+'-before.cth')) (Join-Path $env:TOTENGINE_SAVES ($name+'.cth'))
            Load-Slot $name
            foreach($item in $route.inputs) {
                if($item.type -eq 'click'){Click-At $item.x $item.y}
                elseif($item.type -eq 'key'){
                    $vk=@{'ArrowDown'=40;'ArrowUp'=38;'Enter'=13;'Escape'=27}[$item.key]
                    if(-not $vk){throw ('unsupported production key '+$item.key)}
                    Send-Key $vk
                } else {throw 'unsupported production input'}
            }
            Send-Chord 83;Save-Dialog ($name+'after')
            $actual=(Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($name+'after.cth')) -Algorithm SHA256).Hash
            $expected=(Get-FileHash (Join-Path $env:TOTENGINE_PRODUCTION_FIXTURES ($route.name+'-after.cth')) -Algorithm SHA256).Hash
            if($actual -ne $expected){throw ('Production checkpoint differs: '+$route.name)}
            Load-Slot ($name+'after')
        }
    }
    if(-not $MovementOnly -and -not $SelectionOnly -and $env:TOTENGINE_ENCOUNTER_FIXTURES) {
        foreach($name in @('battle','capture')) {
            $actor=if($name -eq 'battle'){1}else{7}
            Copy-Item (Join-Path $env:TOTENGINE_ENCOUNTER_FIXTURES "before-$name.cth") (Join-Path $env:TOTENGINE_SAVES "$name.cth")
            Load-Slot $name
            for($n=0;$n -lt 80;$n++) {
                if(@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.text.EndsWith(" #$actor")}).Count -gt 0){break}
                Send-Text 'w'
            }
            if($n -eq 80){throw 'combat actor absent'}
            if(Has-Text (Read-Scene) 'Viewing [V]'){Send-Text 'v'}
            Send-Text '8';Send-Key 27;Send-Text '8';Send-Key 13
            Send-Chord 83;Save-Dialog ($name+'after')
            $actual=(Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($name+'after.cth')) -Algorithm SHA256).Hash
            $expected=(Get-FileHash (Join-Path $env:TOTENGINE_ENCOUNTER_FIXTURES "after-$name.cth") -Algorithm SHA256).Hash
            if($actual -ne $expected){throw "Encounter checkpoint differs: $name"}
        }
    }
    if(-not $MovementOnly -and -not $SelectionOnly -and $env:TOTENGINE_PANELS_FIXTURES) {
        $routes=Get-Content -Encoding UTF8 (Join-Path $env:TOTENGINE_PANELS_FIXTURES 'route.json') -Raw | ConvertFrom-Json
        foreach($route in $routes) {
            $name='panel'+$route.name
            Copy-Item (Join-Path $env:TOTENGINE_PANELS_FIXTURES ($route.name+'-before.cth')) (Join-Path $env:TOTENGINE_SAVES ($name+'.cth'))
            Load-Slot $name
            if(Has-Text (Read-Scene) 'Manage city'){Click-Button 'Manage city'}
            Click-Button 'Info'
            foreach($step in $route.inputs) {
                $item=$step.input
                if($item.type -eq 'click'){Click-At $item.x $item.y}
                elseif($item.type -eq 'key'){
                    $vk=@{'ArrowDown'=40;'ArrowUp'=38;'Enter'=13;'Escape'=27;'PageDown'=34;'PageUp'=33}[$item.key]
                    if(-not $vk){throw ('unsupported panel key '+$item.key)}
                    Send-Key $vk
                } else {throw 'unsupported panel input'}
                $draws=(Read-Scene).draws
                if(@($draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 744 -and $_.y -eq 193 -and $_.text -eq $step.panel}).Count -ne 1){throw ('Panel transition differs '+$route.name)}
                if($step.text -and @($draws | Where-Object {$_.text -eq $step.text}).Count -ne 1){throw ('Unit roster count differs '+$route.name)}
                if($step.modal){
                    if(@($draws | Where-Object {$_.text -eq $step.modal}).Count -ne 1){throw 'Sale confirmation missing'}
                } else {
                    if(@($draws | Where-Object {$_.text -eq 'Confirm improvement sale'}).Count -ne 0){throw 'Sale confirmation did not close'}
                    if($step.pages -gt 1){
                        $pageText='List '+$step.page+' / '+$step.pages
                        if(@($draws | Where-Object {$_.text -eq $pageText}).Count -ne 1){throw ('List pagination differs '+$route.name)}
                    }
                }
            }
            $scene=Read-Scene
            if(@($scene.draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 744 -and $_.y -eq 193 -and $_.text -eq $route.panel}).Count -ne 1){throw ('Wrong displayed panel '+$route.name)}
            if(@($scene.draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 18 -and $_.y -eq 14 -and $_.text -eq $route.heading}).Count -ne 1){throw ('Wrong displayed city '+$route.name)}
            Send-Chord 83;Save-Dialog ($name+'after')
            $actual=(Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($name+'after.cth')) -Algorithm SHA256).Hash
            $expected=(Get-FileHash (Join-Path $env:TOTENGINE_PANELS_FIXTURES ($route.name+'-after.cth')) -Algorithm SHA256).Hash
            if($actual -ne $expected){throw ('Panel checkpoint differs: '+$route.name)}
            Load-Slot ($name+'after')
            if(@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 744 -and $_.y -eq 193 -and $_.text -eq $route.panel}).Count -ne 1){throw ('Panel reset on reload '+$route.name)}
            Send-Chord 83;Save-Dialog ($name+'reload')
            if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($name+'reload.cth')) -Algorithm SHA256).Hash -ne $expected){throw ('Panel reload checkpoint differs '+$route.name)}
        }
        Send-Key 27
        # #178: map-to-session load defaults to Info; an open workspace retains
        # Happy. Both must preserve the saved production budget exactly.
        Load-Slot 'panelbudgettabs'
        if(@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 744 -and $_.y -eq 193 -and $_.text -eq 'Info'}).Count -ne 1){throw 'Map load retained old city panel'}
        $expected=(Get-FileHash (Join-Path $env:TOTENGINE_PANELS_FIXTURES 'budgettabs-before.cth') -Algorithm SHA256).Hash
        Send-Chord 83;Save-Dialog 'panelnewsession'
        if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES 'panelnewsession.cth') -Algorithm SHA256).Hash -ne $expected){throw 'Info default changed loaded gameplay'}
        Click-Button 'Happy'
        Load-Slot 'panelnewsession'
        if(@((Read-Scene).draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 744 -and $_.y -eq 193 -and $_.text -eq 'Happy'}).Count -ne 1){throw 'Open workspace lost its panel'}
        Send-Chord 83;Save-Dialog 'panelnewsessionreload'
        if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES 'panelnewsessionreload.cth') -Algorithm SHA256).Hash -ne $expected){throw 'Open workspace load changed gameplay'}
        Send-Key 27
    }
    if(-not $MovementOnly -and -not $SelectionOnly -and $env:TOTENGINE_CITY_OPEN_FIXTURE) {
        Copy-Item $env:TOTENGINE_CITY_OPEN_FIXTURE (Join-Path $env:TOTENGINE_SAVES 'cityopen.cth')
        Load-Slot 'cityopen'
        if(Has-Text (Read-Scene) 'Return to map [Esc]'){Send-Key 27}
        $city=@((Read-Scene).draws | Where-Object {$_.kind -eq 'sprite' -and $_.sprite.StartsWith('city-')})[0]
        if(-not $city){throw 'city sprite absent'}
        [SpikeWindow]::DoubleClick($window,$app.Id,($city.x+$city.w/2),($city.y+$city.h*0.6))
        [void](Wait-Scene {param($s) @($s.draws | Where-Object {$_.kind -eq 'text' -and $_.x -eq 18 -and $_.y -eq 14 -and $_.text.Contains('population')}).Count -gt 0})
        Send-Chord 83;Save-Dialog 'citydouble'
        if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES 'citydouble.cth')).Hash -ne (Get-FileHash $env:TOTENGINE_CITY_OPEN_FIXTURE).Hash){throw 'double click changed gameplay'}
        Send-Key 27
    }
    Focus-Button 'Close client'
    Send-Key 13
    if(-not $app.WaitForExit(15000)){throw 'client did not close'}
    [uint32]$code=0
    if(-not [SpikeWindow]::GetExitCodeProcess($handle,[ref]$code) -or $code -ne 0){throw "client exit $code"}
    Write-Output $run
} finally {if($app -and -not $app.HasExited){$app.Kill();$app.WaitForExit()}}
