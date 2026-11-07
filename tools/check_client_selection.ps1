# Dot-sourced by the owned-window checker; all input uses its guarded helpers.
$selectionRoot=$env:TOTENGINE_SELECTION_FIXTURES
$selectionMeta=Get-Content -Encoding UTF8 (Join-Path $selectionRoot 'route.json') -Raw | ConvertFrom-Json
$selectionA=[int]$selectionMeta.source.slot;$selectionB=[int]$selectionMeta.target.slot
foreach($name in @('before','stack')) {Copy-Item (Join-Path $selectionRoot ($name+'.cth')) (Join-Path $env:TOTENGINE_SAVES ('selection'+$name+'.cth'))}
function Selection-Unit([int]$Slot) {
    $u=@((Read-Scene).draws | Where-Object {$_.kind -eq 'unit_sprite' -and $_.unit -eq $Slot})[0]
    if(-not $u){throw "visible unit absent $Slot"};return $u
}
function Selection-Expect([int]$Slot,[bool]$Moving=$true) {
    if(-not (Selection-Unit $Slot).selected){throw "wrong selected identity $Slot"}
    $label=if($Moving){'Moving [V]'}else{'Viewing [V]'}
    if(-not (Has-Text (Read-Scene) $label)){throw ('wrong input mode: '+(Read-Scene).status)}
}
function Selection-Point([int]$Slot,[int]$Part=1) {
    $u=Selection-Unit $Slot;$p=$selectionMeta.hit_points[$Part]
    return @(($u.x+$p.x*$u.w),($u.y+$p.y*$u.h))
}
function Selection-Pointer([int]$Slot,[int]$Part=1) {$p=Selection-Point $Slot $Part;Click-At $p[0] $p[1]}
function Selection-Compare([string]$Name,[string]$Wanted) {
    Send-Chord 83;Save-Dialog $Name
    if((Get-FileHash (Join-Path $env:TOTENGINE_SAVES ($Name+'.cth'))).Hash -ne (Get-FileHash (Join-Path $selectionRoot ($Wanted+'.cth'))).Hash){throw "selection checkpoint differs $Name"}
}
Load-Slot 'selectionbefore';Selection-Expect $selectionA
Selection-Pointer $selectionB 0;Selection-Expect $selectionB
if((Selection-Unit $selectionA).moves_left -ne 3 -or (Selection-Unit $selectionB).moves_left -ne 3){throw 'selection spent movement'}
$revision=(Read-Scene).revision;$pixels=@{};$stable=@{};$pb=Selection-Point $selectionB;$pa=Selection-Point $selectionA
for($n=0;$n -lt 12;$n++) {
    $pixels[[SpikeWindow]::Pixel($window,$app.Id,$pb[0],$pb[1])]=$true
    $stable[[SpikeWindow]::Pixel($window,$app.Id,$pa[0],$pa[1])]=$true
    Start-Sleep -Milliseconds 100
}
if($pixels.Count -lt 2 -or -not $pixels.ContainsKey($selectionMeta.hit_points[1].rgb) -or $stable.Count -ne 1){throw ('selected sprite must blink, other unit stays visible: '+($pixels.Keys -join ','))}
if((Read-Scene).revision -ne $revision){throw 'blink generated input'}
for($n=0;$n -lt 100;$n++) {if([SpikeWindow]::Pixel($window,$app.Id,$pb[0],$pb[1]) -ne $selectionMeta.hit_points[1].rgb){break};Start-Sleep -Milliseconds 20}
if($n -eq 100){throw 'unit did not enter hidden phase'}
Selection-Pointer $selectionB;Selection-Expect $selectionB
if(-not (Read-Scene).status.StartsWith("Unit activated.")){throw "hidden-phase click missed the unit"}
Selection-Pointer $selectionA 2;Selection-Expect $selectionA;Send-Text 'v';Selection-Expect $selectionA $false
Selection-Pointer $selectionB 2;Selection-Expect $selectionB;Selection-Compare 'selectioninert' 'before'
[SpikeWindow]::ResizeClient($window,$app.Id,1280,900);Start-Sleep -Milliseconds 250;Send-Text 'z'
Selection-Pointer $selectionA;Selection-Expect $selectionA;Selection-Pointer $selectionB;Selection-Expect $selectionB
Send-Key 189;[SpikeWindow]::ResizeClient($window,$app.Id,1024,768);Start-Sleep -Milliseconds 250
Selection-Compare 'selectionresized' 'before'
Load-Slot 'selectionstack';$u=@((Read-Scene).draws | Where-Object {$_.kind -eq 'unit_sprite' -and $_.own})[0]
Selection-Pointer $u.unit
if(-not (Has-Text (Read-Scene) 'Activate a unit')){throw 'stack chooser absent'}
for($n=0;$n -lt 20;$n++) {
    Send-Key 40
    if(@((Read-Scene).draws | Where-Object {$_.text -and $_.text.StartsWith([string][char]0x25cf) -and $_.text.Contains('#'+$selectionB+' ')}).Count -gt 0){break}
}
if($n -eq 20){throw 'stack member absent'}
Send-Key 13;Selection-Expect $selectionB;Selection-Compare 'selectionstackinert' 'stack'
Load-Slot 'selectionbefore';Selection-Pointer $selectionB;Selection-Expect $selectionB
$u=Selection-Unit $selectionB
Click-At ($u.x+$u.w/2+$selectionMeta.move_delta[0]*32) ($u.y+$u.h*.75+$selectionMeta.move_delta[1]*16)
Selection-Expect $selectionA
if((Selection-Unit $selectionB).moves_left -ne 0){throw 'unit not exhausted'}
Selection-Compare 'selectionmove' 'after-move'
Selection-Pointer $selectionB;Selection-Expect $selectionB $false
if((Selection-Unit $selectionB).blink){throw 'exhausted unit blinking'}
Send-Text 'v';Selection-Expect $selectionA;Send-Key 32
if(-not (Has-Text (Read-Scene) 'End of turn')){throw 'end-of-turn state absent'}
Selection-Compare 'selectionskip' 'after-skip';Send-Chord 78
[void](Wait-Scene {param($s) $s.phase -eq 'Human' -and @($s.draws | Where-Object {$_.text -and $_.text.StartsWith('Turn '+($selectionMeta.turn+1)+' ')}).Count -gt 0})
Research
if(@((Read-Scene).draws | Where-Object {$_.kind -eq 'unit_sprite' -and $_.selected -and $_.blink -and $_.moves_left -gt 0}).Count -eq 0){throw 'new turn has no ready selection'}
Selection-Compare 'selectionturn' 'after-turn'
