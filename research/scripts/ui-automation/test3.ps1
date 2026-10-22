$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Y {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h, string t);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static void PhysClick(int x, int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(150);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(80);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[Y]::SetProcessDPIAware() | Out-Null
$log = Join-Path $env:TEMP 'texmod_test3.log'
Set-Content $log 'START'
function Log($m) { Add-Content $log ('{0} {1}' -f (Get-Date -Format HH:mm:ss), $m) }
Get-Process Texmod,notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
$proc = Start-Process -FilePath 'D:\Texmod\Texmod.exe' -PassThru
$deadline = (Get-Date).AddSeconds(20)
$main = [IntPtr]::Zero
while ((Get-Date) -lt $deadline) {
  $proc.Refresh()
  if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $main = $proc.MainWindowHandle; break }
  Start-Sleep -Milliseconds 300
}
if ($main -eq [IntPtr]::Zero) { Log 'no main window'; exit }
Start-Sleep -Milliseconds 800
Log ("main=0x{0:X}" -f $main.ToInt64())
# click target icon
$icon = [IntPtr]::Zero
foreach ($c in [Y]::Kids($main)) { if ([Y]::C($c) -eq 'Button' -and [Y]::T($c) -eq '') { $r = New-Object Y+RECT; [Y]::GetWindowRect($c,[ref]$r) | Out-Null; if (($r.R-$r.L) -eq 60 -and ($r.B-$r.T) -eq 60) { $icon = $c; break } } }
Log ("icon=0x{0:X}" -f $icon.ToInt64())
[void][Y]::PostMessage($icon, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
$deadline = (Get-Date).AddSeconds(8)
$dlg = [IntPtr]::Zero
while ((Get-Date) -lt $deadline) { $dlg = [Y]::Top([uint32]$proc.Id) | Where-Object { [Y]::C($_) -eq '#32770' -and [Y]::T($_) -eq 'Select Executable' } | Select-Object -First 1; if ($dlg -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 200 }
Log ("dlg=0x{0:X}" -f $dlg.ToInt64())
if ($dlg -eq [IntPtr]::Zero) { exit }
$edit = [IntPtr]::Zero
foreach ($c in [Y]::Kids($dlg)) { if ([Y]::C($c) -eq 'Edit' -and [Y]::GetDlgCtrlID($c) -eq 1148) { $edit = $c; break } }
$ok = [Y]::SetWindowText($edit, 'C:\Windows\SysWOW64\notepad.exe')
Log ("settext ok=" + $ok + " read='" + [Y]::T($edit) + "'")
$open = [IntPtr]::Zero
foreach ($c in [Y]::Kids($dlg)) { if ([Y]::C($c) -eq 'Button' -and [Y]::GetDlgCtrlID($c) -eq 1 -and [Y]::IsWindowVisible($c)) { $open = $c; break } }
Log ("open=0x{0:X} enabled={1}" -f $open.ToInt64(), [Y]::IsWindowEnabled($open))
[void][Y]::PostMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Seconds 2
$still = ([Y]::Top([uint32]$proc.Id) | Where-Object { [Y]::C($_) -eq '#32770' -and [Y]::T($_) -eq 'Select Executable' }).Count
Log "dialog open after BM_CLICK: $still"
if ($still -gt 0) {
  [Y]::SetForegroundWindow($dlg) | Out-Null
  Start-Sleep -Milliseconds 300
  $r = New-Object Y+RECT; [Y]::GetWindowRect($open,[ref]$r) | Out-Null
  [Y]::PhysClick([int](($r.L+$r.R)/2), [int](($r.T+$r.B)/2))
  Start-Sleep -Seconds 2
  $still = ([Y]::Top([uint32]$proc.Id) | Where-Object { [Y]::C($_) -eq '#32770' -and [Y]::T($_) -eq 'Select Executable' }).Count
  Log "dialog open after phys click: $still"
}
$static = ''
foreach ($c in [Y]::Kids($main)) { if ([Y]::C($c) -eq 'Static' -and [Y]::T($c) -match 'notepad') { $static = [Y]::T($c) } }
Log "target static='$static'"
# run
$run = [IntPtr]::Zero
foreach ($c in [Y]::Kids($main)) { if ([Y]::C($c) -eq 'Button' -and [Y]::T($c) -eq 'Run') { $run = $c; break } }
Log ("run=0x{0:X}" -f $run.ToInt64())
[void][Y]::PostMessage($run, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Seconds 2
$warn = [Y]::Top([uint32]$proc.Id) | Where-Object { [Y]::C($_) -eq '#32770' } | Select-Object -First 1
Log ("warn=0x{0:X} text='{1}'" -f $warn.ToInt64(), [Y]::T($warn))
if ($warn -ne [IntPtr]::Zero) {
  foreach ($c in [Y]::Kids($warn)) { $ct=[Y]::T($c); Log "  dlgbtn cls=$([Y]::C($c)) id=$([Y]::GetDlgCtrlID($c)) text='$ct'" }
  foreach ($c in [Y]::Kids($warn)) { if ([Y]::C($c) -eq 'Button' -and ([Y]::T($c) -match 'Yes|&Yes|OK')) { [void][Y]::PostMessage($c, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero); Log 'clicked warn button'; break } }
}
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) { if (Get-Process notepad -ErrorAction SilentlyContinue) { break }; Start-Sleep -Milliseconds 500 }
Log ("notepad started=" + [bool](Get-Process notepad -ErrorAction SilentlyContinue))
Start-Sleep -Seconds 900


