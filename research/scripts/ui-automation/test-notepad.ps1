$ErrorActionPreference = 'Stop'

Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class TM {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(64);GetClassName(h,s,64);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(256);GetWindowText(h,s,256);return s.ToString();}
  public static RECT R(IntPtr h){RECT r;GetWindowRect(h,out r);return r;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr WinByClass(uint pid, string cls){IntPtr f=IntPtr.Zero;EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls){f=h;return false;}return true;},IntPtr.Zero);return f;}
  public static void Click(IntPtr h){PostMessage(h,0x00F5,IntPtr.Zero,IntPtr.Zero);}
  public static void Key(IntPtr h, int vk){PostMessage(h,0x0100,(IntPtr)vk,IntPtr.Zero);System.Threading.Thread.Sleep(120);PostMessage(h,0x0101,(IntPtr)vk,IntPtr.Zero);}
  public static void PhysClick(int x, int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(200);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(100);mouse_event(4,0,0,0,UIntPtr.Zero);}
  public static void SetAnsi(IntPtr e,string t){SendMessage(e,0x000C,IntPtr.Zero,Marshal.StringToHGlobalAnsi(t));}
  public static int Cnt(IntPtr lv){return (int)SendMessage(lv,0x1004,IntPtr.Zero,IntPtr.Zero);}
  public static IntPtr FindList(IntPtr main){IntPtr f=IntPtr.Zero;foreach(var c in Kids(main)){if(C(c)=="SysListView32"){f=c;break;}}return f;}
  public static IntPtr FindButtonByText(IntPtr main, string text){IntPtr f=IntPtr.Zero;foreach(var c in Kids(main)){if(C(c)=="Button"&&T(c)==text){f=c;break;}}return f;}
  public static bool HasStaticText(IntPtr main, string needle){foreach(var c in Kids(main)){if(C(c)=="Static"&&T(c).IndexOf(needle,StringComparison.OrdinalIgnoreCase)>=0)return true;}return false;}
}
"@

[TM]::SetProcessDPIAware() | Out-Null

$game      = "D:\Games\Need For Speed - Underground 1"
$texmodDir = Join-Path $game "Mods\TexMod\Texmod"
$texmod    = Join-Path $texmodDir "Texmod.exe"
$tpf       = Join-Path $texmodDir "redux1.0.8.tpf"
$exePath   = "C:\Windows\SysWOW64\notepad.exe"
$logFile   = Join-Path $env:TEMP "texmod_test.log"
Set-Content $logFile ("{0} START" -f (Get-Date).ToString('HH:mm:ss'))
function Log($msg) { Add-Content $logFile ("{0} {1}" -f (Get-Date).ToString('HH:mm:ss'), $msg) }

# If the game is already running, don't start a second copy
if (Get-Process notepad -ErrorAction SilentlyContinue) { Log "game already running; exit"; exit }
# Kill a stale TexMod (from an aborted previous session)
Get-Process Texmod -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# Seed TexMod's remembered package so the recents menu always contains it
New-Item -Path "HKCU:\Software\TexMod" -Force | Out-Null
Set-ItemProperty "HKCU:\Software\TexMod" -Name "Packages"   -Value $tpf -Type String
Set-ItemProperty "HKCU:\Software\TexMod" -Name "recentpkg0" -Value $tpf -Type String

# Start TexMod
$proc = Start-Process -FilePath $texmod -WorkingDirectory $texmodDir -PassThru
$main = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(15)
while ((Get-Date) -lt $deadline) {
    $main = [TM]::WinByClass([uint32]$proc.Id, "tmlwndcls")
    if ($main -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 250
}
if ($main -eq [IntPtr]::Zero) { Log "main window not found"; exit }
Start-Sleep -Milliseconds 900
Log "main=$main"

# === 1) target application ===
$targetOk = $false
for ($try = 1; $try -le 4 -and -not $targetOk; $try++) {
    Log "target attempt $try"
    $main = [TM]::WinByClass([uint32]$proc.Id, "tmlwndcls")
    $icon = [IntPtr]::Zero
    foreach ($c in [TM]::Kids($main)) {
        if ([TM]::C($c) -eq "Button" -and [TM]::T($c) -eq "") {
            $r = [TM]::R($c)
            if (($r.R-$r.L) -eq 60 -and ($r.B-$r.T) -eq 60) { $icon = $c; break }
        }
    }
    if ($icon -eq [IntPtr]::Zero) { Log "target icon not found"; break }
    [TM]::Click($icon)
    $dlg = [IntPtr]::Zero
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) { $dlg = [TM]::WinByClass([uint32]$proc.Id, "#32770"); if ($dlg -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 200 }
    if ($dlg -eq [IntPtr]::Zero) { Log "target dialog did not open"; continue }
    $edit = [IntPtr]::Zero
    foreach ($c in [TM]::Kids($dlg)) { if ([TM]::C($c) -eq "Edit" -and [TM]::GetDlgCtrlID($c) -eq 1148) { $edit = $c; break } }
    if ($edit -eq [IntPtr]::Zero) { Log "target edit not found"; continue }
    [TM]::SetAnsi($edit, $exePath)
    Start-Sleep -Milliseconds 900
    foreach ($c in [TM]::Kids($dlg)) { if ([TM]::C($c) -eq "Button" -and [TM]::GetDlgCtrlID($c) -eq 1) { [TM]::Click($c); break } }
    Start-Sleep -Seconds 2
    $main = [TM]::WinByClass([uint32]$proc.Id, "tmlwndcls")
    $targetOk = $true
    Log "target check: title='$([TM]::T($main))' ok=$targetOk"
}
if (-not $targetOk) { Log "TARGET FAILED"; Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue; exit }
Log "target done"

# === 2) package via recents dropdown ===
$pkgOk = $false
for ($try = 1; $try -le 4 -and -not $pkgOk; $try++) {
    Log "package attempt $try"
    $main = [TM]::WinByClass([uint32]$proc.Id, "tmlwndcls")
    $lv = [TM]::FindList($main)
    if ($lv -eq [IntPtr]::Zero) { Log "list not found"; break }
    $lr = [TM]::R($lv)
    $btn = [IntPtr]::Zero
    foreach ($c in [TM]::Kids($main)) {
        if ([TM]::C($c) -eq "Button" -and [TM]::T($c) -eq "") {
            $r = [TM]::R($c)
            if (($r.R-$r.L) -eq 28 -and ($r.B-$r.T) -eq 28 -and $r.L -gt $lr.R -and [Math]::Abs($r.T - $lr.T) -le 2) { $btn = $c }
        }
    }
    if ($btn -eq [IntPtr]::Zero) { Log "dropdown not found"; break }
    [TM]::Click($btn)
    $menu = [IntPtr]::Zero
    $deadline = (Get-Date).AddSeconds(6)
    while ((Get-Date) -lt $deadline) { $menu = [TM]::WinByClass([uint32]$proc.Id, "#32768"); if ($menu -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 200 }
    if ($menu -eq [IntPtr]::Zero) { Log "recents menu did not open"; continue }
    # choose the recent entry with accelerator "1"
    [TM]::Key($menu, 0x31)
    Start-Sleep -Seconds 2
    # close the menu if it is somehow still open
    $menu2 = [TM]::WinByClass([uint32]$proc.Id, "#32768")
    if ($menu2 -ne [IntPtr]::Zero) { [TM]::Key($menu2, 0x1B) }
    $cnt = [TM]::Cnt($lv)
    Log "list count after '1': $cnt"
    if ($cnt -ge 1) { $pkgOk = $true; break }
    # fallback: open menu again and click "Browse..." (first item)
    [TM]::Click($btn)
    $deadline = (Get-Date).AddSeconds(6)
    $menu = [IntPtr]::Zero
    while ((Get-Date) -lt $deadline) { $menu = [TM]::WinByClass([uint32]$proc.Id, "#32768"); if ($menu -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 200 }
    if ($menu -eq [IntPtr]::Zero) { continue }
    $mr = [TM]::R($menu)
    [TM]::PhysClick(($mr.L + 60), ($mr.T + 12))   # "Browse..." row
    $dlg2 = [IntPtr]::Zero
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) { $dlg2 = [TM]::WinByClass([uint32]$proc.Id, "#32770"); if ($dlg2 -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 200 }
    if ($dlg2 -eq [IntPtr]::Zero) { Log "browse dialog did not open"; continue }
    $edit2 = [IntPtr]::Zero
    foreach ($c in [TM]::Kids($dlg2)) { if ([TM]::C($c) -eq "Edit" -and [TM]::GetDlgCtrlID($c) -eq 1148) { $edit2 = $c; break } }
    if ($edit2 -eq [IntPtr]::Zero) { Log "browse edit not found"; continue }
    [TM]::SetAnsi($edit2, $tpf)
    Start-Sleep -Milliseconds 900
    foreach ($c in [TM]::Kids($dlg2)) { if ([TM]::C($c) -eq "Button" -and [TM]::GetDlgCtrlID($c) -eq 1) { [TM]::Click($c); break } }
    Start-Sleep -Seconds 2
    $cnt = [TM]::Cnt($lv)
    Log "list count after browse: $cnt"
    if ($cnt -ge 1) { $pkgOk = $true }
}
if (-not $pkgOk) { Log "PACKAGE FAILED"; Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue; exit }
Log "package done"

# === 3) run ===
$main = [TM]::WinByClass([uint32]$proc.Id, "tmlwndcls")
$run = [TM]::FindButtonByText($main, "Run")
if ($run -eq [IntPtr]::Zero) { Log "run not found"; exit }
[TM]::Click($run)
Log "run clicked"

# === 4) wait for the game, then babysit until it exits ===
$ok = $false
$deadline = (Get-Date).AddSeconds(60)
while ((Get-Date) -lt $deadline) {
    if (Get-Process notepad -ErrorAction SilentlyContinue) { $ok = $true; break }
    Start-Sleep -Milliseconds 500
}
Log "game started=$ok"
if (-not $ok) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue; exit }

$watchDeadline = (Get-Date).AddHours(24)
while ((Get-Date) -lt $watchDeadline) {
    if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { break }
    Start-Sleep -Seconds 5
}
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Log "game exited; texmod closed"

