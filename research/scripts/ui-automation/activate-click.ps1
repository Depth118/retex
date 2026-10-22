Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class D3 {
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
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr Dialog(uint pid, string text){
    foreach(var h in Top(pid)){ if(C(h)=="#32770" && (text==null || T(h)==text)) return h; }
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
  public static void Click(int x,int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(150);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(70);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[D3]::SetProcessDPIAware() | Out-Null
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [D3]::Dialog($procId, 'Select Executable')
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
$dr = New-Object D3+RECT; [D3]::GetWindowRect($dlg, [ref]$dr) | Out-Null
$open = [D3]::OpenButton($dlg)
$or = New-Object D3+RECT; [D3]::GetWindowRect($open, [ref]$or) | Out-Null
"dlg {0},{1} {2},{3}; open {4},{5} {6},{7}" -f $dr.L,$dr.T,$dr.R,$dr.B,$or.L,$or.T,$or.R,$or.B
[D3]::SetWindowPos($dlg, [IntPtr](-1), 0,0,0,0, 0x0001 -bor 0x0002) | Out-Null
Start-Sleep -Milliseconds 500
# click title bar twice to activate
[D3]::Click([int](($dr.L+$dr.R)/2), ($dr.T+14))
Start-Sleep -Milliseconds 300
[D3]::Click([int](($dr.L+$dr.R)/2), ($dr.T+14))
Start-Sleep -Milliseconds 500
"fg=0x{0:X} dlg=0x{1:X}" -f [D3]::GetForegroundWindow().ToInt64(), $dlg.ToInt64()
# click Open twice
[D3]::Click([int](($or.L+$or.R)/2), [int](($or.T+$or.B)/2))
Start-Sleep -Milliseconds 400
[D3]::Click([int](($or.L+$or.R)/2), [int](($or.T+$or.B)/2))
Start-Sleep -Seconds 2
"closed=" + ([D3]::Dialog($procId, 'Select Executable') -eq [IntPtr]::Zero)
"main title='" + [D3]::T($p.MainWindowHandle) + "'"
foreach ($c in [D3]::Kids($p.MainWindowHandle)) { $t=[D3]::T($c); if ($t -and [D3]::C($c) -eq 'Static') { "static='$t'" } }
