Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class N2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h, string t);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
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
$dlg = [N2]::Find($procId, '#32770', 'Select Executable')
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
$edit = [N2]::ById($dlg, 'Edit', 1148)
$open = [N2]::ById($dlg, 'Button', 1)
$want = 'C:\Windows\SysWOW64\notepad.exe'
[N2]::SetWindowText($edit, $want) | Out-Null
Start-Sleep -Milliseconds 300
"text='{0}'" -f [N2]::T($edit)
"open enabled before EN_CHANGE = {0}" -f [N2]::IsWindowEnabled($open)
# EN_CHANGE = 0x0300, control id 1148
$wp = [IntPtr]((0x0300 -shl 16) -bor 1148)
[N2]::SendMessage($dlg, 0x0111, $wp, $edit) | Out-Null
Start-Sleep -Milliseconds 800
"open enabled after EN_CHANGE = {0}" -f [N2]::IsWindowEnabled($open)
if ([N2]::IsWindowEnabled($open)) {
  [N2]::PostMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
  Start-Sleep -Seconds 2
  "closed after BM_CLICK = " + ([N2]::Find($procId, '#32770', 'Select Executable') -eq [IntPtr]::Zero)
  if ([N2]::Find($procId, '#32770', 'Select Executable') -ne [IntPtr]::Zero) {
    [N2]::SendMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Start-Sleep -Seconds 2
    "closed after SendMessage = " + ([N2]::Find($procId, '#32770', 'Select Executable') -eq [IntPtr]::Zero)
  }
}
"main title='" + [N2]::T($p.MainWindowHandle) + "'"
