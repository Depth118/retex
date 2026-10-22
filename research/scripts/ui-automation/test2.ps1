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
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(64);GetClassName(h,s,64);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(256);GetWindowText(h,s,256);return s.ToString();}
  public static RECT R(IntPtr h){RECT r;GetWindowRect(h,out r);return r;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr WinByClass(uint pid, string cls){IntPtr f=IntPtr.Zero;EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls){f=h;return false;}return true;},IntPtr.Zero);return f;}
  public static void Click(IntPtr h){PostMessage(h,0x00F5,IntPtr.Zero,IntPtr.Zero);}
  public static void SetAnsi(IntPtr e,string t){SendMessage(e,0x000C,IntPtr.Zero,Marshal.StringToHGlobalAnsi(t));}
  public static IntPtr FindButtonByText(IntPtr main, string text){foreach(var c in Kids(main)){if(C(c)=="Button"&&T(c)==text)return c;}return IntPtr.Zero;}
  public static void ClickButtonInDialog(IntPtr dlg, string text, int id){
    foreach(var c in Kids(dlg)){ if(C(c)=="Button" && (T(c)==text || GetDlgCtrlID(c)==id)){ Click(c); return; } }
  }
}
"@
[TM]::SetProcessDPIAware() | Out-Null
$texmod = 'D:\Texmod\Texmod.exe'
$target = 'C:\Windows\SysWOW64\notepad.exe'
$log = Join-Path $env:TEMP 'texmod_test2.log'
Set-Content $log 'START'
function Log($m) { Add-Content $log ('{0} {1}' -f (Get-Date -Format HH:mm:ss), $m) }
Get-Process Texmod -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 400
$proc = Start-Process -FilePath $texmod -PassThru
$main = [IntPtr]::Zero; $deadline = (Get-Date).AddSeconds(15)
while ((Get-Date) -lt $deadline) { $main = [TM]::WinByClass([uint32]$proc.Id, 'tmlwndcls'); if ($main -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 250 }
Log "main=$main"
if ($main -eq [IntPtr]::Zero) { exit }
Start-Sleep -Milliseconds 900
# target icon: 60x60 empty-text button
$icon = [IntPtr]::Zero
foreach ($c in [TM]::Kids($main)) { if ([TM]::C($c) -eq 'Button' -and [TM]::T($c) -eq '') { $r = [TM]::R($c); if (($r.R-$r.L) -eq 60 -and ($r.B-$r.T) -eq 60) { $icon = $c; break } } }
Log "icon=$icon"
[TM]::Click($icon)
$dlg = [IntPtr]::Zero; $deadline = (Get-Date).AddSeconds(8)
while ((Get-Date) -lt $deadline) { $dlg = [TM]::WinByClass([uint32]$proc.Id, '#32770'); if ($dlg -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 200 }
Log "dlg=$dlg"
$edit = [IntPtr]::Zero
foreach ($c in [TM]::Kids($dlg)) { if ([TM]::C($c) -eq 'Edit' -and [TM]::GetDlgCtrlID($c) -eq 1148) { $edit = $c; break } }
Log "edit=$edit"
[TM]::SetAnsi($edit, $target)
Start-Sleep -Milliseconds 900
foreach ($c in [TM]::Kids($dlg)) { if ([TM]::C($c) -eq 'Button' -and [TM]::GetDlgCtrlID($c) -eq 1) { [TM]::Click($c); break } }
Start-Sleep -Seconds 2
$main = [TM]::WinByClass([uint32]$proc.Id, 'tmlwndcls')
Log ("title='" + [TM]::T($main) + "'")
# Run
$run = [TM]::FindButtonByText($main, 'Run')
Log "run=$run"
if ($run -eq [IntPtr]::Zero) { exit }
[TM]::Click($run)
Start-Sleep -Seconds 2
# possible warning dialog (no packages)
$dlg2 = [TM]::WinByClass([uint32]$proc.Id, '#32770')
Log "warn=$dlg2"
if ($dlg2 -ne [IntPtr]::Zero) { [TM]::ClickButtonInDialog($dlg2, 'Yes', 6); Log 'clicked yes' }
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) { if (Get-Process notepad -ErrorAction SilentlyContinue) { break }; Start-Sleep -Milliseconds 500 }
Log ("game started=" + [bool](Get-Process notepad -ErrorAction SilentlyContinue))
# keep alive 10 min for inspection
Start-Sleep -Seconds 600
