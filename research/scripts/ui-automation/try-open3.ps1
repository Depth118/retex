Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class T2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
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
$dlg = [T2]::Find($procId, '#32770', 'Select Executable')
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
$edit = [T2]::ById($dlg, 'Edit', 1148)
$open = [T2]::ById($dlg, 'Button', 1)
"dlg=0x{0:X} enabled={1} fg=0x{2:X}" -f $dlg.ToInt64(), [T2]::IsWindowEnabled($dlg), [T2]::GetForegroundWindow().ToInt64()
"open enabled={0} text='{1}'" -f [T2]::IsWindowEnabled($open), [T2]::T($open)
function Closed { return ([T2]::Find($procId, '#32770', 'Select Executable') -eq [IntPtr]::Zero) }

# attempt 1: Enter to the edit
[T2]::PostMessage($edit, 0x0100, [IntPtr]0x0D, [IntPtr]0) | Out-Null
[T2]::PostMessage($edit, 0x0101, [IntPtr]0x0D, [IntPtr]0) | Out-Null
Start-Sleep -Seconds 2
"after edit Enter: closed=$(Closed)"

if (-not (Closed)) {
  # attempt 2: WM_COMMAND IDOK with button handle
  $lr = [IntPtr]$open.ToInt64()
  [T2]::PostMessage($dlg, 0x0111, [IntPtr]1, $lr) | Out-Null
  Start-Sleep -Seconds 2
  "after WM_COMMAND: closed=$(Closed)"
}
if (-not (Closed)) {
  # attempt 3: synchronous BM_CLICK
  $r = [T2]::SendMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)
  Start-Sleep -Seconds 2
  "after SendMessage BM_CLICK: closed=$(Closed)"
}
if (-not (Closed)) {
  # attempt 4: WM_COMMAND IDOK to the parent of the dialog (main window)
  $main = $p.MainWindowHandle
  [T2]::PostMessage($main, 0x0111, [IntPtr]1, [IntPtr]::Zero) | Out-Null
  Start-Sleep -Seconds 2
  "after main WM_COMMAND: closed=$(Closed)"
}
