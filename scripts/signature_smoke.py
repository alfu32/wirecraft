#!/usr/bin/env python3
"""Native smoke of the read-only generated signature above the editor, the title bar file name
and version, and the dropdown alignment. Owned PIDs and temporary state only.

Each case launches the desktop on a temporary copy of examples/webserver-rust.orch with the
entity preselected through ui.json, so no input is sent for the signatures. Only the
`dropdown` case (not run by default) clicks once, moving the pointer; pass --no-click to skip
the click. Screenshots go to
target/signature-smoke/. Requires X11/XWayland with Vulkan, xdotool and ImageMagick.
"""
import json,os,pathlib,shutil,subprocess,sys,tempfile,time
from smoke_focus import await_focus
ROOT=pathlib.Path(__file__).resolve().parent.parent
BINARY=os.environ.get('WIRECRAFT_SMOKE_BINARY',str(ROOT/'target/debug/wirecraft-desktop'))
ARTIFACT=ROOT/'target/signature-smoke';ARTIFACT.mkdir(parents=True,exist_ok=True)
# Content-relative position of the editor bar's Language dropdown at 1400x900.
DROPDOWN=(int(os.environ.get('WIRECRAFT_SMOKE_DROPDOWN_X','396')),int(os.environ.get('WIRECRAFT_SMOKE_DROPDOWN_Y','241')))
env=os.environ.copy();env.pop('WAYLAND_DISPLAY',None)
source=json.loads((ROOT/'examples/webserver-rust.orch').read_text())
def named(name,kind=None):
 return next(n['id'] for n in source['nodes'] if n['name']==name and (kind is None or n['kind']==kind))
def wait(app,check,timeout=20):
 until=time.monotonic()+timeout
 while time.monotonic()<until:
  if app.poll() is not None:raise RuntimeError('owned desktop exited; see the log')
  try:
   result=check()
   if result:return result
  except (subprocess.CalledProcessError,IndexError,ValueError):pass
  time.sleep(.15)
 raise TimeoutError('desktop condition timed out')
def content_inset(window):
 text=subprocess.check_output(['xprop','-id',str(window),'_GTK_FRAME_EXTENTS'],env=env,text=True)
 if '=' not in text:return (0,0)
 values=[int(v.strip()) for v in text.split('=',1)[1].split(',')]
 return (values[0],values[2])
def case(name,node,click=None):
 with tempfile.TemporaryDirectory(prefix='wirecraft-signature-') as temporary:
  state=pathlib.Path(temporary);project=state/'webserver-rust.orch'
  shutil.copy(ROOT/'examples/webserver-rust.orch',project)
  (state/'ui.json').write_text(json.dumps({'document_id':source['id'],'selected_ids':[node],'selected':node,'tab':'Editor','section':'source','viewport':{'x':40,'y':40,'zoom':1}}))
  log=open(ARTIFACT/f'{name}.log','w')
  app=subprocess.Popen([BINARY,str(project),'--state-dir',str(state)],env=env,stdout=log,stderr=log)
  try:
   window=wait(app,lambda:subprocess.check_output(['xdotool','search','--onlyvisible','--pid',str(app.pid)],env=env,text=True).splitlines()[0])
   dx,dy=content_inset(window)
   subprocess.run(['xdotool','windowsize','--sync',window,str(1400+2*dx),str(900+2*dy)],env=env,check=True)
   # Connection, snapshot, generation and highlighting.
   time.sleep(6)
   path=ARTIFACT/f'{name}.png'
   subprocess.run(['import','-window',window,str(path)],env=env,check=True)
   if click:
    await_focus(window,env)
    subprocess.run(['xdotool','mousemove','--window',window,str(click[0]+dx),str(click[1]+dy),'click','1'],env=env,check=True)
    time.sleep(1.2)
    subprocess.run(['import','-window',window,str(ARTIFACT/f'{name}-open.png')],env=env,check=True)
   print(path)
  finally:
   app.terminate()
   try:app.wait(10)
   except subprocess.TimeoutExpired:app.kill();app.wait()
cases=sys.argv[1:] or ['processor','type','link']
for c in [c for c in cases if not c.startswith('--')]:
 if c=='processor':case(c,named('socket_respond'))
 elif c=='type':case(c,named('Request','Type'))
 elif c=='link':case(c,named('routeHomeAboutRequest'))
 elif c=='dropdown':
  case(c,named('socket_respond'),None if '--no-click' in sys.argv else DROPDOWN)
