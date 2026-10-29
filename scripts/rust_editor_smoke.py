#!/usr/bin/env python3
"""Rust body editor regression: tree-sitter syntax, rust-analyzer colors, Ctrl-Space completion and the
background cargo check of the generated program. Only owned PID and a temporary copy of the Rust WebServer.
Requires X11/XWayland with Vulkan, xdotool, ImageMagick, rust-analyzer and cargo.
"""
import json,os,pathlib,shutil,subprocess,tempfile,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/rust-editor-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-rust-editor-') as temporary:
 state=pathlib.Path(temporary);project=state/'webserver-rust.orch'
 shutil.copy(ROOT/'examples/webserver-rust.orch',project)
 reference=json.loads(project.read_text());node=next(n['id'] for n in reference['nodes'] if n['name']=='router')
 original=next(n['text']['source'] for n in reference['nodes'] if n['id']==node)
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
  if int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True))!=int(window):
   subprocess.run(['xdotool','windowactivate','--sync',window],env=env,check=True,capture_output=True)
  if args[0]=='key':args=(args[0],'--clearmodifiers',*args[1:])
  return subprocess.run(['xdotool',*map(str,args)],env=env,check=True,capture_output=True,text=True).stdout.strip()
 def click(x,y):xd('mousemove','--window',window,x,y,'click',1);time.sleep(.3)
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
  generated=[int(p) for p in subprocess.check_output(['pgrep','-P',str(backend_pid)],text=True).split()]
  assert sum(any('rust-analyzer' in part for part in cmdline(pid)) for pid in generated)>=2,'editor and graph analyzers'
  width=int(subprocess.check_output(['xdotool','getwindowgeometry','--shell',window],env=env,text=True).split('WIDTH=')[1].splitlines()[0])
  click(width-17,28);app.wait(timeout=15);assert app.returncode==0
  until=time.monotonic()+5
  while any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,*generated]) and time.monotonic()<until:time.sleep(.05)
  assert not any(pathlib.Path(f'/proc/{pid}').exists() for pid in [backend_pid,*generated])
  print(json.dumps({'rustEditor':'passed','rustAnalyzer':True,'completionInsertion':True,'cargoCheckRound':True,'ownedChildrenStopped':True,'artifacts':str(ARTIFACT)}))
 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
