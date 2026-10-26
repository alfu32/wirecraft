#!/usr/bin/env python3
"""Run under xvfb-run. Uses only an isolated state directory and owned desktop."""
import os, pathlib, subprocess, time, tempfile, json
root=pathlib.Path(__file__).resolve().parent.parent
artifact=root/'target'/'desktop-smoke'
artifact.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-desktop-smoke-') as tmp:
 state=pathlib.Path(tmp)
 env=os.environ.copy();env.pop('WAYLAND_DISPLAY',None);env['XDG_SESSION_TYPE']='x11'
 log=open(artifact/'desktop.log','w')
 app=subprocess.Popen([str(root/'target/debug/wirecraft-desktop'),'--state-dir',str(state)],env=env,stdout=log,stderr=log)
 def xd(*args):return subprocess.check_output(['xdotool',*map(str,args)],env=env,text=True).strip()
 def wait_for(f,timeout=12):
  until=time.monotonic()+timeout
  while time.monotonic()<until:
   if app.poll() is not None:raise RuntimeError('desktop exited; see target/desktop-smoke/desktop.log')
   try:
    value=f()
    if value:return value
   except (subprocess.CalledProcessError,FileNotFoundError,json.JSONDecodeError):pass
   time.sleep(.1)
  raise TimeoutError('desktop smoke condition timed out')
 window=None
 try:
  window=wait_for(lambda:xd('search','--onlyvisible','--pid',app.pid).splitlines()[0])
  assert int(xd('getwindowpid',window))==app.pid
  time.sleep(.5)
  xd('windowfocus','--sync',window)
  time.sleep(1)
  subprocess.run(['import','-window',window,str(artifact/'initial.png')],env=env,check=True)
  # Node mode button then two canvas positions; never addresses Threadworks' display.
  xd('mousemove','--window',window,370,114);xd('click',1)
  xd('mousemove','--window',window,450,300);xd('click',1)
  wait_for(lambda:state.joinpath('recovery.json').exists())
  recovery=lambda:json.loads(state.joinpath('recovery.json').read_text())
  wait_for(lambda:len(recovery()['document']['nodes'])==2)
  xd('mousemove','--window',window,760,450);xd('click',1)
  wait_for(lambda:len(recovery()['document']['nodes'])==3)
  # Escape returns to Select; zoom and drag are exercised independently.
  xd('key','Escape');xd('mousemove','--window',window,480,320);xd('mousedown',1);xd('mousemove','--window',window,530,350);xd('mouseup',1)
  time.sleep(.4)
  subprocess.run(['import','-window',window,str(artifact/'canvas.png')],env=env,check=True)
  # A draft must survive process exit and recovery without being silently applied.
  xd('mousemove','--window',window,132,70);xd('click',1)
  time.sleep(.3)
  subprocess.run(['import','-window',window,str(artifact/'editor.png')],env=env,check=True)
  xd('mousemove','--window',window,450,280);xd('click',1)
  xd('type','--clearmodifiers','--delay',1,'int main(void) { return missing; }')
  wait_for(lambda:state.joinpath('ui.json').exists() and (json.loads(state.joinpath('ui.json').read_text()).get('draft') or {}).get('text')=='int main(void) { return missing; }')
  # Quit through our own registered action and check process lifecycle.
  xd('key','ctrl+q');app.wait(timeout=10)
  assert app.returncode==0, app.returncode
  assert len(recovery()['document']['nodes'])==3
  assert state.joinpath('ui.json').exists()
  # Reopen the same isolated session, apply its recovered editor draft, then use clangd.
  app=subprocess.Popen([str(root/'target/debug/wirecraft-desktop'),'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait_for(lambda:xd('search','--onlyvisible','--pid',app.pid).splitlines()[0])
  assert int(xd('getwindowpid',window))==app.pid
  time.sleep(.7);xd('windowfocus','--sync',window)
  xd('mousemove','--window',window,1320,116);xd('click',1)
  wait_for(lambda:any(n['text']['source']=='int main(void) { return missing; }' for n in recovery()['document']['nodes'].values()))
  # Tool output is visually recorded; real LSP result assertions also run in cargo tests.
  xd('mousemove','--window',window,445,70);xd('click',1);time.sleep(.2)
  subprocess.run(['import','-window',window,str(artifact/'tools.png')],env=env,check=True)
  xd('key','ctrl+q');app.wait(timeout=10)
  print(json.dumps({'desktop':'passed','editorDraft':True,'draftRestoredAndApplied':True,'nodes':3,'recovery':True,'exitCode':app.returncode,'artifacts':str(artifact)}))
 finally:
  if app.poll() is None:
   try:
    if window is None:raise RuntimeError('No owned window')
    xd('key','--window',window,'ctrl+q');app.wait(timeout=8)
   except Exception:app.terminate();app.wait(timeout=5)
  log.close()
