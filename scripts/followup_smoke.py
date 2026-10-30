#!/usr/bin/env python3
"""Native canvas/tree gesture regression. Only owned PID and temporary fixture/state.
No physical printing. Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,subprocess,tempfile,time,urllib.request
from smoke_focus import await_focus
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/followup-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-canvas-') as temporary:
 state=pathlib.Path(temporary);project=state/'fixture.orch'
 import shutil
 shutil.copy(ROOT/'docs/migration/threadworks/example-c.orch',project)
 reference=json.loads(project.read_text());reference_nodes=reference['nodes']
 read_time=next(n['id'] for n in reference_nodes if n['name']=='read_time');packets=next(n['id'] for n in reference_nodes if n['name']=='Packets')
 (state/'ui.json').write_text(json.dumps({'document_id':reference['id'],'parent':'root','tab':'Editor','selected':read_time,'selected_ids':[read_time],'sidebar_width':400,'inspector_width':450,'export_settings':{'sheet':{'format':'A4','scale':1,'split':True,'overlapMm':5}},'viewport':{'x':40,'y':40,'zoom':1}}))
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
  endpoint=wait(discover);expected_name=json.loads(project.read_text())['name'];wait(lambda:api('session.snapshot')['document']['name']==expected_name);time.sleep(.5)
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
  launch();time.sleep(3)
  colors=shot('semantic-colors')
  dx,dy=content_inset()
  histogram=subprocess.check_output(['convert',str(colors),'-crop',f'520x550+{420+dx}+{300+dy}','-format','%c','histogram:info:'],text=True)
  for color in ['#72D4EB','#DF83E6','#9CDBB4']:
   assert color in histogram.upper(),f'Missing semantic color {color}'
  xd('mousemove','--window',window,500,465);time.sleep(1);shot('hover-struct')
  xd('mousemove','--window',window,516,645);time.sleep(1);shot('hover-injected-function')
  diagnostics=api('editor.generated_diagnostics')['diagnostics']
  mapped=[d for d in diagnostics if d.get('severity') in [1,2] and d.get('source',{}).get('nodeId')]
  assert mapped
  click(1300,880);time.sleep(.5);shot('compiler-issue-list')
  click(600,185);wait(lambda:ui()['tab']=='Editor' and ui()['selected']==mapped[0]['source']['nodeId'])
  click(65,102);wait(lambda:ui()['tab']=='Designer');command('Zoom Extents');shot('dependency-callouts')
  menu_zoom=ui()['viewport']['zoom']
  xd('mousemove','--window',window,850,380);xd('click',3);time.sleep(.4);shot('context-top')
  xd('mousemove','--window',window,1100,600);xd('click','--repeat',30,'--delay',30,5);time.sleep(.4);shot('context-bottom');assert ui()['viewport']['zoom']==menu_zoom
  # Last context row opens the nested archetype categories.
  click(1050,862);time.sleep(.5);shot('context-archetype-groups')
  click(1050,340);time.sleep(.4);shot('context-archetype-items')
  before=len(document()['nodes']);click(1050,340);wait(lambda:len(document()['nodes'])>before)
  command('Undo');wait(lambda:len(document()['nodes'])==before)

  click(680,102);wait(lambda:ui()['tab']=='Export');time.sleep(2);shot('assembled-plan')
  preview=api('export.preview',{'kind':'plan','options':{'format':'A4','scale':1,'split':True,'overlapMm':5},'overview':True})
  assert preview['overview'] and preview['pages']>1
  assert 'page-contours' in preview['svg'] and 'alignment-marks' in preview['svg']
  assert preview['svg'].count('id="title-block"')==1
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  shutil.copy(ROOT/'crates/wirecraft-core/tests/fixtures/webserver.orch',project)
  webserver=json.loads(project.read_text());router=next(n['id'] for n in webserver['nodes'] if n['name']=='router')
  (state/'recovery.json').unlink(missing_ok=True)
  (state/'ui.json').write_text(json.dumps({'document_id':webserver['id'],'parent':'root','tab':'Designer','selected':router,'selected_ids':[router],'sidebar_width':300,'viewport':{'x':40,'y':40,'zoom':1}}))
  launch();command('Zoom Extents');shot('webserver-routed-channels')
  before_zoom=ui()['viewport']['zoom'];xd('mousemove','--window',window,1030,650,'click','--repeat',3,'--delay',100,4);wait(lambda:ui()['viewport']['zoom']>before_zoom);shot('webserver-routing-detail')
  web=api('export.preview',{'kind':'plan','options':{'format':'Auto'},'overview':True})
  assert web['svg'].count('class="composite-port"')==6
  assert 'routing-error' not in web['svg']
  accept=next(n for n in document()['nodes'].values() if n['name']=='socket_accept')['layout']
  # Restore an exact owned-fixture viewport for the geometry evidence. Rapid
  # synthetic pan/zoom can race session persistence and capture another region.
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  capture_ui=ui();capture_ui['viewport']={'x':850-305-(accept['x']+accept['width']/2)*1.4,'y':510-172-(accept['y']+accept['height']/2)*1.4,'zoom':1.4}
  (state/'ui.json').write_text(json.dumps(capture_ui));launch()
  wait(lambda:abs(ui()['viewport']['zoom']-1.4)<.001)
  shot('self-loop-and-port-caps')
  detail=point(accept['x']-50,accept['y']+accept['height']/2)
  xd('mousemove','--window',window,*detail,'click','--repeat',2,'--delay',100,4);time.sleep(.5);shot('port-caps-and-vias-detail')
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  print(json.dumps({'semanticColors':'passed','contextMenuScrollAndNestedInsert':'passed','dependencyCallouts':'captured','planOverview':'passed','webserverCompositeRouting':'passed','compilerIssueReveal':'passed','hoverPopups':'captured','artifacts':str(ARTIFACT)}))
 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
