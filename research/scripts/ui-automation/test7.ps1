$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class K7 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr ById(IntPtr parent, string cls, int id){
    foreach(var c in Kids(parent)){ if(C(c)==cls && GetDlgCtrlID(c)==id) return c; }
    return IntPtr.Zero;
  }
  public static IntPtr Dialog(uint pid, string text){
    foreach(var h in Top(pid)){ if(C(h)=="#32770" && (text==null || T(h)==text)) return h; }
    return IntPtr.Zero;
  }
  public static void Click(int x,int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(180);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(80);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[K7]::SetProcessDPIAware() | Out-Null
$log = Join-Path $env:TEMP 'texmod_test7.log'
Set-Content $log 'START'
function Log($m) { Add-Content $log ('{0} {1}' -f (Get-Date -Format HH:mm:ss), $m) }
function HexPtr($h) { if ($h -eq $null) { 'null' } else { '0x{0:X}' -f $h.ToInt64() } }

Get-Process Texmod,Speed,notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$proc = Start-Process -FilePath 'D:\Texmod\Texmod.exe' -PassThru
$proc.Refresh()
$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline -and $proc.MainWindowHandle -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 300; $proc.Refresh() }
$main = $proc.MainWindowHandle
$pid32 = [uint32]$proc.Id
Log ("main=" + (HexPtr $main))
if ($main -eq [IntPtr]::Zero) { exit }
Start-Sleep -Milliseconds 1000
[void][K7]::PostMessage([K7]::ById($main,'Button',100), 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
$dlg = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) { $dlg = [K7]::Dialog($pid32, 'Select Executable'); if ($dlg -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 250 }
Log ("dlg=" + (HexPtr $dlg))
if ($dlg -eq [IntPtr]::Zero) { exit }
$edit = [K7]::ById($dlg, 'Edit', 1148)
$r = New-Object K7+RECT
[K7]::GetWindowRect($edit, [ref]$r) | Out-Null
Log ("edit rect {0},{1} {2},{3}" -f $r.L,$r.T,$r.R,$r.B)
[K7]::SetForegroundWindow($dlg) | Out-Null
Start-Sleep -Milliseconds 300
[K7]::Click([int](($r.L+$r.R)/2), [int](($r.T+$r.B)/2))
Start-Sleep -Milliseconds 600
Log ("fg='" + [K7]::T([K7]::GetForegroundWindow()) + "'")
[System.Windows.Forms.SendKeys]::SendWait('^a')
Start-Sleep -Milliseconds 200
[System.Windows.Forms.SendKeys]::SendWait('D:\Games\Need For Speed - Underground 1\Speed.exe')
Start-Sleep -Milliseconds 400
Log ("edit text now='" + [K7]::T($edit) + "'")
[System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
Start-Sleep -Seconds 3
$still = ([K7]::Dialog($pid32, 'Select Executable') -ne [IntPtr]::Zero)
Log ("dialog open after enter=" + $still)
Log ("main title='" + [K7]::T($main) + "'")
foreach ($c in [K7]::Kids($main)) { $t=[K7]::T($c); if ($t -and [K7]::C($c) -eq 'Static') { Log ("static='" + $t + "'") } }
Start-Sleep -Seconds 600
