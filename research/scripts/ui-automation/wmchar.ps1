Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CH {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static IntPtr Find(uint pid,string cls,string txt){IntPtr f=IntPtr.Zero;EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls&&T(h)==txt){f=h;return false;}return true;},IntPtr.Zero);return f;}
  public static IntPtr ById(IntPtr dlg,string cls,int id){IntPtr f=IntPtr.Zero;EnumChildWindows(dlg,(h,l)=>{if(C(h)==cls&&GetDlgCtrlID(h)==id){f=h;return false;}return true;},IntPtr.Zero);return f;}
  public static void TypeText(IntPtr edit, string s){
    SendMessage(edit, 0x00B1, (IntPtr)0, (IntPtr)(-1)); // EM_SETSEL 0,-1
    SendMessage(edit, 0x0303, IntPtr.Zero, IntPtr.Zero); // WM_CLEAR
    foreach(char ch in s){ SendMessage(edit, 0x0102, (IntPtr)ch, (IntPtr)1); }
  }
}
"@
$p=Get-Process Texmod; $pid32=[uint32]$p.Id
$dlg=[CH]::Find($pid32,'#32770','Select Executable'); if($dlg -eq [IntPtr]::Zero){'no dialog';exit}
$edit=[CH]::ById($dlg,'Edit',1148); $open=[CH]::ById($dlg,'Button',1)
"edit=0x{0:X} open=0x{1:X}" -f $edit.ToInt64(),$open.ToInt64()
"before='{0}'" -f [CH]::T($edit)
[CH]::TypeText($edit, 'D:\Games\Need For Speed - Underground 1\Speed.exe')
Start-Sleep -Milliseconds 500
"after='{0}'" -f [CH]::T($edit)
[void][CH]::PostMessage($open,0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
Start-Sleep -Seconds 2
"still open after bm=" + ([CH]::Find($pid32,'#32770','Select Executable') -ne [IntPtr]::Zero)
$p.Refresh(); "main title='$($p.MainWindowTitle)'"