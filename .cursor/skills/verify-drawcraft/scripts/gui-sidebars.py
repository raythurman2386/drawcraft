#!/usr/bin/env python3
"""Exercise DrawCraft sidebars through XTest on a private GUI display."""
import ctypes, json, os, subprocess, time
from pathlib import Path

ROOT=Path(os.environ["DRAWCRAFT_ROOT"]);BIN=Path(os.environ["DRAWCRAFT_BIN"]);P=Path(os.environ["DRAWCRAFT_PROOF"]);assert BIN.is_file(),BIN
(P/"run-stopped.txt").unlink(missing_ok=True)
BASE={"version":1,"pages":[{"id":"page-1","name":"Page 1","shapes":[]}],"active_page":"page-1","meta":{}}
for id,kind,x,y,w,h,z in [("a","rectangle",20,30,100,80,0),("b","ellipse",180,110,80,120,1),("c","rectangle",340,20,60,40,2)]:
    BASE["pages"][0]["shapes"].append({"id":id,"kind":kind,"pos":{"x":x,"y":y},"size":{"x":w,"y":h},"rotation":0,"flip_x":False,"flip_y":False,
        "style":{"stroke":"#262626","fill_color":"#d9c8f5","stroke_width":2,"fill":"semi","dash":"solid","opacity":1,"font_size":18,"font_family":"sans"},
        "text":"","src":None,"points":[],"locked":False,"group_id":None,"parent_id":None,"label":"","arrow_start":None,"arrow_end":None,"arrow_style":"straight","arrow_start_head":"none","arrow_end_head":"arrow","z":z})

X=ctypes.CDLL("libX11.so.6");T=ctypes.CDLL("libXtst.so.6")
X.XOpenDisplay.restype=ctypes.c_void_p;X.XDefaultRootWindow.restype=ctypes.c_ulong;X.XStringToKeysym.restype=ctypes.c_ulong
D=ctypes.c_void_p(X.XOpenDisplay(None));assert D

def flush(): X.XFlush(D);X.XSync(D,0)
def move(x,y):
    T.XTestFakeMotionEvent(D,-1,int(x),int(y),0);flush();time.sleep(.2)
def click(x,y):
    move(x,y);T.XTestFakeButtonEvent(D,1,1,0);flush();time.sleep(.3)
    T.XTestFakeButtonEvent(D,1,0,0);flush();time.sleep(.5)
def key(name,down=True):
    kc=X.XKeysymToKeycode(D,X.XStringToKeysym(name.encode()));T.XTestFakeKeyEvent(D,kc,int(down),0);flush();time.sleep(.25)
def chord(name):
    key("Control_L");key(name);key(name,False);key("Control_L",False);time.sleep(.4)
def focus():
    # Keep the pointer still: moving into the native title bar can drag the
    # window on a window-managerless Xvfb and invalidate traced coordinates.
    root=X.XDefaultRootWindow(D);r=ctypes.c_ulong();c=ctypes.c_ulong();i=ctypes.c_int();mask=ctypes.c_uint()
    X.XQueryPointer(D,ctypes.c_ulong(root),ctypes.byref(r),ctypes.byref(c),ctypes.byref(i),ctypes.byref(i),ctypes.byref(i),ctypes.byref(i),ctypes.byref(mask));assert c.value
    X.XSetInputFocus(D,c,1,0);flush();time.sleep(.4)
def screenshot(name):
    time.sleep(.5);subprocess.run(["import","-window","root",str(P/f"{name}.png")],check=True)
def positions(file):return [(s["pos"]["x"],s["pos"]["y"]) for s in json.loads(file.read_text())["pages"][0]["shapes"]]
def stop():subprocess.run(["kill",str((P/"case.pid").read_text())],check=False);time.sleep(.6)
def window_handle():
    root=X.XDefaultRootWindow(D);r=ctypes.c_ulong();parent=ctypes.c_ulong();children=ctypes.POINTER(ctypes.c_ulong)();count=ctypes.c_uint()
    X.XQueryTree(D,root,ctypes.byref(r),ctypes.byref(parent),ctypes.byref(children),ctypes.byref(count));result=[]
    for i in range(count.value):
        rr=ctypes.c_ulong();x=ctypes.c_int();y=ctypes.c_int();w=ctypes.c_uint();h=ctypes.c_uint();bd=ctypes.c_uint();depth=ctypes.c_uint()
        X.XGetGeometry(D,children[i],ctypes.byref(rr),ctypes.byref(x),ctypes.byref(y),ctypes.byref(w),ctypes.byref(h),ctypes.byref(bd),ctypes.byref(depth))
        if w.value>=960 and h.value>=620:result.append((w.value*h.value,children[i]))
    X.XFree(children)
    assert len(result)==1,result
    return max(result)[1]
def all_controls(probe):
    result={}
    for path in probe.glob("*.json"):
        try: result[path.stem]=json.loads(path.read_text())
        except (json.JSONDecodeError,FileNotFoundError):pass
    return result

def launch(name):
    q=P/name;q.mkdir(exist_ok=True);os.environ["XDG_CONFIG_HOME"]=str(q/"xdg")
    file=q/"board.drawcraft";file.write_text(json.dumps(BASE));probe=q/"controls.json";probe.mkdir(exist_ok=True)
    it=json.dumps(["left","center","right","top","middle","bottom"])
    env=dict(os.environ,WAYLAND_DISPLAY="",DRAWCRAFT_GUI_PROOF=str(probe),DRAWCRAFT_GUI_LABELS=it)
    with (P/"environment.json").open("a") as log:
        json.dump({"case":name,"binary":str(BIN),"fixture":str(file),"display":os.environ["DISPLAY"],"proof":str(probe)},log);log.write("\n")
    log=q/"app.log";app=subprocess.Popen([str(BIN),"open",str(file)],stdout=log.open("w"),stderr=subprocess.STDOUT,env=env)
    (P/"case.pid").write_text(str(app.pid));(q/"app.pid").write_text(str(app.pid));time.sleep(4)
    assert app.poll() is None,(q/"app.log").read_text()
    # A bare Xvfb has no WM to focus/map the new window. Activate the actual
    # native window without moving the cursor and wait for the first paint.
    for _ in range(100):
        if (probe/"canvas.json").exists():break
        # With no WM an initial resize/focus alone may precede winit's first
        # redraw. A real pointer move inside the native canvas requests it.
        X.XRaiseWindow(D,window_handle());X.XSetInputFocus(D,window_handle(),1,0)
        move(850,400);time.sleep(.15)
    if not (probe/"canvas.json").exists():
        screenshot(f"{name}-no-first-frame")
    assert (probe/"canvas.json").exists(),f"No first {name} canvas frame"
    for _ in range(5):
        if (probe/"tool-parallelogram.json").exists():break
        X.XRaiseWindow(D,window_handle());flush();time.sleep(.3)
    screenshot(f"{name}-initial")
    return file,q,probe

def controls(probe):
    result={}
    for _ in range(90):
        for control_name in EXPECTED:
            path=probe/f"align-{control_name}.json"
            if path.exists(): result[control_name]=json.loads(path.read_text())
        if len(result)==6: return result
        time.sleep(.15)
    raise AssertionError("No visible complete alignment widget trace")
def select_all():focus();chord("a");time.sleep(.8)
def save_revert(file):
    focus();chord("s");time.sleep(.8);applied=positions(file)
    chord("z");time.sleep(.5);chord("s");time.sleep(.8)
    return applied,positions(file)

def distribution_positions(axis):
    dimension="x" if axis=="horizontal" else "y"
    rows=sorted([(s["pos"][dimension],s["size"][dimension],index,s) for index,s in enumerate(BASE["pages"][0]["shapes"])])
    start=rows[0][0];end=max(x+w for x,w,index,s in rows);gap=(end-start-sum(w for x,w,index,s in rows))/(len(rows)-1)
    result=[None]*len(rows);cursor=start
    for x,w,index,shape in rows:
        pos=dict(shape["pos"]);pos[dimension]=cursor;result[index]=(pos["x"],pos["y"]);cursor+=w+gap
    return result

ORIGINAL=[(20,30),(180,110),(340,20)]
EXPECTED={"left":[(20,30),(20,110),(20,20)],"center":[(160,30),(170,110),(180,20)],"right":[(300,30),(320,110),(340,20)],
    "top":[(20,20),(180,20),(340,20)],"middle":[(20,85),(180,65),(340,105)],"bottom":[(20,150),(180,110),(340,190)]}
results=[]
if os.environ.get("DRAWCRAFT_ALIGNMENT_ONLY") == "1":
    raise SystemExit(0)
for edge,expected in EXPECTED.items():
    only=os.environ.get("DRAWCRAFT_ONLY_CASE")
    if only and edge!=only:continue
    file,q,probe=launch(f"align-{edge}")
    # X11 focus and keyboard input only; the on-screen button controls are not synthesized internally.
    select_all();screenshot(f"align-{edge}-selected");geometry=controls(probe)
    point=geometry[edge];label=point["label"];assert label==f"Align {edge}" and point["enabled"],point
    move(point["x"],point["y"]);time.sleep(1.7);screenshot(f"align-{edge}-tooltip")
    # Toolbar and fit-to-content can settle while a tooltip is visible.
    # Read the actual capturable control center again before the click, never
    # guess a pixel position or route an editing command internally.
    focus();point=controls(probe)[edge]
    click(point["x"],point["y"]);screenshot(f"align-{edge}-clicked")
    applied,undone=save_revert(file)
    assert expected==applied,(edge,expected,applied);assert undone==ORIGINAL,(edge,undone)
    screenshot(f"align-{edge}-undone")
    result={"action":edge,"click":point,"before":ORIGINAL,"after":applied,"undo":undone};results.append(result)
    (q/"result.json").write_text(json.dumps(result,indent=2));print("ALIGN PASS",edge,"after",applied,"undo",undone,flush=True);stop()
(P/"align-actions.json").write_text(json.dumps(results,indent=2));print(json.dumps(results,indent=2))

# Preserve actual baseline for optional comparators; complete-check default is the selected edge suite.

# Final selection and layout checks: still only ordinary X11 pointer/key/UI actions.
file,q,probe=launch("selection-and-layout")
if os.environ.get("DRAWCRAFT_LEGACY_BASELINE"):
    stop()
else:
    for width,height in [(960,620),(1280,800),(1440,900)]:
        X.XResizeWindow(D,window_handle(),width,height);time.sleep(2)
        screenshot(f"rail-{width}x{height}")
        checks=all_controls(probe)
        for control_name,record in checks.items():
            if record.get("scale") is not None:
                left,top,right,bottom=record["rect"];assert right-left>=113, (control_name,record)
    trace=json.loads((probe/"canvas.json").read_text());shape=trace["shape"]
    # canvas trace is in logical egui pixels; XTest uses physical framebuffer pixels.
    click(int(shape["x"]*1.25+.5),int(shape["y"]*1.25+.5));time.sleep(1)
    screenshot("single-selected")
    checks=all_controls(probe)
    for edge in EXPECTED:
        assert checks[f"align-{edge}"]["enabled"] is False, (edge,checks[f"align-{edge}"])
    for name in ("distribute-h","distribute-v"):
        assert checks[name]["enabled"] is False
    before=positions(file);before_document=json.loads(file.read_text());click(checks["align-left"]["x"],checks["align-left"]["y"]);focus();chord("s")
    assert positions(file)==before,"single-selection align must be ignored"
    for name,axis in (("flip-h","flip_x"),("flip-v","flip_y")):
        assert checks[name]["enabled"] is True
        click(checks[name]["x"],checks[name]["y"]);focus();chord("s")
        modified=json.loads(file.read_text())["pages"][0]["shapes"]
        assert [row[axis] for row in modified]==[True,False,False],(name,modified)
        chord("z");focus();chord("s")
        assert json.loads(file.read_text())["pages"][0]["shapes"]==before_document["pages"][0]["shapes"]
    select_all();checks=all_controls(probe)
    for name,expected in (("distribute-h",distribution_positions("horizontal")),("distribute-v",distribution_positions("vertical"))):
        assert checks[name]["enabled"] is True
        click(checks[name]["x"],checks[name]["y"]);focus();chord("s")
        assert positions(file)==expected,(name,positions(file));screenshot(f"{name}-clicked")
        chord("z");focus();chord("s");assert positions(file)==ORIGINAL
    # The widest labels are selected through their actual rectangles, not icon shortcuts.
    for name in ("parallelogram","arrow_shape","double_arrow"):
        row=checks[f"tool-{name}"]
        x=(row["rect"][0]+row["rect"][2])/2;y=(row["rect"][1]+row["rect"][3])/2
        click(x,y);screenshot(f"tool-{name}-selected")
    summary={"single_selection":shape,"all_alignment_disabled":True,"distribution_disabled":True,"single_align_ignored":True,"flip_single_and_undo":True,"distribution_h_v_and_undo":True,"resizes":[[960,620],[1280,800],[1440,900]],"control_snapshot":checks}
    (q/"result.json").write_text(json.dumps(summary,indent=2));stop()

(P/"run-stopped.txt").unlink(missing_ok=True)
(P/"run-passed.txt").write_text("all sidebar GUI assertions passed\n")
