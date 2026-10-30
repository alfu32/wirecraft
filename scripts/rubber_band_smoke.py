#!/usr/bin/env python3
"""Native smoke: placement ghosts and routed previews of links under construction.
Only owned PID and temporary copies of examples/webserver*.orch.
Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,shutil,subprocess,tempfile,time,urllib.request
from smoke_focus import await_focus
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/rubber-band-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-components-') as temporary:
 state=pathlib.Path(temporary)
 for name in ['webserver-component.orch']:shutil.copy(ROOT/'examples'/name,state/name)
 project=state/'webserver-component.orch'
 reference=json.loads(project.read_text())
 rendered=next(n['id'] for n in reference['nodes'] if n.get('link') and n['name']=='rendered')
 (state/'ui.json').write_text(json.dumps({'document_id':reference['id'],'parent':'root','tab':'Designer','selected':None,'selected_ids':[rendered],'sidebar_width':400,'inspector_width':450,'export_settings':{'sheet':{'format':'A4','scale':1,'split':True,'overlapMm':5}},'viewport':{'x':40,'y':40,'zoom':1}}))
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
  raise TimeoutError('condition timed out')
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
 def click(x,y):xd('mousemove','--window',window,x,y);time.sleep(.15);xd('mousedown',1);time.sleep(.08);xd('mouseup',1);time.sleep(.5)
 def shot(name):
  path=ARTIFACT/(name+'.png');dx,dy=content_inset()
  # Some X11 captures contain backgrounds without text; wait for visible menu labels.
  for attempt in range(12):
   time.sleep(.3);subprocess.run(['import','-window',window,str(path)],env=env,check=True)
   deviation=float(subprocess.check_output(['convert',str(path),'-crop',f'400x18+{20+dx}+{7+dy}','-colorspace','gray','-format','%[fx:standard_deviation]','info:'],text=True))
   if deviation>.015:return path
   # Keep the pointer where the step put it: the band under test follows it.
   xd('mousemove_relative','--',1 if attempt%2==0 else -1,0)
  raise RuntimeError('Owned window capture never displayed menu text')
 def ui():return json.loads((state/'ui.json').read_text())
 def command(text):
  xd('key','ctrl+shift+p');time.sleep(.4);xd('type','--clearmodifiers','--delay',2,text);time.sleep(.25);xd('key','Return');time.sleep(.6)
 def api(command,arguments=None):
  request=urllib.request.Request(endpoint+'/api',json.dumps({'command':command,'arguments':arguments or {}}).encode(),{'Content-Type':'application/json','Authorization':f'Bearer {token}'})
  with urllib.request.urlopen(request,timeout=10) as response:
   value=json.load(response);assert value['ok'],value.get('error');return value['result']
 def document():return api('session.snapshot')['document']
 def launch():
  global app,window,endpoint,token
  window=None
  app=subprocess.Popen([BINARY,str(project),'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait(lambda:subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(app.pid)],env=env,text=True).splitlines()[0])
  backend=wait(lambda:int(subprocess.check_output(['pgrep','-P',str(app.pid)],text=True).splitlines()[0]))
  args=pathlib.Path(f'/proc/{backend}/cmdline').read_bytes().decode().split('\0');token=args[args.index('--token')+1]
  def discover():
   sockets={p.readlink().name for p in pathlib.Path(f'/proc/{backend}/fd').iterdir() if str(p.readlink()).startswith('socket:')}
   for line in pathlib.Path(f'/proc/{backend}/net/tcp').read_text().splitlines()[1:]:
    fields=line.split()
    if fields[3]=='0A' and f'socket:[{fields[9]}]' in sockets:return f'http://127.0.0.1:{int(fields[1].split(":")[1],16)}'
  endpoint=wait(discover);wait(lambda:document()['name']==reference['name'])
  dx,dy=content_inset();xd('windowsize','--sync',window,1400+2*dx,900+2*dy);time.sleep(.6)
 def point(x,y):
  v=ui()['viewport'];return (round(ui()['sidebar_width']+5+v['x']+x*v['zoom']),round(172+v['y']+y*v['zoom']))
 def hover(x,y):
  xd('mousemove','--window',window,x-30,y-20);time.sleep(.2)
  xd('mousemove','--window',window,x,y);time.sleep(.8);xd('mousemove','--window',window,x+1,y);time.sleep(3.5)
 try:
  launch()
  click(898,161)  # toolbar Fit
  initial=set(document()['nodes'])
  by={n['name']:n for n in document()['nodes'].values()}
  respond=by['socket_respond']['layout'];accept=by['socket_accept']['layout']
  centre=lambda r:point(r['x']+r['width']/2,r['y']+r['height']/2)
  # Rewire: press the selected link's end handle, hover a new entity, drop it there.
  shot('10-link-ends')
  dx,dy=content_inset()
  blobs=subprocess.check_output(['convert',str(ARTIFACT/'10-link-ends.png'),'-crop','990x680+420+185','+repage','-fuzz','6%','-fill','white','-opaque','#70d5c6','-fill','black','+opaque','white','-morphology','Erode','Disk:2.5','-define','connected-components:verbose=true','-define','connected-components:area-threshold=4','-connected-components','8','null:'],text=True)
  ends=[]
  for line in blobs.splitlines()[1:]:
   f=line.split()
   if f[-1].startswith('gray(255') or f[-1].startswith('srgb(255') or f[-1]=='white':
    x,y=map(float,f[2].split(','));ends.append((round(x)+420-dx,round(y)+185-dy))
  print('handles',ends)
  assert len(ends)>=2,blobs
  # The handle nearer socket_respond is the `rendered` source end.
  rx,ry=centre(respond)
  source=min(ends,key=lambda e:(e[0]-rx)**2+(e[1]-ry)**2)
  click(*source);shot('11-picked-up')
  listener=by['ListenerConfig']['layout']
  hover(*centre(by['base_lib']['layout']));shot('12-rewire-preview')
  xd('key','Escape');time.sleep(.5)
  assert set(document()['nodes'])==initial,'a cancelled rewire must not change the design'
  # Node, Type and Port modes: the future contour follows the pointer.
  empty=point(respond['x']-150,respond['y']+330)
  for button,name in [((562,161),'00-node'),((652,161),'01-type'),((818,161),'02-port')]:
   click(*button);hover(*empty);shot(name)
  # Link mode: a routed preview to an entity, and to empty canvas (an output port).
  click(735,161);click(*centre(respond))
  hover(*centre(accept));shot('03-link-to-target')
  hover(*empty);shot('04-link-to-empty')
  xd('key','Escape');time.sleep(.5);shot('05-cancelled')
  assert set(document()['nodes'])==initial,'previews must not change the design'
  # Drop the picked-up end on base_lib for real: one reconnect.
  click(898,161);click(*source);click(*centre(by['base_lib']['layout']))
  link=wait(lambda:[n for n in document()['nodes'].values() if n['name']=='rendered' and n['link']['sourceNodeId']==by['base_lib']['id']])
  shot('13-rewired')
  print('OK')
 finally:
  if app and app.poll() is None:app.terminate();app.wait(10)
  log.close()
