$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Z {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h, string t);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr ById(IntPtr parent, string cls, int id){
    foreach(var c in Kids(parent)){ if(C(c)==cls && GetDlgCtrlID(c)==id) return c; }
    return IntPtr.Zero;
  }
  public static IntPtr ByText(IntPtr parent, string cls, string text){
    foreach(var c in Kids(parent)){ if(C(c)==cls && T(c)==text) return c; }
    return IntPtr.Zero;
  }
  public static IntPtr Dialog(uint pid, string text){
    foreach(var h in Top(pid)){ if(C(h)=="#32770" && (text==null || T(h)==text)) return h; }
    return IntPtr.Zero;
  }
}
"@
$log = Join-Path $env:TEMP 'texmod_test4.log'
Set-Content $log 'START'
function Log($m) { Add-Content $log ('{0} {1}' -f (Get-Date -Format HH:mm:ss), $m) }
function HexPtr($h) { if ($h -eq $null) { return 'null' } else { return ('0x{0:X}' -f $h.ToInt64()) } }

Get-Process Texmod,notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$proc = Start-Process -FilePath 'D:\Texmod\Texmod.exe' -PassThru
$main = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline) {
  $proc.Refresh()
  if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $main = $proc.MainWindowHandle; break }
  Start-Sleep -Milliseconds 300
}
Log ("main=" + (HexPtr $main))
if ($main -eq [IntPtr]::Zero) { exit }
Start-Sleep -Milliseconds 1200
$pid32 = [uint32]$proc.Id

# dump candidate buttons
foreach ($c in [Z]::Kids($main)) {
  if ([Z]::C($c) -eq 'Button') {
    $r = New-Object Z+RECT; [Z]::GetWindowRect($c, [ref]$r) | Out-Null
    Log ("btn id={0} vis={1} text='{2}' {3}x{4}" -f [Z]::GetDlgCtrlID($c), [Z]::IsWindowVisible($c), [Z]::T($c), ($r.R-$r.L), ($r.B-$r.T))
  }
}

# click the target icon by control id 100
$icon = [Z]::ById($main, 'Button', 100)
Log ("icon=" + (HexPtr $icon))
[void][Z]::PostMessage($icon, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)

$dlg = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) {
  $dlg = [Z]::Dialog($pid32, 'Select Executable')
  if ($dlg -ne [IntPtr]::Zero) { break }
  Start-Sleep -Milliseconds 250
}
Log ("dlg=" + (HexPtr $dlg))
if ($dlg -eq [IntPtr]::Zero) {
  foreach ($h in [Z]::Top($pid32)) { Log ("  top cls='{0}' text='{1}'" -f [Z]::C($h), [Z]::T($h)) }
  exit
}

$edit = [Z]::ById($dlg, 'Edit', 1148)
Log ("edit=" + (HexPtr $edit))
$ok = [Z]::SetWindowText($edit, 'C:\Windows\SysWOW64\notepad.exe')
Log ("settext=" + $ok + " read='" + [Z]::T($edit) + "'")
Start-Sleep -Milliseconds 500

$open = [IntPtr]::ById($dlg, 'Button', 1)
Log ("open=" + (HexPtr $open) + " text='" + [Z]::T($open) + "'")
[void][Z]::PostMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Seconds 2
$dlg2 = [Z]::Dialog($pid32, 'Select Executable')
Log ("dlg after open=" + (HexPtr $dlg2))

# if still open, press Enter on the filename edit
if ($dlg2 -ne [IntPtr]::Zero) {
  [void][Z]::PostMessage($edit, 0x0100, [IntPtr]0x0D, [IntPtr]0)
  [void][Z]::PostMessage($edit, 0x0101, [IntPtr]0x0D, [IntPtr]0)
  Start-Sleep -Seconds 2
  Log ("dlg after enter=" + (HexPtr ([Z]::Dialog($pid32, 'Select Executable'))))
}

# report target state
$main = $proc.MainWindowHandle
foreach ($c in [Z]::Kids($main)) { $t=[Z]::T($c); if ($t -and [Z]::C($c) -eq 'Static') { Log ("static='" + $t + "'") } }

$run = [Z]::ById($main, 'Button', 103)
if ($run -eq [IntPtr]::Zero) { $run = [Z]::ByText($main, 'Button', 'Run') }
Log ("run=" + (HexPtr $run))
[void][Z]::PostMessage($run, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Seconds 3

$warn = [Z]::Dialog($pid32, $null)
Log ("warn=" + (HexPtr $warn) + " text='" + $(if($warn -ne [IntPtr]::Zero){[Z]::T($warn)}else{''}) + "'")
if ($warn -ne [IntPtr]::Zero) {
  foreach ($c in [Z]::Kids($warn)) { if ([Z]::C($c) -eq 'Button' -or [Z]::C($c) -eq 'Static') { Log ("  dlg ctrl cls='{0}' id={1} text='{2}'" -f [Z]::C($c), [Z]::GetDlgCtrlID($c), [Z]::T($c)) } }
  foreach ($c in [Z]::Kids($warn)) { if ([Z]::C($c) -eq 'Button' -and ([Z]::T($c) -match 'Yes|OK')) { [void][Z]::PostMessage($c, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero); Log 'clicked warn'; break } }
}
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) { if (Get-Process notepad -ErrorAction SilentlyContinue) { break }; Start-Sleep -Milliseconds 500 }
Log ("notepad started=" + [bool](Get-Process notepad -ErrorAction SilentlyContinue))
Start-Sleep -Seconds 900

