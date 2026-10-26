#!/usr/bin/env python3
"""Native canvas/tree gesture regression. Only owned PID and temporary fixture/state.
No physical printing. Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,subprocess,tempfile,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/canvas-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-canvas-') as temporary:
 state=pathlib.Path(temporary);project=state/'fixture.orch'
 def node(id,name,parent,x,y,width=120,height=80,**extra):
  n={'id':id,'name':name,'kind':'Processor','parentId':parent,'children':[],'layout':{'x':x,'y':y,'width':width,'height':height,'isExpanded':True}};n.update(extra);return n
 nodes=[node('root','Canvas interactions',None,0,0,children=['a','b','c','group','obstacle','wire']),node('a','A','root',100,100),node('b','B','root',360,100),node('c','C','root',620,100),node('group','Group','root',50,350,350,200,kind='Group',children=['child']),node('child','Child','group',100,400),node('obstacle','Obstacle','root',520,230,80,100),node('wire','wire','root',0,0,kind='Link',link={'sourceNodeId':'a','targetNodeId':'c','sourcePortName':'out','targetPortName':'in'})]
 project.write_text(json.dumps({'id':'canvas-interactions-owned','name':'Canvas interactions','rootNodeId':'root','nodes':nodes}))
 (state/'ui.json').write_text(json.dumps({'document_id':'canvas-interactions-owned','parent':'root','tab':'Designer','viewport':{'x':40,'y':40,'zoom':1},'sidebar_width':250,'inspector_width':360}))
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
  if int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True))!=int(window):
   subprocess.run(['xdotool','windowactivate','--sync',window],env=env,check=True,capture_output=True)
  assert int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True))==int(window)
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
 def point(x,y):
  v=ui()['viewport'];return (round(ui()['sidebar_width']+5+v['x']+x*v['zoom']),round(172+v['y']+y*v['zoom']))
 def pick(id,modifier=None):
  n=document()['nodes'][id]['layout']
  if modifier:xd('keydown',modifier)
  click(*point(n['x']+45,n['y']+35))
  if modifier:xd('keyup',modifier)
 def drag(start,end,button=1,modifier=None,pause=.15):
  if modifier:xd('keydown',modifier)
  xd('mousemove','--window',window,*start);xd('mousedown',button);time.sleep(.1)
  xd('mousemove','--window',window,round((start[0]+end[0])/2),round((start[1]+end[1])/2));time.sleep(.1)
  xd('mousemove','--window',window,*end);time.sleep(pause);xd('mouseup',button)
  if modifier:xd('keyup',modifier)
  time.sleep(.25)
 def has_color(path,rect,color):
  dx,dy=content_inset();x,y,w,h=rect
  hist=subprocess.check_output(['convert',str(path),'-crop',f'{w}x{h}+{x+dx}+{y+dy}','-format','%c','histogram:info:'],text=True)
  assert color.upper() in hist.upper(),(color,rect,hist[:100])
 try:
  launch();shot('initial')
  # Wheel is zoom without Control, and preserves the point below the pointer.
  before=ui()['viewport'];anchor=(560,390);local=(anchor[0]-ui()['sidebar_width']-5,anchor[1]-172)
  world=((local[0]-before['x'])/before['zoom'],(local[1]-before['y'])/before['zoom'])
  xd('mousemove','--window',window,*anchor);xd('click',4)
  wait(lambda:ui()['viewport']['zoom']>before['zoom']);after=ui()['viewport']
  assert max(abs(world[i]*after['zoom']+after['x' if i==0 else 'y']-local[i]) for i in [0,1])<1.5
  xd('click',5);wait(lambda:abs(ui()['viewport']['zoom']-before['zoom'])<1e-5)
  # Middle-button panning and release outside the canvas.
  before=ui()['viewport'];drag((600,400),(670,445),2)
  wait(lambda:abs(ui()['viewport']['x']-before['x']-70)<2 and abs(ui()['viewport']['y']-before['y']-45)<2)
  drag((670,445),(600,400),2);wait(lambda:abs(ui()['viewport']['x']-before['x'])<2)
  drag((500,350),(240,345),2);wait(lambda:abs(ui()['viewport']['x']-before['x']+260)<2)
  stopped=ui()['viewport'];xd('mousemove','--window',window,500,350);time.sleep(.2);assert ui()['viewport']==stopped
  drag((500,350),(760,355),2);wait(lambda:abs(ui()['viewport']['x']-before['x'])<2)
  pick('a');wait(lambda:selected()=={'a'});pick('b','Shift_L');wait(lambda:selected()=={'a','b'})
  pick('a','Shift_L');assert selected()=={'a','b'}
  pick('a','Control_L');wait(lambda:selected()=={'b'})
  pick('b','Control_L');wait(lambda:selected()==set())
  pick('a');wait(lambda:selected()=={'a'});pick('b','Shift_L');wait(lambda:selected()=={'a','b'})
  # A plain click on an already-selected group member replaces the group.
  pick('b');wait(lambda:selected()=={'b'})
  click(*point(800,400));wait(lambda:selected()==set())
  drag(point(90,90),point(420,190));wait(lambda:selected()=={'a'})
  drag(point(420,190),point(90,90));wait(lambda:selected()=={'a','b','wire'})
  pick('c');wait(lambda:selected()=={'c'})
  drag(point(90,90),point(250,190),modifier='Shift_L');wait(lambda:selected()=={'a','c'})
  drag(point(90,90),point(250,190),modifier='Control_L');wait(lambda:selected()=={'c'})
  # Graph and tree share the same selected set, with a whole-selection contour.
  pick('a');pick('b','Shift_L');wait(lambda:selected()=={'a','b'})
  path=shot('selection-contours');x,y=point(95,125);has_color(path,(x-1,y,3,35),'#70D5C6')
  has_color(path,(230,258,8,15),'#253F45');has_color(path,(230,286,8,15),'#253F45')
  # A selected group drags together. Pause to let debounced routing finish; no model
  # mutation happens until mouse-up, and all movement is a single undo step.
  revision=api('session.status')['revision'];a=document()['nodes']['a']['layout'];b=document()['nodes']['b']['layout']
  start=point(a['x']+45,a['y']+35);end=(start[0]+60,start[1]+45)
  xd('mousemove','--window',window,*start);xd('mousedown',1);xd('mousemove','--window',window,start[0]+25,start[1]+20);time.sleep(.05);xd('mousemove','--window',window,*end);time.sleep(.4);shot('drag-rerouted-preview')
  assert api('session.status')['revision']==revision
  assert document()['nodes']['a']['layout']==a
  xd('mouseup',1);wait(lambda:document()['nodes']['a']['layout']['x']>a['x']+59/ui()['viewport']['zoom'])
  assert selected()=={'a','b'}
  updated=document()['nodes'];assert abs(updated['b']['layout']['x']-b['x']-(updated['a']['layout']['x']-a['x']))<.01
  assert api('session.status')['revision']==revision+1
  command('Undo');wait(lambda:document()['nodes']['a']['layout']==a and document()['nodes']['b']['layout']==b)
  # Tree press selects before drag starts; dropping onto a different tree node reparents.
  click(95,298);wait(lambda:selected()=={'b'})
  xd('keydown','Shift_L');click(95,326);xd('keyup','Shift_L');wait(lambda:selected()=={'b','c'})
  xd('keydown','Control_L');click(95,298);xd('keyup','Control_L');wait(lambda:selected()=={'c'})
  xd('mousemove','--window',window,95,270);xd('mousedown',1);time.sleep(.25);wait(lambda:selected()=={'a'})
  xd('mousemove','--window',window,108,272);time.sleep(.15);xd('mousemove','--window',window,95,214);time.sleep(.25);xd('mouseup',1)
  wait(lambda:document()['nodes']['a']['parentId']=='group');assert selected()=={'a'};shot('tree-reparented')
  # Selecting a composite and its child must translate descendants exactly once.
  pick('group');pick('child','Shift_L');wait(lambda:selected()=={'group','child'})
  saved={id:document()['nodes'][id]['layout'].copy() for id in ['group','a','child']}
  start=point(saved['group']['x']+45,saved['group']['y']+35)
  drag(start,(start[0]+20,start[1]+15),pause=.3)
  wait(lambda:document()['nodes']['group']['layout']['x']>saved['group']['x']+19/ui()['viewport']['zoom'])
  for id in saved:
   layout=document()['nodes'][id]['layout'];assert abs(layout['x']-saved[id]['x']-20/ui()['viewport']['zoom'])<.01;assert abs(layout['y']-saved[id]['y']-15/ui()['viewport']['zoom'])<.01
  command('Undo');wait(lambda:all(document()['nodes'][id]['layout']==old for id,old in saved.items()))
  # A forbidden cycle does not mutate the graph.
  revision=api('session.status')['revision'];drag((95,214),(110,242));time.sleep(.3);assert api('session.status')['revision']==revision
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  print(json.dumps({'canvas':'passed','wheelZoomAnchored':True,'middleDragPan':True,'directionalWindows':True,'addRemoveReplace':True,'treeSelectionAndReparent':True,'selectionContoursAndBounds':True,'groupDragSingleUndo':True,'descendantsMoveOnce':True,'releaseOutsideCanvas':True,'debouncedPreviewWithoutMutation':True,'cycleRejected':True,'artifacts':str(ARTIFACT)}))
 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
