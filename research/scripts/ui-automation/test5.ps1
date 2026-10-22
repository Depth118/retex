$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class L2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h, string t);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
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
  public static IntPtr Dialog(uint pid, string text){
    foreach(var h in Top(pid)){ if(C(h)=="#32770" && (text==null || T(h)==text)) return h; }
    return IntPtr.Zero;
  }
  public static void Click(int x,int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(180);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(90);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[L2]::SetProcessDPIAware() | Out-Null
$log = Join-Path $env:TEMP 'texmod_test5.log'
Set-Content $log 'START'
function Log($m) { Add-Content $log ('{0} {1}' -f (Get-Date -Format HH:mm:ss), $m) }
function HexPtr($h) { if ($h -eq $null) { 'null' } else { '0x{0:X}' -f $h.ToInt64() } }

Get-Process Texmod,notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$proc = Start-Process -FilePath 'D:\Texmod\Texmod.exe' -PassThru
$proc.Refresh()
$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline -and $proc.MainWindowHandle -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 300; $proc.Refresh() }
$main = $proc.MainWindowHandle
Log ("main=" + (HexPtr $main))
if ($main -eq [IntPtr]::Zero) { exit }
Start-Sleep -Milliseconds 1000
$pid32 = [uint32]$proc.Id
$icon = [L2]::ById($main, 'Button', 100)
[void][L2]::PostMessage($icon, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
$dlg = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) { $dlg = [L2]::Dialog($pid32, 'Select Executable'); if ($dlg -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 250 }
Log ("dlg=" + (HexPtr $dlg))
if ($dlg -eq [IntPtr]::Zero) { exit }
$want = 'D:\Games\Need For Speed - Underground 1\Speed.exe'
$edit = [L2]::ById($dlg, 'Edit', 1148)
$combo = [L2]::ById($dlg, 'ComboBox', 1148)
$comboex = [L2]::ById($dlg, 'ComboBoxEx32', 1148)
Log ("edit=" + (HexPtr $edit) + " combo=" + (HexPtr $combo) + " comboex=" + (HexPtr $comboex))
[void][L2]::SetWindowText($edit, $want)
[void][L2]::SetWindowText($combo, $want)
[void][L2]::SetWindowText($comboex, $want)
Start-Sleep -Milliseconds 400
Log ("read edit='" + [L2]::T($edit) + "' combo='" + [L2]::T($combo) + "'")
$wp = [IntPtr]((0x0300 -shl 16) -bor 1148)
[void][L2]::SendMessage($dlg, 0x0111, $wp, $combo)
Start-Sleep -Milliseconds 800
$open = [L2]::ById($dlg, 'Button', 1)
Log ("open=" + (HexPtr $open) + " enabled=" + [L2]::IsWindowEnabled($open) + " text='" + [L2]::T($open) + "'")
$r = New-Object L2+RECT
[L2]::GetWindowRect($open, [ref]$r) | Out-Null
Log ("open rect {0},{1} {2},{3}" -f $r.L,$r.T,$r.R,$r.B)
[void][L2]::SetForegroundWindow($dlg)
Start-Sleep -Milliseconds 300
[L2]::Click([int](($r.L+$r.R)/2), [int](($r.T+$r.B)/2))
Start-Sleep -Seconds 2
$still = ([L2]::Dialog($pid32, 'Select Executable') -ne [IntPtr]::Zero)
Log ("still open=$still")
$main = $proc.MainWindowHandle
Log ("main title='" + [L2]::T($main) + "'")
foreach ($c in [L2]::Kids($main)) { $t=[L2]::T($c); if ($t -and [L2]::C($c) -eq 'Static') { Log ("static='" + $t + "'") } }
Start-Sleep -Seconds 600
