Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class C3 {
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
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static IntPtr Find(uint pid, string cls, string text){
    IntPtr f=IntPtr.Zero;
    EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls&&T(h)==text){f=h;return false;}return true;},IntPtr.Zero);
    return f;
  }
  public static IntPtr ById(IntPtr parent, string cls, int id){
    IntPtr f=IntPtr.Zero;
    EnumChildWindows(parent,(h,l)=>{if(C(h)==cls&&GetDlgCtrlID(h)==id){f=h;return false;}return true;},IntPtr.Zero);
    return f;
  }
}
"@
[C3]::SetProcessDPIAware() | Out-Null
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [C3]::Find($procId, '#32770', 'Select Executable')
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
[C3]::SetForegroundWindow($dlg) | Out-Null
Start-Sleep -Milliseconds 400
$cancel = [C3]::ById($dlg, 'Button', 2)
$r = New-Object C3+RECT; [C3]::GetWindowRect($cancel, [ref]$r) | Out-Null
"cancel rect {0},{1} {2},{3} text='{4}'" -f $r.L,$r.T,$r.R,$r.B, [C3]::T($cancel)
[C3]::SetCursorPos([int](($r.L+$r.R)/2), [int](($r.T+$r.B)/2)) | Out-Null
Start-Sleep -Milliseconds 300
[C3]::mouse_event(2,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 120; [C3]::mouse_event(4,0,0,0,[UIntPtr]::Zero)
Start-Sleep -Seconds 2
"closed after Cancel click: " + ([C3]::Find($procId, '#32770', 'Select Executable') -eq [IntPtr]::Zero)
