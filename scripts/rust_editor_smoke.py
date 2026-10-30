#!/usr/bin/env python3
"""Rust body editor regression: tree-sitter syntax, rust-analyzer colors, Ctrl-Space completion, the
background cargo check of the generated program, Go to definition (F12, Ctrl-click) and Find references. Only owned PID and a temporary copy of the Rust WebServer.
Requires X11/XWayland with Vulkan, xdotool, ImageMagick, rust-analyzer and cargo.
"""
import json,os,pathlib,shutil,subprocess,tempfile,time,urllib.request
from smoke_focus import await_focus
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/rust-editor-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-rust-editor-') as temporary:
 state=pathlib.Path(temporary);project=state/'webserver-rust.orch'
 shutil.copy(ROOT/'examples/webserver-rust.orch',project)
 reference=json.loads(project.read_text());node=next(n['id'] for n in reference['nodes'] if n['name']=='router')
 original=next(n['text']['source'] for n in reference['nodes'] if n['id']==node)
 named=lambda name:next(n['id'] for n in reference['nodes'] if n['name']==name)
 env=os.environ.copy();env.pop('WAYLAND_DISPLAY',None)
 log=open(ARTIFACT/'interaction.log','w');app=None;window=None
 def wait(check,timeout=30):
  until=time.monotonic()+timeout
  while time.monotonic()<until:
   if app.poll() is not None:raise RuntimeError('owned desktop exited; see interaction.log')
   try:
    result=check()
    if result:return result
   except (subprocess.CalledProcessError,FileNotFoundError,json.JSONDecodeError,KeyError,IndexError):pass
   time.sleep(.12)
  raise TimeoutError('rust editor condition timed out')
 def xd(*args):
  await_focus(window,env)
  if args[0]=='key':args=(args[0],'--clearmodifiers',*args[1:])
  return subprocess.run(['xdotool',*map(str,args)],env=env,check=True,capture_output=True,text=True).stdout.strip()
 def click(x,y):xd('mousemove','--window',window,x,y,'click',1);time.sleep(.3)
 def inset():
  text=subprocess.check_output(['xprop','-id',str(window),'_GTK_FRAME_EXTENTS'],env=env,text=True)
  if '=' not in text:return (0,0)
  values=[int(v.strip()) for v in text.split('=',1)[1].split(',')];return (values[0],values[2])
 def at(x,y):
  # Screenshots include the client-side frame; pointer coordinates do not.
  dx,dy=inset();return (x-dx,y-dy)
 def shot(name):
  path=ARTIFACT/(name+'.png');time.sleep(.3);subprocess.run(['import','-window',window,str(path)],env=env,check=True);return path
 def ui():return json.loads((state/'ui.json').read_text())
 def draft():return (ui().get('draft') or {}).get('text','')
 def command(text):
  xd('key','ctrl+shift+p');time.sleep(.4);xd('type','--clearmodifiers','--delay',2,text);time.sleep(.25);xd('key','Return');time.sleep(.6)
 def api(command,arguments=None):
  request=urllib.request.Request(endpoint+'/api',json.dumps({'command':command,'arguments':arguments or {}}).encode(),{'Content-Type':'application/json','Authorization':f'Bearer {token}'})
  with urllib.request.urlopen(request,timeout=60) as response:
   value=json.load(response);assert value['ok'],value.get('error');return value['result']
 def document():return api('session.snapshot')['document']
 def cmdline(pid):return pathlib.Path(f'/proc/{pid}/cmdline').read_bytes().decode().split('\0')
 try:
  (state/'ui.json').write_text(json.dumps({'document_id':reference['id'],'parent':'root','selected':node,'selected_ids':[node],'section':'source','tab':'Editor'}))
  app=subprocess.Popen([BINARY,str(project),'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait(lambda:subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(app.pid)],env=env,text=True).splitlines()[0])
  backend_pid=wait(lambda:int(subprocess.check_output(['pgrep','-P',str(app.pid)],text=True).splitlines()[0]))
  args=cmdline(backend_pid);assert '--backend' in args;token=args[args.index('--token')+1]
  def discover():
   sockets={p.readlink().name for p in pathlib.Path(f'/proc/{backend_pid}/fd').iterdir() if str(p.readlink()).startswith('socket:')}
   for line in pathlib.Path(f'/proc/{backend_pid}/net/tcp').read_text().splitlines()[1:]:
    fields=line.split()
    if fields[3]=='0A' and f'socket:[{fields[9]}]' in sockets:return f'http://127.0.0.1:{int(fields[1].split(":")[1],16)}'
  endpoint=wait(discover);wait(lambda:document()['name']=='WebServerRust')
  shot('rust-syntax')
  # The desktop's own coloring request starts rust-analyzer; the editor session reports its pid.
  analyzer_pid=api('editor.complete',{'nodeId':node,'section':'source','text':'pop','offset':3})['pid']
  assert any('rust-analyzer' in part for part in cmdline(analyzer_pid)),cmdline(analyzer_pid)
  time.sleep(3);shot('rust-semantic-colors')
  click(600,350);xd('key','ctrl+End');xd('key','Return');xd('type','--clearmodifiers','threadwork_runner.is_r')
  wait(lambda:draft().endswith('threadwork_runner.is_r'))
  xd('key','ctrl+space');time.sleep(3);shot('rust-completions')
  xd('key','Return');wait(lambda:draft().endswith('threadwork_runner.is_running'))
  # Applying a method used as a value is a rustc error, reported by the background cargo check.
  xd('key','ctrl+Return');wait(lambda:document()['nodes'][node]['text']['source'].endswith('threadwork_runner.is_running'))
  wait(lambda:ui().get('draft') is None)
  time.sleep(15);shot('rust-cargo-check-issue')
  command('Undo');wait(lambda:document()['nodes'][node]['text']['source']==original)
  time.sleep(10);shot('rust-cargo-check-cleared')
  def source(name):return next(n['text']['source'] for n in reference['nodes'] if n['name']==name)
  def cursor_on(name,needle,occurrence=-1):
   # Character steps from the start are independent of soft wrapping and scrolling.
   text=source(name);index=[i for i in range(len(text)) if text.startswith(needle,i)][occurrence]+1
   # From the nearer end, so that few key events are sent.
   click(600,350)
   key,steps=('Right',index) if index<len(text)-index else ('Left',len(text)-index)
   xd('key','ctrl+Home' if key=='Right' else 'ctrl+End')
   # Short runs, so that the focus is checked again before each one.
   for run in range(0,steps,10):xd('key','--delay',30,'--repeat',min(10,steps-run),key)
   time.sleep(.3)
  # F12 on `item` in `push(route, item);` jumps to its `let`; Shift-F12 there lists its uses.
  cursor_on('router','item');xd('key','F12');time.sleep(2)
  assert ui().get('draft') is None and ui()['selected']==node
  xd('key','shift+F12');time.sleep(3);shot('rust-references-local');xd('key','Escape')
  # F12 on a library function called through the lent instance opens the library body.
  click(*at(127,778));wait(lambda:ui()['selected']==named('socket_respond'));time.sleep(1)
  cursor_on('socket_respond','http_render_header');xd('key','F12')
  wait(lambda:ui()['selected']==named('lib_http_server'));time.sleep(1)
  # Shift-F12 at the definition lists the call in socket_respond's body.
  def clean(name):
   text=draft() if ui().get('draft') else source(name);original=source(name)
   at=next((i for i,(a,b) in enumerate(zip(text,original)) if a!=b),min(len(text),len(original)))
   assert ui().get('draft') is None,(name,at,repr(original[at-20:at+20]),repr(text[at-20:at+20]))
  clean('lib_http_server')
  xd('key','shift+F12');time.sleep(3);shot('rust-references-library');clean('lib_http_server');xd('key','Escape');time.sleep(.5);clean('lib_http_server')
  # Ctrl-click on the modeled type in `pop::<ReqRep>` reveals the Type. Pointer coordinates:
  # after ctrl+Home, line 1 of the default 1424x924 window is at y 326, 20 px per line.
  click(*at(103,722));wait(lambda:ui()['selected']==node);time.sleep(1)
  click(600,350);xd('key','ctrl+Home');time.sleep(.5)
  xd('keydown','ctrl');xd('mousemove','--window',window,537,346,'click',1);xd('keyup','ctrl')
  wait(lambda:ui()['selected']==named('ReqRep'));shot('rust-definition-type')
  generated=[int(p) for p in subprocess.check_output(['pgrep','-P',str(backend_pid)],text=True).split()]
  assert sum(any('rust-analyzer' in part for part in cmdline(pid)) for pid in generated)>=2,'editor and graph analyzers'
  width=int(subprocess.check_output(['xdotool','getwindowgeometry','--shell',window],env=env,text=True).split('WIDTH=')[1].splitlines()[0])
  click(width-17,28);app.wait(timeout=15);assert app.returncode==0
  until=time.monotonic()+5
  while any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,*generated]) and time.monotonic()<until:time.sleep(.05)
  assert not any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,*generated])
  print(json.dumps({'rustEditor':'passed','rustAnalyzer':True,'completionInsertion':True,'cargoCheckRound':True,'definitions':True,'ownedChildrenStopped':True,'artifacts':str(ARTIFACT)}))
 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
