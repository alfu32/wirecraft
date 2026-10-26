#!/usr/bin/env python3
"""Compiler-context editor and compact inspector regression. Only owned PID and temporary fixture/state.
No physical printing. Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,shutil,subprocess,tempfile,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/editor-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-walkthrough-') as temporary:
 state=pathlib.Path(temporary);project=state/'fixture.orch'
 shutil.copy(ROOT/'crates/wirecraft-runtime/tests/fixtures/walkthrough.orch',project)
 env=os.environ.copy();env.pop('WAYLAND_DISPLAY',None)
 log=open(ARTIFACT/'interaction.log','w');app=None;window=None
 def wait(check,timeout=15):
  until=time.monotonic()+timeout
  while time.monotonic()<until:
   if app.poll() is not None:raise RuntimeError('owned desktop exited; see interaction.log')
   try:
    result=check()
    if result:return result
   except (subprocess.CalledProcessError,FileNotFoundError,json.JSONDecodeError,KeyError,IndexError):pass
   time.sleep(.12)
  raise TimeoutError('walkthrough condition timed out')
 def xd(*args):
  if int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True))!=int(window):
   subprocess.run(['xdotool','windowactivate','--sync',window],env=env,check=True,capture_output=True)
  assert int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True))==int(window)
  if args[0]=='key':args=(args[0],'--clearmodifiers',*args[1:])
  return subprocess.run(['xdotool',*map(str,args)],env=env,check=True,capture_output=True,text=True).stdout.strip()
 def click(x,y):xd('mousemove','--window',window,x,y,'click',1);time.sleep(.3)
 def shot(name):
  time.sleep(.35);path=ARTIFACT/(name+'.png');subprocess.run(['import','-window',window,str(path)],env=env,check=True);return path
 def ui():return json.loads((state/'ui.json').read_text())
 def document():return api('session.snapshot')['document']
 def command(text):
  xd('key','ctrl+shift+p');time.sleep(.4);xd('type','--clearmodifiers','--delay',2,text);time.sleep(.25);xd('key','Return');time.sleep(.6)
 def api(command,arguments=None):
  request=urllib.request.Request(endpoint+'/api',json.dumps({'command':command,'arguments':arguments or {}}).encode(),{'Content-Type':'application/json','Authorization':f'Bearer {token}'})
  with urllib.request.urlopen(request,timeout=10) as response:
   value=json.load(response);assert value['ok'],value.get('error');return value['result']
 def launch():
  global app,window,endpoint,token,backend_pid
  app=subprocess.Popen([BINARY,str(project),'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait(lambda:subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(app.pid)],env=env,text=True).splitlines()[0]);assert int(subprocess.check_output(['xdotool','getwindowpid',window],env=env,text=True))==app.pid
  backend_pid=wait(lambda:int(subprocess.check_output(['pgrep','-P',str(app.pid)],text=True).splitlines()[0]))
  args=pathlib.Path(f'/proc/{backend_pid}/cmdline').read_bytes().decode().split('\0');assert '--backend' in args;token=args[args.index('--token')+1]
  def discover():
   sockets={p.readlink().name for p in pathlib.Path(f'/proc/{backend_pid}/fd').iterdir() if str(p.readlink()).startswith('socket:')}
   for line in pathlib.Path(f'/proc/{backend_pid}/net/tcp').read_text().splitlines()[1:]:
    fields=line.split()
    if fields[3]=='0A' and f'socket:[{fields[9]}]' in sockets:return f'http://127.0.0.1:{int(fields[1].split(":")[1],16)}'
  endpoint=wait(discover);wait(lambda:api('session.snapshot')['document']['name']=='ticker');time.sleep(.5)
 try:
  shutil.copy(ROOT/'docs/migration/threadworks/example-c.orch',project)
  reference=json.loads(project.read_text());node=next(n['id'] for n in reference['nodes'] if n['name']=='write_append_file')
  (state/'ui.json').write_text(json.dumps({'document_id':reference['id'],'parent':'root','selected':node,'selected_ids':[node],'section':'source','tab':'Editor'}))
  launch();time.sleep(1);shot('025-wirecraft-context-before')
  click(600,350);xd('key','ctrl+End');xd('key','Return');xd('type','--clearmodifiers','argslib__get_ar')
  wait(lambda:(ui().get('draft') or {}).get('text','').endswith('argslib__get_ar'))
  xd('key','ctrl+space');time.sleep(2);shot('025-wirecraft-context-completions')
  xd('key','Return');wait(lambda:(ui().get('draft') or {}).get('text','').endswith('argslib__get_arg'))
  assert document()['nodes'][node]['text']['source']==next(n['text']['source'] for n in reference['nodes'] if n['id']==node)
  xd('key','ctrl+z');wait(lambda:(ui().get('draft') or {}).get('text','').endswith('argslib__get_ar'))
  xd('key','ctrl+space');time.sleep(1);xd('type','--clearmodifiers','x');time.sleep(.5);shot('025-completion-stale-dismissed')
  assert (ui()['draft']['text']).endswith('argslib__get_arx')
  click(620,253);wait(lambda:ui().get('draft') is None)
  click(1270,598);time.sleep(.3);shot('compact-status-dropdown');click(1270,660)
  wait(lambda:(ui().get('draft') or {}).get('properties') and json.loads(ui()['draft']['properties']).get('status')=='BUSINESS')
  click(950,165);wait(lambda:document()['nodes'][node].get('status')!='FUNCTIONAL');wait(lambda:ui().get('draft') is None)
  packet=next(n['id'] for n in reference['nodes'] if n['name']=='Packet');click(90,442);wait(lambda:ui()['selected']==packet)
  xd('mousemove','--window',window,1320,740);xd('click','--repeat',35,'--delay',20,5);time.sleep(2);shot('compact-inline-tables');xd('mousemove','--window',window,1100,500);time.sleep(1);shot('compact-inline-tables-settled')
  clangd_pid=api('editor.complete',{'nodeId':node,'section':'source','text':'argslib__op','offset':len('argslib__op')})['pid']
  def width():return int(subprocess.check_output(['xdotool','getwindowgeometry','--shell',window],env=env,text=True).split('WIDTH=')[1].splitlines()[0])
  click(width()-51,28);wait(lambda:'_NET_WM_STATE_MAXIMIZED_VERT' in subprocess.check_output(['xprop','-id',window,'_NET_WM_STATE'],env=env,text=True));time.sleep(.3)
  click(width()-51,16);wait(lambda:'_NET_WM_STATE_MAXIMIZED_VERT' not in subprocess.check_output(['xprop','-id',window,'_NET_WM_STATE'],env=env,text=True));time.sleep(.4)
  click(width()-85,28);time.sleep(.4);assert '_NET_WM_STATE_HIDDEN' in subprocess.check_output(['xprop','-id',window,'_NET_WM_STATE'],env=env,text=True)
  xd('key','Escape');time.sleep(.4);click(width()-17,28);app.wait(timeout=10);assert app.returncode==0
  until=time.monotonic()+5
  while any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,clangd_pid]) and time.monotonic()<until:time.sleep(.05)
  assert not any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,clangd_pid])
  print(json.dumps({'editor':'passed','compilerContextInsertion':True,'completionUndo':True,'staleMenuDismissal':True,'statusDropdownAndApply':True,'inlineTablesVisible':True,'minimizeMaximizeRestoreClose':True,'ownedChildrenStopped':True,'artifacts':str(ARTIFACT)}))

 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
