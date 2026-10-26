#!/usr/bin/env python3
"""Native screenshot walkthrough regression. Only owned PID and temporary fixture/state.
No physical printing. Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,shutil,subprocess,tempfile,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/walkthrough-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
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
  endpoint=wait(discover);wait(lambda:api('session.snapshot')['document']['name']=='WebServer');time.sleep(.5)
  # Client shadows can be included or excluded from the WM's initial bounds.
  # Set a deterministic content size before using content-relative coordinates.
  dx,dy=content_inset();xd('windowsize','--sync',window,1400+2*dx,900+2*dy);time.sleep(.4)
 try:
  launch();command('Zoom Extents');shot('001-designer')
  # Arrow selection must choose the second filtered command; Escape dismisses Help.
  subprocess.run(['xclip','-selection','clipboard'],input='wirecraft-smoke-unset',env=env,text=True,check=True)
  xd('key','ctrl+shift+p');time.sleep(1);xd('type','--clearmodifiers','Help');time.sleep(1);xd('key','Down');time.sleep(.5);shot('016-command-palette');xd('key','Return');time.sleep(1);shot('012-agent-guide')
  click(510,688)
  guide=subprocess.check_output(['xclip','-selection','clipboard','-o'],env=env,text=True,timeout=5)
  assert guide.startswith('# Wirecraft agent guide'),guide[:80]
  xd('key','Escape');time.sleep(.2)
  # Select, drag between actual Kanban lanes, unassign and restore through Undo.
  click(300,102);wait(lambda:ui()['tab']=='Project Management')
  route=next(n['id'] for n in document()['nodes'].values() if n['name']=='routeApiUserDetails')
  click(335,290);wait(lambda:ui()['selected']==route)
  xd('mousemove','--window',window,330,300);xd('mousedown',1);xd('mousemove','--window',window,320,305);time.sleep(.2);xd('mousemove','--window',window,120,320);time.sleep(.25);xd('mouseup',1)
  wait(lambda:document()['nodes'][route]['status']=='BUSINESS');shot('003-kanban-moved')
  click(205,277);wait(lambda:document()['nodes'][route]['status'] is None)
  click(151,151);shot('003-backlog')
  # A source-style inspector name edit must preserve existing source and history.
  before=document()['nodes'][route]['text']['source'];click(165,102)
  click(1200,230);xd('key','ctrl+a');xd('type','--clearmodifiers','routeApiUserDetailsReviewed')
  wait(lambda:(ui().get('draft') or {}).get('name')=='routeApiUserDetailsReviewed')
  click(950,151);wait(lambda:document()['nodes'][route]['name']=='routeApiUserDetailsReviewed');assert document()['nodes'][route]['text']['source']==before;shot('002-inspector')
  # Matrix cells select the real relationship endpoints without mutating the graph.
  click(460,102);wait(lambda:ui()['tab']=='Analysis');time.sleep(.7);shot('005-network-analysis')
  report=api('analysis.report');order=report['nodeOrder'];pair=next((i,order.index(target),source,target) for i,source in enumerate(order[:8]) for target in report['graphs']['principal']['adjacency'][source])
  i,j,a,b=pair;click(round(220+(j+.5)*45),396+i*28);wait(lambda:ui()['tab']=='Designer' and set(ui()['selected_ids'])=={a,b})
  # Preserve pane widths and staged settings through restart.
  xd('mousemove','--window',window,252,450);xd('mousedown',1);xd('mousemove','--window',window,282,450);xd('mouseup',1)
  wait(lambda:ui()['sidebar_width']>=275)
  command('Options…');click(480,273);xd('key','ctrl+a');xd('type','--clearmodifiers','light');click(450,537);xd('key','ctrl+a');xd('type','--clearmodifiers','2');click(60,833)
  wait(lambda:(state/'settings.json').exists() and json.loads((state/'settings.json').read_text())['appearance']['theme']=='light');shot('018-light-options')
  settings=json.loads((state/'settings.json').read_text());assert settings['editorIndentSpaces']==2;assert settings['appearance']['colors']['canvasBackground']=='#F7F9FB'
  # Preview selection must not import a model; explicit insertion must.
  count=len(document()['nodes']);click(575,102);wait(lambda:ui()['tab']=='Archetypes');wait(lambda:api('archetypes.list')['archetypes'])
  click(120,265);time.sleep(.6);assert len(document()['nodes'])==count;shot('004-archetype-preview')
  click(125,830);wait(lambda:len(document()['nodes'])>count);shot('004-archetype-inserted')
  # Preview uses the real backend raster; white content must actually be visible.
  command('Export…');wait(lambda:ui()['tab']=='Export');time.sleep(2);path=shot('013-plan')
  mean=float(subprocess.check_output(['convert',str(path),'-crop','700x300+350+430','-colorspace','gray','-format','%[fx:mean]','info:'],text=True));assert mean>.7,('blank preview',mean)
  click(385,151);time.sleep(1.2);shot('014-documentation')
  # Endpoint management is real and keeps the desktop connection alive.
  command('Manage Server');wait(lambda:ui()['tab']=='MCP');click(126,273)
  wait(lambda:api('mcp.status')['enabled'] is False);assert api('session.status')['revision']>0
  click(42,273);wait(lambda:api('mcp.status')['enabled'] is True);shot('010-mcp')
  # A second isolated fixture exercises telescopic composite geometry through real pointer input.
  def node(id,name,kind,parent,children,x,y,w,h,**extra):
   value={'id':id,'name':name,'kind':kind,'parentId':parent,'children':children,'layout':{'x':x,'y':y,'width':w,'height':h,'openWidth':w,'openHeight':h,'closedWidth':180,'closedHeight':70,'isExpanded':True}}
   value.update(extra);return value
  canvas=state/'canvas.orch'
  canvas.write_text(json.dumps({'id':'owned-canvas-fixture','name':'Canvas fixture','rootNodeId':'root','nodes':[
   node('root','Canvas fixture','Processor',None,['composite','peer','packet','wire'],0,0,200,70),
   node('composite','Composite','Group','root',['child'],100,100,400,300),
   node('child','Child','Processor','composite',[],180,180,160,80),
   node('peer','Peer','Processor','root',[],700,200,180,80),
   node('packet','Packet','Type','root',[],20,20,160,70,typeDefinition={'qualifier':'object','fields':[]}),
   node('wire','messages','Link','root',[],0,0,200,70,link={'sourceNodeId':'child','targetNodeId':'peer','sourcePortName':'out','targetPortName':'in','typeDefinitionId':'packet'})
  ]}))
  api('file.open',{'path':str(canvas),'discardChanges':True});wait(lambda:document()['id']=='owned-canvas-fixture');time.sleep(.6)
  click(65,102);wait(lambda:ui()['tab']=='Designer')
  # Reset a formerly fitted viewport using an owned session's normal Fit; compute screen coordinates.
  # The UI retained the source fixture's fitted transform, which is persisted and observable.
  viewport=ui()['viewport'];origin_x=ui()['sidebar_width']+5;origin_y=172
  def point(x,y):return (round(origin_x+viewport['x']+x*viewport['zoom']),round(origin_y+viewport['y']+y*viewport['zoom']))
  click(*point(112,112));wait(lambda:document()['nodes']['composite']['layout']['isExpanded'] is False)
  click(*point(112,112));wait(lambda:document()['nodes']['composite']['layout']['isExpanded'] is True)
  # Select composite below its toggle and move 50x30 screen pixels (converted to world coordinates).
  x,y=point(145,145);xd('mousemove','--window',window,x,y);xd('mousedown',1);xd('mousemove','--window',window,x+50,y+30);xd('mouseup',1)
  dx=50/viewport['zoom'];dy=30/viewport['zoom'];wait(lambda:abs(document()['nodes']['child']['layout']['x']-(180+dx))<3)
  comp=document()['nodes']['composite']['layout'];child_before=document()['nodes']['child']['layout'].copy()
  x,y=point(comp['x']+comp['width']-3,comp['y']+comp['height']-3);xd('mousemove','--window',window,x,y);xd('mousedown',1);xd('mousemove','--window',window,x+35,y+25);xd('mouseup',1)
  wait(lambda:document()['nodes']['composite']['layout']['width']>comp['width']+35/viewport['zoom']-4);assert document()['nodes']['child']['layout']==child_before
  click(*point(225,36));wait(lambda:ui()['selected']=='wire');shot('001-composite-resize-callout')
  # A blank type's structured collections can be edited and invalid drafts recover intact.
  click(100,270);wait(lambda:ui()['selected']=='packet');click(165,102);wait(lambda:ui()['tab']=='Editor')
  def bottom_inspector():xd('mousemove','--window',window,1240,780);xd('click','--repeat',60,'--delay',5,5);time.sleep(.3)
  def inspector_regions(buttons=False,path=None):
   xd('mousemove','--window',window,900,300)
   path=path or shot('inspector-fields-detection');dx,dy=content_inset()
   args=['convert',str(path),'-crop',f'350x688+{1045+dx}+{172+dy}']
   if buttons:args+=['-fill','white','+opaque','#242c36','-fill','black','-opaque','#242c36']
   args+=['-colorspace','gray','-threshold','8%','-define','connected-components:verbose=true','-connected-components','8','null:']
   output=subprocess.check_output(args,text=True,stderr=subprocess.STDOUT)
   import re
   fields=[]
   for line in output.splitlines():
    match=re.search(r'\d+: (\d+)x(\d+)\+(\d+)\+(\d+).*gray\(0\)',line)
    if match:
     width,height,x,y=map(int,match.groups())
     if (width>280 if buttons else width>=70) and 18<=height<=40:fields.append((1045+x+width//2,172+y+height//2))
   return sorted(fields,key=lambda p:(p[1],p[0]))
  def inspector_inputs():
   tables=(ui().get('draft') or {}).get('tables',{});key='metadata' if tables.get('metadata') else 'types'
   for _ in range(10):
    path=shot('inspector-fields-detection');fields=inspector_regions(False,path);buttons=inspector_regions(True,path)
    if not tables.get(key):return fields
    target=buttons[-2 if key=='metadata' else -3][1]-30
    if len(fields)>=2 and abs(fields[-1][1]-target)<7 and fields[-1][1]==fields[-2][1]:return fields
    bottom_inspector()
   raise RuntimeError(f'Inline {key} row has not repainted: {fields}, {buttons}')
  def add_table_row(field=False):
   bottom_inspector();buttons=inspector_regions(True);assert len(buttons)>=3,buttons
   key='types' if field else 'metadata';before=len((ui().get('draft') or {}).get('tables',{}).get(key,[]))
   if os.environ.get('WIRECRAFT_SMOKE_TRACE'):print('add table row',key,'buttons',buttons,'before',before,flush=True)
   click(*buttons[-3 if field else -2])
   wait(lambda:len((ui().get('draft') or {}).get('tables',{}).get(key,[]))==before+1)
   bottom_inspector()
  def fill(position,text):click(*position);xd('key','ctrl+a');xd('type','--clearmodifiers',text)
  # Restore Dark for deterministic input-color detection, using the normal Options editor.
  command('Options…');click(480,273);xd('key','ctrl+a');xd('type','--clearmodifiers','dark');click(60,833)
  wait(lambda:json.loads((state/'settings.json').read_text())['appearance']['theme']=='dark');click(165,102)
  add_table_row(True);fields=inspector_inputs();assert len(fields)>=2,fields
  fill(fields[-2],'sequence')
  add_table_row();fields=inspector_inputs();assert len(fields)>=2;fill(fields[-2],'fixture_note');wait(lambda:ui()['draft']['tables']['metadata'][-1][0]=='fixture_note');fill(fields[-1],'first');wait(lambda:ui()['draft']['tables']['metadata'][-1][1]=='first')
  add_table_row();fields=inspector_inputs();fill(fields[-2],'fixture_note');wait(lambda:ui()['draft']['tables']['metadata'][-1][0]=='fixture_note');fill(fields[-1],'second');wait(lambda:ui()['draft']['tables']['metadata'][-1][1]=='second')
  click(950,151);time.sleep(.5);assert document()['nodes']['packet']['typeDefinition']['fields']==[]
  wait(lambda:len((ui().get('draft') or {}).get('tables',{}).get('metadata',[]))==2);shot('002-invalid-table-draft')
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  app=subprocess.Popen([BINARY,'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait(lambda:subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(app.pid)],env=env,text=True).splitlines()[0]);time.sleep(1)
  # Rediscover only the newly owned backend after restart.
  backend_pid=int(subprocess.check_output(['pgrep','-P',str(app.pid)],text=True).splitlines()[0]);args=pathlib.Path(f'/proc/{backend_pid}/cmdline').read_bytes().decode().split('\0');token=args[args.index('--token')+1]
  sockets={p.readlink().name for p in pathlib.Path(f'/proc/{backend_pid}/fd').iterdir() if str(p.readlink()).startswith('socket:')}
  for line in pathlib.Path(f'/proc/{backend_pid}/net/tcp').read_text().splitlines()[1:]:
   fields=line.split()
   if fields[3]=='0A' and f'socket:[{fields[9]}]' in sockets:endpoint=f'http://127.0.0.1:{int(fields[1].split(":")[1],16)}'
  assert len(ui()['draft']['tables']['metadata'])==2
  bottom_inspector();fields=inspector_inputs();fill(fields[-2],'another_note');click(950,151)
  wait(lambda:document()['nodes']['packet']['metadata'].get('another_note')=='second');assert document()['nodes']['packet']['metadata']['fixture_note']=='first';assert document()['nodes']['packet']['typeDefinition']['fields'][0]['name']=='sequence';assert document()['nodes']['packet']['typeDefinition']['fields'][0]['typeId']=='string';shot('002-applied-type-and-metadata')
  # Keep the earlier Light-theme restoration assertion meaningful after this deterministic Dark check.
  command('Options…');wait(lambda:ui()['tab']=='Options');time.sleep(.5);click(480,273);xd('key','ctrl+a');xd('type','--clearmodifiers','light');shot('018-options-staged');click(60,833);wait(lambda:json.loads((state/'settings.json').read_text())['appearance']['theme']=='light')
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  # Launch without a project argument so recovery, including unsaved imported IDs, wins.
  app=subprocess.Popen([BINARY,'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  window=wait(lambda:subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(app.pid)],env=env,text=True).splitlines()[0]);time.sleep(1)
  assert json.loads((state/'settings.json').read_text())['appearance']['theme']=='light';assert ui()['sidebar_width']>=275;shot('restored')
  xd('key','ctrl+q');app.wait(timeout=10);assert app.returncode==0
  print(json.dumps({'walkthrough':'passed','kanbanDrag':True,'backlog':True,'inspectorPreservedSource':True,'panePersistence':True,'lightThemeAndIndentPersistence':True,'archetypePreviewAndInsert':True,'realPlanPreview':True,'mcpIndependentControls':True,'compositeCollapseMoveResize':True,'typeUsageCalloutSelection':True,'typeAndMetadataEdits':True,'invalidTableDraftRecovery':True,'matrixEndpointNavigation':True,'artifacts':str(ARTIFACT)}))
 finally:
  if app is not None and app.poll() is None:
   if window is not None:subprocess.run(['import','-window',window,str(ARTIFACT/'interaction-failure.png')],env=env,capture_output=True)
   app.terminate();app.wait(timeout=10)
  log.close()
