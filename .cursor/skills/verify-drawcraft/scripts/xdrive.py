#!/usr/bin/env python3
"""Minimal XTest driver (no xdotool needed). Usage:
  xdrive.py move X Y | down | up | key KEYSYM | drag X1 Y1 X2 Y2 ... (polyline, button held, release at end)
  xdrive.py dragpath --hold "x,y x,y ..."   (press, move along path, leave button held)
"""
import ctypes, sys, time
X = ctypes.cdll.LoadLibrary("libX11.so.6"); T = ctypes.cdll.LoadLibrary("libXtst.so.6")
X.XOpenDisplay.restype = ctypes.c_void_p
X.XDefaultRootWindow.restype = ctypes.c_ulong
X.XStringToKeysym.restype = ctypes.c_ulong
d = X.XOpenDisplay(None)
if not d: sys.exit("cannot open display")
d = ctypes.c_void_p(d)
def flush(): X.XFlush(d); X.XSync(d, 0)
def move(x, y): T.XTestFakeMotionEvent(d, -1, int(x), int(y), 0); flush()
def btn(down): T.XTestFakeButtonEvent(d, 1, 1 if down else 0, 0); flush()
def key(name):
    kc = X.XKeysymToKeycode(d, X.XStringToKeysym(name.encode()))
    T.XTestFakeKeyEvent(d, kc, 1, 0); flush(); time.sleep(0.05)
    T.XTestFakeKeyEvent(d, kc, 0, 0); flush()
def path(pts, release):
    move(*pts[0]); time.sleep(0.15); btn(True); time.sleep(0.1)
    for (x0,y0),(x1,y1) in zip(pts, pts[1:]):
        for i in range(1, 13):
            move(x0+(x1-x0)*i/12, y0+(y1-y0)*i/12); time.sleep(0.03)
    time.sleep(0.2)
    if release: btn(False); time.sleep(0.2)
def focus_under_pointer():
    root = X.XDefaultRootWindow(d)
    r=ctypes.c_ulong(); c=ctypes.c_ulong(); i=ctypes.c_int()
    X.XQueryPointer(d, ctypes.c_ulong(root), ctypes.byref(r), ctypes.byref(c), ctypes.byref(i), ctypes.byref(i), ctypes.byref(i), ctypes.byref(i), ctypes.byref(ctypes.c_uint()))
    if c.value:
        X.XRaiseWindow(d, c); X.XSetInputFocus(d, c, 1, 0); flush()
    print("focused window", hex(c.value))
a = sys.argv[1:]
if a[0] == "move": move(a[1], a[2])
elif a[0] == "down": btn(True)
elif a[0] == "up": btn(False)
elif a[0] == "key": key(a[1])
elif a[0] == "focus": focus_under_pointer()
elif a[0] in ("drag", "dragpath"):
    hold = "--hold" in a
    pts = [tuple(float(v) for v in p.split(",")) for p in a[1:] if p != "--hold"]
    path(pts, not hold)
time.sleep(0.1)
