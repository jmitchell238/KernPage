# Diagnostic: list the windows (and their controls' text) owned by running MSPUB processes.
# Used to find hidden dialogs that block automation. Prints titles/labels only.
Add-Type @"
using System; using System.Text; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class Win {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public static List<string> Dump(uint pid) {
    var o = new List<string>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid) { o.Add(Describe(h, "")); EnumChildWindows(h, (c, l2) => { o.Add(Describe(c, "    ")); return true; }, IntPtr.Zero); }
      return true; }, IntPtr.Zero);
    return o;
  }
  static string Describe(IntPtr h, string ind) { var t = new StringBuilder(512); var c = new StringBuilder(256);
    GetWindowText(h, t, 512); GetClassName(h, c, 256);
    return ind + (IsWindowVisible(h) ? "[visible] " : "[hidden]  ") + c + " | " + t; }
}
"@
foreach ($p in Get-Process MSPUB -ErrorAction SilentlyContinue) {
  "MSPUB pid $($p.Id) started $($p.StartTime)"
  [Win]::Dump([uint32]$p.Id) | Where-Object { $_ -notmatch '\| $' }
}
