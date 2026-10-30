#!/usr/bin/env python3
"""Native inspector commit and restored-draft regression. Only owned PID and temporary fixture/state.
No physical printing. Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,subprocess,tempfile,time,urllib.request
from smoke_focus import await_focus
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/draft-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-startup-') as temporary:
 state=pathlib.Path(temporary);project=state/'fixture.orch'
 def node(id,name,parent,x,y,width=120,height=80,**extra):
  n={'id':id,'name':name,'kind':'Processor','parentId':parent,'children':[],'layout':{'x':x,'y':y,'width':width,'height':height,'isExpanded':True}};n.update(extra);return n
 nodes=[node('root','Canvas interactions',None,0,0,children=['a','b','c','group','obstacle','wire']),node('a','A','root',100,100),node('b','B','root',360,100),node('c','C','root',620,100),node('group','Group','root',50,350,350,200,kind='Group',children=['child']),node('child','Child','group',100,400),node('obstacle','Obstacle','root',520,230,80,100),node('wire','wire','root',0,0,kind='Link',link={'sourceNodeId':'a','targetNodeId':'c','sourcePortName':'out','targetPortName':'in'})]
 project.write_text(json.dumps({'id':'canvas-interactions-owned','name':'Canvas interactions','rootNodeId':'root','nodes':nodes}))
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
  raise TimeoutError('canvas condition timed out; state='+json.dumps(ui()))
 def content_inset():
  if window is None:return (0,0)
  text=subprocess.check_output(['xprop','-id',str(window),'_GTK_FRAME_EXTENTS'],env=env,text=True)
  if '=' not in text:return (0,0)
  values=[int(v.strip()) for v in text.split('=',1)[1].split(',')]
  return (values[0],values[2])
 def xd(*args):
  if args and args[0]=='mousemove' and len(args)>4 and args[1]=='--window':
   dx,dy=content_inset();args=(*args[:3],int(args[3])+dx,int(args[4])+dy,*args[5:])
  await_focus(window,env)
  if args[0]=='key':args=(args[0],'--clearmodifiers',*args[1:])
  return subprocess.run(['xdotool',*map(str,args)],env=env,check=True,capture_output=True,text=True).stdout.strip()
 def click(x,y):xd('mousemove','--window',window,x,y,'click',1);time.sleep(.3)
 def shot(name):
  path=ARTIFACT/(name+'.png');dx,dy=content_inset()
  # Some X11 captures contain backgrounds without text; wait for visible menu labels.
  for attempt in range(12):
   time.sleep(.2);subprocess.run(['import','-window',window,str(path)],env=env,check=True)
   deviation=float(subprocess.check_output(['convert',str(path),'-crop',f'400x18+{20+dx}+{7+dy}','-colorspace','gray','-format','%[fx:standard_deviation]','info:'],text=True))
   if deviation>.015:return path
   xd('mousemove','--window',window,900+attempt%2,300)
  raise RuntimeError('Owned window capture never displayed menu text')
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
  endpoint=wait(discover);wait(lambda:api('session.snapshot')['document']['name']=='Canvas interactions');time.sleep(.5)
  # Client shadows can be included or excluded from the WM's initial bounds.
  # Set a deterministic content size before using content-relative coordinates.
  dx,dy=content_inset();xd('windowsize','--sync',window,1400+2*dx,900+2*dy);time.sleep(.4)
 def selected():return set(ui()['selected_ids'])
 def stop():
  global app,window
  if app is not None and app.poll() is None:
   xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  app=None;window=None
 try:
  (state/'ui.json').write_text(json.dumps({'document_id':'canvas-interactions-owned','selected':'wire','selected_ids':['wire'],'parent':'root','tab':'Editor','sidebar_width':250,'inspector_width':360}))
  launch();time.sleep(1.5)
  click(1250,230);xd('key','ctrl+a');xd('type','--clearmodifiers','--delay',40,'renamed_wire');wait(lambda:(ui().get('draft') or {}).get('name')=='renamed_wire');xd('key','Return')
  wait(lambda:document()['nodes']['wire']['name']=='renamed_wire');wait(lambda:ui().get('draft') is None)
  click(1250,230);xd('key','ctrl+a');xd('type','--clearmodifiers','--delay',40,'renamed_on_blur');wait(lambda:(ui().get('draft') or {}).get('name')=='renamed_on_blur');click(600,380)
  wait(lambda:document()['nodes']['wire']['name']=='renamed_on_blur');wait(lambda:ui().get('draft') is None)
  api('file.save');time.sleep(.5)
  click(600,350);xd('type','--clearmodifiers','draft preserved across restart');wait(lambda:ui().get('draft') is not None)
  recovered=ui()['draft'];click(65,102);wait(lambda:ui()['tab']=='Designer');shot('designer-draft-controls');stop()
  launch();wait(lambda:ui().get('draft') is not None);assert ui()['draft']['text']=='draft preserved across restart'
  # The fixed footer is available without returning to Editor.
  click(1070,830);wait(lambda:document()['nodes']['wire']['text']['source']=='draft preserved across restart');wait(lambda:ui().get('draft') is None)
  click(95,298);wait(lambda:selected()=={'b'});stop()
  # Simulate an old session's name-only draft, matching the user's evidence.
  recovered.update({'name':'legacy_recovered_name','text':recovered['base_text'],'properties':recovered['base_properties']})
  session=ui();session.update({'tab':'Designer','selected':'wire','selected_ids':['wire'],'draft':recovered});(state/'ui.json').write_text(json.dumps(session))
  launch();wait(lambda:ui().get('draft') is not None);shot('recovered-name-controls')
  click(1215,830);wait(lambda:ui().get('draft') is None);click(95,270);wait(lambda:selected()=={'a'});assert document()['nodes']['wire']['name']=='renamed_on_blur'
  shot('selection-unlocked');stop()
  print(json.dumps({'nameEnter':'passed','nameBlur':'passed','restoredBodyApplyFromDesigner':'passed','recoveredNameDiscardFromDesigner':'passed','selection':'passed','artifacts':str(ARTIFACT)}))
 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
