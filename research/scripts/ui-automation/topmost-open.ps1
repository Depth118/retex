Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class F2 {
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
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
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
  public static void Click(int x,int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(180);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(90);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[F2]::SetProcessDPIAware() | Out-Null
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [F2]::Dialog($procId, 'Select Executable')
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
$dr = New-Object F2+RECT; [F2]::GetWindowRect($dlg, [ref]$dr) | Out-Null
"dlg rect {0},{1} {2},{3}" -f $dr.L,$dr.T,$dr.R,$dr.B
$open = [F2]::OpenButton($dlg)
"open=0x{0:X} text='{1}'" -f $open.ToInt64(), [F2]::T($open)
$or = New-Object F2+RECT; [F2]::GetWindowRect($open, [ref]$or) | Out-Null
"open rect {0},{1} {2},{3}" -f $or.L,$or.T,$or.R,$or.B
# force topmost
[F2]::SetWindowPos($dlg, [IntPtr](-1), 0,0,0,0, 0x0001 -bor 0x0002) | Out-Null
Start-Sleep -Milliseconds 700
$pt = New-Object F2+POINT; $pt.X=[int](($or.L+$or.R)/2); $pt.Y=[int](($or.T+$or.B)/2)
$at = [F2]::WindowFromPoint($pt)
"window at open center = 0x{0:X} cls='{1}' id={2} text='{3}'" -f $at.ToInt64(),[F2]::C($at),[F2]::GetDlgCtrlID($at),[F2]::T($at)
[F2]::Click($pt.X, $pt.Y)
Start-Sleep -Seconds 2
"closed=" + ([F2]::Dialog($procId, 'Select Executable') -eq [IntPtr]::Zero)
$main = $p.MainWindowHandle
"main title='" + [F2]::T($main) + "'"
