#!/usr/bin/env python3
"""Native smoke: IO badges and dashed outlines, ports inside an expanded and a collapsed composite, Port mode.
Only owned PID and temporary copies of examples/webserver*.orch.
Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,shutil,subprocess,tempfile,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/ports-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='wirecraft-components-') as temporary:
 state=pathlib.Path(temporary)
 for name in ['webserver-component.orch']:shutil.copy(ROOT/'examples'/name,state/name)
 project=state/'webserver-component.orch'
 reference=json.loads(project.read_text())
 (state/'ui.json').write_text(json.dumps({'document_id':reference['id'],'parent':'root','tab':'Designer','selected':None,'selected_ids':[],'sidebar_width':400,'inspector_width':450,'export_settings':{'sheet':{'format':'A4','scale':1,'split':True,'overlapMm':5}},'viewport':{'x':40,'y':40,'zoom':1}}))
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
  if int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True))!=int(window):
   subprocess.run(['xdotool','windowactivate','--sync',window],env=env,check=True,capture_output=True)
  if args[0]=='key':args=(args[0],'--clearmodifiers',*args[1:])
  return subprocess.run(['xdotool',*map(str,args)],env=env,check=True,capture_output=True,text=True).stdout.strip()
 def click(x,y):xd('mousemove','--window',window,x,y,'mousedown',1);time.sleep(.08);xd('mouseup',1);time.sleep(.5)
 def shot(name):
  path=ARTIFACT/(name+'.png');dx,dy=content_inset()
  # Some X11 captures contain backgrounds without text; wait for visible menu labels.
  for attempt in range(12):
   time.sleep(.3);subprocess.run(['import','-window',window,str(path)],env=env,check=True)
   deviation=float(subprocess.check_output(['convert',str(path),'-crop',f'400x18+{20+dx}+{7+dy}','-colorspace','gray','-format','%[fx:standard_deviation]','info:'],text=True))
   if deviation>.015:return path
   xd('mousemove','--window',window,900+attempt%2,300)
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
 try:
  launch()
  root=document()['rootNodeId']
  nodes=document()['nodes']
  by={n['name']:n for n in nodes.values()}
  # IO marking on the transport entities.
  api('entities.patch',{'changes':[{'nodeId':by['socket_accept']['id'],'patch':{'io':'inout'}},{'nodeId':by['socket_respond']['id'],'patch':{'io':'out'}}]})
  # A composite with a nested component port pair.
  a=by['socket_accept']['layout']
  stage=api('threadwork.create_entity',{'name':'stage','attributes':{'layout':{'x':a['x'],'y':a['y']+420}}})['entity']['id']
  inner=api('threadwork.create_entity',{'name':'inner','parentId':stage,'attributes':{'layout':{'x':a['x']+200,'y':a['y']+500}}})['entity']['id']
  api('components.publish',{'nodeId':inner,'direction':'output','name':'tick','typeDefinitionId':'int','parentId':stage,'x':a['x']+450,'y':a['y']+500})
  api('components.publish',{'nodeId':inner,'direction':'input','name':'seed','typeDefinitionId':'int','parentId':stage,'x':a['x']+40,'y':a['y']+500})
  api('threadwork.update_entity',{'nodeId':stage,'patch':{'layout':{'isExpanded':True,'width':560,'height':220}}})
  time.sleep(1.5);command('Zoom Extents');shot('00-expanded')
  api('threadwork.update_entity',{'nodeId':stage,'patch':{'layout':{'isExpanded':False}}})
  time.sleep(2.5);command('Zoom Extents');time.sleep(1);shot('01-collapsed')
  # Port mode: click empty canvas, a new unlinked port appears.
  command('Port Mode')
  before=len([n for n in document()['nodes'].values() if n.get('stereotype')=='Exterior'])
  click(700,820)
  wait(lambda:len([n for n in document()['nodes'].values() if n.get('stereotype')=='Exterior'])==before+1)
  shot('02-port-mode')
  print('OK')
 finally:
  if app and app.poll() is None:app.terminate();app.wait(10)
  log.close()
