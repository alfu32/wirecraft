#!/usr/bin/env python3
"""Requires X11/XWayland with Vulkan/DRI3. Uses only temporary state and an owned PID."""
import os, pathlib, subprocess, time, tempfile, json, urllib.request
from smoke_focus import await_focus
root=pathlib.Path(__file__).resolve().parent.parent
binary=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(root/'target/debug/wirecraft-desktop'))
artifact=root/'target'/'desktop-smoke'
artifact.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-desktop-smoke-') as tmp:
 state=pathlib.Path(tmp)
 env=os.environ.copy();env.pop('WAYLAND_DISPLAY',None);env['XDG_SESSION_TYPE']='x11'
 log=open(artifact/'desktop.log','w')
 app=subprocess.Popen([binary,'--state-dir',str(state)],env=env,stdout=log,stderr=log)
 def content_inset():
  if window is None:return (0,0)
  text=subprocess.check_output(['xprop','-id',str(window),'_GTK_FRAME_EXTENTS'],env=env,text=True)
  if '=' not in text:return (0,0)
  values=[int(v.strip()) for v in text.split('=',1)[1].split(',')]
  return (values[0],values[2])
 def xd(*args):
  if args and args[0]=='mousemove' and len(args)>4 and args[1]=='--window':
   dx,dy=content_inset();args=(*args[:3],int(args[3])+dx,int(args[4])+dy,*args[5:])
  if args and args[0] in ('key','type','mousemove','click','mousedown','mouseup') and window is not None:
   await_focus(window,env)
  if args and args[0]=='key' and window is not None:args=(args[0],'--clearmodifiers',*args[1:])
  result=subprocess.run(['xdotool',*map(str,args)],env=env,text=True,capture_output=True)
  if result.returncode and args[-1]!='ctrl+q':raise subprocess.CalledProcessError(result.returncode,result.args,result.stdout,result.stderr)
  return result.stdout.strip()
 def caption():
  dx,dy=content_inset();return subprocess.check_output(['import','-window',window,'-crop',f'350x32+{850+dx}+{132+dy}','rgba:-'],env=env)
 def choose_mode(x,y):
  before=caption();until=time.monotonic()+8
  while time.monotonic()<until:
   xd('windowraise',window);xd('mousemove','--window',window,x,y);xd('click',1)
   time.sleep(.25)
   if caption()!=before:return
  raise TimeoutError('tool mode did not repaint in the owned window')
 def wait_for(f,timeout=12):
  until=time.monotonic()+timeout
  while time.monotonic()<until:
   if app.poll() is not None:raise RuntimeError('desktop exited; see target/desktop-smoke/desktop.log')
   try:
    value=f()
    if value:return value
   except (subprocess.CalledProcessError,FileNotFoundError,json.JSONDecodeError,KeyError):pass
   time.sleep(.1)
  raise TimeoutError('desktop smoke condition timed out')
 window=None
 try:
  window=wait_for(lambda:xd('search','--onlyvisible','--pid',app.pid).splitlines()[0])
  assert int(xd('getwindowpid',window))==app.pid
  time.sleep(.5)
  await_focus(window,env)
  time.sleep(1)
  subprocess.run(['import','-window',window,str(artifact/'initial.png')],env=env,check=True)
  # Node mode button then two canvas positions; never addresses Threadworks' display.
  choose_mode(395,148)
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
  # Synthetic clicks are paced across frames; completion is checked in persisted state.
  wait_for(lambda:json.loads(state.joinpath('ui.json').read_text()).get('draft') is None)
  choose_mode(570,148)
  xd('mousemove','--window',window,530,350);xd('click',1);time.sleep(.2)
  xd('mousemove','--window',window,800,470);xd('click',1)
  wait_for(lambda:sum(n['kind']=='Link' for n in recovery()['document']['nodes'].values())==1)
  xd('key','Escape');xd('mousemove','--window',window,530,350);xd('click',1)
  xd('key','ctrl+c');time.sleep(.2);xd('key','ctrl+v')
  wait_for(lambda:len(recovery()['document']['nodes'])==5)
  xd('key','ctrl+z');wait_for(lambda:len(recovery()['document']['nodes'])==4)
  xd('mousemove','--window',window,530,350);xd('click',1)
  subprocess.run(['import','-window',window,str(artifact/'canvas.png')],env=env,check=True)
  # A draft must survive process exit and recovery without being silently applied.
  xd('mousemove','--window',window,165,102);xd('click',1)
  time.sleep(.3)
  subprocess.run(['import','-window',window,str(artifact/'editor.png')],env=env,check=True)
  xd('mousemove','--window',window,450,350);xd('click',1)
  xd('type','--clearmodifiers','--delay',1,'int main(void) { return missing; }')
  wait_for(lambda:state.joinpath('ui.json').exists() and (json.loads(state.joinpath('ui.json').read_text()).get('draft') or {}).get('text')=='int main(void) { return missing; }')
  # Quit through our own registered action and check process lifecycle.
  xd('key','ctrl+q');app.wait(timeout=10)
  assert app.returncode==0, app.returncode
  assert len(recovery()['document']['nodes'])==4
  assert state.joinpath('ui.json').exists()
  # Reopen the same isolated session, apply its recovered editor draft, then use clangd.
  app=subprocess.Popen([binary,'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait_for(lambda:xd('search','--onlyvisible','--pid',app.pid).splitlines()[0])
  assert int(xd('getwindowpid',window))==app.pid
  time.sleep(.7);await_focus(window,env)
  xd('mousemove','--window',window,950,150);xd('click',1)
  wait_for(lambda:any(n['text']['source']=='int main(void) { return missing; }' for n in recovery()['document']['nodes'].values()))
  # Tool output is visually recorded; real LSP result assertions also run in cargo tests.
  xd('mousemove','--window',window,781,102);xd('click',1);time.sleep(.2)
  subprocess.run(['import','-window',window,str(artifact/'tools.png')],env=env,check=True)
  # Inspect only the backend child launched by this owned desktop.
  backend_pid=int(subprocess.check_output(['pgrep','-P',str(app.pid)],text=True).splitlines()[0])
  args=pathlib.Path(f'/proc/{backend_pid}/cmdline').read_bytes().decode().split('\0');assert '--backend' in args
  token=args[args.index('--token')+1]
  sockets={p.readlink().name for p in pathlib.Path(f'/proc/{backend_pid}/fd').iterdir() if str(p.readlink()).startswith('socket:')}
  port=None
  for line in pathlib.Path(f'/proc/{backend_pid}/net/tcp').read_text().splitlines()[1:]:
   fields=line.split()
   if fields[3]=='0A' and f'socket:[{fields[9]}]' in sockets:port=int(fields[1].split(':')[1],16)
  assert port is not None
  def api(command):
   request=urllib.request.Request(f'http://127.0.0.1:{port}/api',json.dumps({'command':command,'arguments':{}}).encode(),{'Content-Type':'application/json','Authorization':f'Bearer {token}'})
   with urllib.request.urlopen(request,timeout=5) as response:return json.load(response)['result']
  xd('mousemove','--window',window,300,199);xd('click',1)
  diagnostics=wait_for(lambda:next((d for d in api('lsp.diagnostics').values() if any('missing' in e.get('message','') for e in d.get('diagnostics',[]))),None))
  lsp_pid=api('lsp.status')['pid'];assert lsp_pid
  xd('mousemove','--window',window,530,199);xd('click',1)
  wait_for(lambda:any(j['status']=='failed' and 'missing' in j.get('stderr','') for j in api('jobs.list')['jobs']))
  xd('mousemove','--window',window,930,199);xd('click',1)
  mapped=wait_for(lambda:next((d for d in api('editor.generated_diagnostics')['diagnostics'] if d.get('source') and d['source']['section']=='source'),None))
  generated_pid=api('editor.generated_diagnostics')['pid'];assert generated_pid
  time.sleep(.3)
  xd('mousemove','--window',window,550,252);xd('click',1)
  wait_for(lambda:json.loads(state.joinpath('ui.json').read_text()).get('tab')=='Editor')
  subprocess.run(['import','-window',window,str(artifact/'tools.png')],env=env,check=True)
  xd('key','ctrl+q');app.wait(timeout=10)
  until=time.monotonic()+5
  while any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,lsp_pid,generated_pid]) and time.monotonic()<until:time.sleep(.05)
  assert not any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,lsp_pid,generated_pid]), 'owned child survived shutdown'
  print(json.dumps({'desktop':'passed','generatedDiagnosticRevealed':True,'generatedDiagnosticsMapped':True,'clangdDiagnostics':True,'externalCompilerDiagnostics':True,'ownedChildrenStopped':True,'editorDraft':True,'draftRestoredAndApplied':True,'nodes':4,'canvasLink':True,'clipboardUndo':True,'recovery':True,'exitCode':app.returncode,'artifacts':str(artifact)}))
 finally:
  if app.poll() is None:
   if window is not None:
    subprocess.run(['import','-window',window,str(artifact/'failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
