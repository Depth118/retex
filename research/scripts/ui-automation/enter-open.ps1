Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class S2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h, string t);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
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
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [S2]::Find($procId, '#32770', 'Select Executable')
"dlg=0x{0:X}" -f $dlg.ToInt64()
if ($dlg -eq [IntPtr]::Zero) { exit }
$edit = [S2]::ById($dlg, 'Edit', 1148)
"edit=0x{0:X} text='{1}'" -f $edit.ToInt64(), [S2]::T($edit)
$want = 'C:\Windows\SysWOW64\notepad.exe'
[S2]::SetWindowText($edit, $want) | Out-Null
Start-Sleep -Milliseconds 400
"after set text='{0}'" -f [S2]::T($edit)
$r = New-Object S2+RECT
[S2]::GetWindowRect($dlg, [ref]$r) | Out-Null
"dlg rect: {0},{1} {2},{3}" -f $r.L,$r.T,$r.R,$r.B
$ro = New-Object S2+RECT
$open = [S2]::ById($dlg, 'Button', 1)
[S2]::GetWindowRect($open, [ref]$ro) | Out-Null
"open rect: {0},{1} {2},{3}" -f $ro.L,$ro.T,$ro.R,$ro.B
[S2]::SetForegroundWindow($dlg) | Out-Null
Start-Sleep -Milliseconds 500
# real Return key
[S2]::keybd_event(0x0D, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[S2]::keybd_event(0x0D, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Seconds 3
"still open=" + ([S2]::Find($procId, '#32770', 'Select Executable') -ne [IntPtr]::Zero)
"main title='" + [S2]::T($p.MainWindowHandle) + "'"
