#!/usr/bin/env python3
"""Native XTest proof of shared Escape rollback + commit/undo. NOT touch hardware proof."""
import json, os, subprocess, sys, time
from pathlib import Path

P=Path(sys.argv[1]).resolve();P.mkdir(parents=True,exist_ok=True)
BIN=Path(os.environ.get("DRAWCRAFT_BIN","target/debug/drawcraft")).resolve()
DRIVER=Path(__file__).with_name("xdrive.py")
assert BIN.is_file(),BIN
assert not (P/"board.drawcraft").exists(),"Use a fresh disposable proof directory"
board=P/"board.drawcraft"
board.write_text(json.dumps({"version":1,"pages":[{"id":"page-1","name":"Page 1","shapes":[]}],"active_page":"page-1","meta":{}}))
env=dict(os.environ,DISPLAY=os.environ.get("DRAWCRAFT_DISPLAY",":194"),XDG_CONFIG_HOME=str(P/"xdg"),XDG_RUNTIME_DIR=str(P/"runtime"))
env.pop("WAYLAND_DISPLAY",None)
Path(env["XDG_RUNTIME_DIR"]).mkdir(mode=0o700)

def drive(*args):
    subprocess.run([sys.executable,str(DRIVER),*map(str,args)],env=env,check=True)
    time.sleep(.12)
def chord(key):
    drive("keydown","Control_L");drive("key",key);drive("keyup","Control_L");time.sleep(.25)
def capture(name):
    time.sleep(.3)
    subprocess.run(["import","-window","root",str(P/f"{name}.png")],env=env,check=True)
def saved(name,count):
    chord("s")
    doc=json.loads(board.read_text());assert sum(len(p["shapes"]) for p in doc["pages"])==count,(name,doc)
    (P/f"{name}.json").write_text(json.dumps(doc,indent=2))
    print(name,"shape_count",count,flush=True)
    return doc

def erase(hold):
    drive("key","e")
    # Begin inside the first rectangle we drew, not the old narrow tool rail.
    args=["dragpath","350,440","370,440","850,440"]
    if hold:args.append("--hold")
    drive(*args)

xvfb=app=None
try:
    xvfb=subprocess.Popen(["Xvfb",env["DISPLAY"],"-screen","0","1400x900x24","-nolisten","tcp"],stdout=(P/"xvfb.log").open("w"),stderr=subprocess.STDOUT)
    (P/"xvfb.pid").write_text(str(xvfb.pid));time.sleep(1)
    assert xvfb.poll() is None,"Private display unavailable; choose DRAWCRAFT_DISPLAY"
    app=subprocess.Popen([str(BIN),"open",str(board)],env=env,stdout=(P/"app.log").open("w"),stderr=subprocess.STDOUT)
    (P/"app.pid").write_text(str(app.pid));time.sleep(4);assert app.poll() is None
    drive("move",700,400);drive("focus");capture("00-opened")
    states=[]
    for i,x in enumerate([310,510,710,910],1):
        drive("key","r");drive("dragpath",f"{x},400",f"{x+80},480")
        states.append(saved(f"created-{i}",i))
    chord("z");before=saved("01-before-cancel-undo3-redo1",3);assert before==states[2]
    capture("01-before-cancel")
    erase(True);saved("02-erased-three-live",0);capture("02-erased-three-live")
    drive("key","Escape");drive("up")
    assert saved("03-after-escape-cancel",3)==before
    capture("03-after-escape-cancel")
    # Observe all existing history states via real shortcuts, not diagnostic setters.
    for count in [2,1,0]:
        chord("z");doc=saved(f"cancel-undo-{count}",count)
        if count:assert doc==states[count-1]
    chord("z");saved("cancel-undo-at-bottom",0)
    for count in [1,2,3,4]:
        chord("y");assert saved(f"cancel-redo-{count}",count)==states[count-1]
    chord("y");assert saved("cancel-redo-at-top",4)==states[3]
    # Normal erasure still commits with one existing undo entry per deleted shape.
    chord("z");assert saved("04-before-commit",3)==before
    erase(False);saved("05-erase-committed",0);capture("05-erase-committed")
    for count in [1,2,3]:
        chord("z");doc=saved(f"committed-erase-undo-{count}",count)
    assert doc==before
    capture("06-committed-erase-undone")
    for count in [2,1,0]:
        chord("y");saved(f"committed-erase-redo-{count}",count)
    for _ in range(3):chord("z")
    assert saved("move-before",3)==before
    drive("key","v");drive("dragpath","350,440","410,470","--hold")
    assert saved("move-live",3)!=before
    capture("07-move-live")
    drive("key","Escape");drive("up")
    assert saved("08-move-cancelled",3)==before
    capture("08-move-cancelled")
    drive("dragpath","350,440","410,470")
    moved=saved("09-move-committed",3);assert moved!=before
    capture("09-move-committed")
    chord("z");assert saved("10-move-undone",3)==before
    chord("y");assert saved("11-move-redone",3)==moved
    capture("11-move-redone")
    result={"surface":"native X11 XTest mouse + keyboard Escape (not touch hardware)","cancel_document_exact":True,"cancel_history_undo_depth":3,"cancel_history_redo_depth":1,"all_preexisting_history_states_verified_via_shortcuts":True,"normal_erase_per_delete_undo_redo":True,"move_cancel_exact":True,"normal_move_single_undo_redo":True,"second_finger_cancellation":"synthetic egui canvas integration tests only; Xvfb has no touch digitizer"}
    (P/"native-result.json").write_text(json.dumps(result,indent=2))
    (P/"run-passed.txt").write_text("all native input cancellation and commit assertions passed\n")
finally:
    for proc in [app,xvfb]:
        if proc is not None and proc.poll() is None:
            proc.terminate()
            try:proc.wait(timeout=5)
            except subprocess.TimeoutExpired:proc.kill();proc.wait()
