$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class G2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Ansi, EntryPoint="SetWindowTextA")] public static extern bool SetWindowTextA(IntPtr h, string t);
  [DllImport("user32.dll", CharSet=CharSet.Ansi, EntryPoint="GetWindowTextA")] public static extern int GetWindowTextA(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static string TA(IntPtr h){var s=new StringBuilder(512);GetWindowTextA(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr Dialog(uint pid, string text){
    foreach(var h in Top(pid)){ if(C(h)=="#32770" && (text==null || T(h)==text)) return h; }
    return IntPtr.Zero;
  }
  public static IntPtr ById(IntPtr parent, string cls, int id){
    foreach(var c in Kids(parent)){ if(C(c)==cls && GetDlgCtrlID(c)==id) return c; }
    return IntPtr.Zero;
  }
  public static IntPtr OpenButton(IntPtr dlg){
    RECT d; GetWindowRect(dlg, out d);
    foreach(var c in Kids(dlg)){
      if(C(c)=="Button" && GetDlgCtrlID(c)==1){
        RECT r; GetWindowRect(c, out r);
        if(r.L>=d.L && r.R<=d.R && r.T>=d.T && r.B<=d.B && T(c).Contains("Open")) return c;
      }
    }
    return IntPtr.Zero;
  }
  public static void Click(int x,int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(180);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(90);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[G2]::SetProcessDPIAware() | Out-Null
$log = Join-Path $env:TEMP 'texmod_test6.log'
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
$icon = [G2]::ById($main, 'Button', 100)
[void][G2]::PostMessage($icon, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
$dlg = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(10)
while ((Get-Date) -lt $deadline) { $dlg = [G2]::Dialog($pid32, 'Select Executable'); if ($dlg -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 250 }
Log ("dlg=" + (HexPtr $dlg))
if ($dlg -eq [IntPtr]::Zero) { exit }
$want = 'D:\Games\Need For Speed - Underground 1\Speed.exe'
$edit = [G2]::ById($dlg, 'Edit', 1148)
$ok = [G2]::SetWindowTextA($edit, $want)
Start-Sleep -Milliseconds 600
Log ("setA=" + $ok + " readA='" + [G2]::TA($edit) + "' readW='" + [G2]::T($edit) + "'")
$open = [G2]::OpenButton($dlg)
Log ("open=" + (HexPtr $open))
[void][G2]::PostMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Seconds 2
$still = ([G2]::Dialog($pid32, 'Select Executable') -ne [IntPtr]::Zero)
Log ("after BM_CLICK still open=" + $still)
if ($still) {
  # topmost + physical click
  [G2]::SetWindowPos($dlg, [IntPtr](-1), 0,0,0,0, 0x0001 -bor 0x0002) | Out-Null
  Start-Sleep -Milliseconds 700
  $r = New-Object G2+RECT; [G2]::GetWindowRect($open, [ref]$r) | Out-Null
  $pt = New-Object G2+POINT; $pt.X=[int](($r.L+$r.R)/2); $pt.Y=[int](($r.T+$r.B)/2)
  $at = [G2]::WindowFromPoint($pt)
  Log ("point {0},{1} -> cls='{2}' id={3} text='{4}'" -f $pt.X,$pt.Y,[G2]::C($at),[G2]::GetDlgCtrlID($at),[G2]::T($at))
  [G2]::Click($pt.X,$pt.Y)
  Start-Sleep -Seconds 2
  Log ("after phys click still open=" + ([G2]::Dialog($pid32, 'Select Executable') -ne [IntPtr]::Zero))
}
Log ("main title='" + [G2]::T($main) + "'")
foreach ($c in [G2]::Kids($main)) { $t=[G2]::T($c); if ($t -and [G2]::C($c) -eq 'Static') { Log ("static='" + $t + "'") } }
Start-Sleep -Seconds 600
