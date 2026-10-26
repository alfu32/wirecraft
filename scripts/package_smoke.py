#!/usr/bin/env python3
"""Install an owned archive into a temporary prefix and exercise real binary contracts."""
import json,os,pathlib,select,subprocess,sys,tempfile,urllib.request,uuid
root=pathlib.Path(__file__).resolve().parent.parent
archive=pathlib.Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix='wirecraft-package-smoke-') as tmp:
 tmp=pathlib.Path(tmp)
 subprocess.run(['tar','-xzf',str(archive),'-C',str(tmp)],check=True)
 package=next(p for p in tmp.iterdir() if p.is_dir())
 prefix=tmp/'installed'
 subprocess.run([str(package/'install.sh'),str(prefix)],check=True,capture_output=True,text=True)
 backend=prefix/'bin/wirecraft';desktop=prefix/'bin/wirecraft-desktop'
 subprocess.run([str(desktop),'--help'],check=True,capture_output=True)
 project=tmp/'fixture.orch'
 subprocess.run([str(backend),'new',str(project)],check=True)
 validation=json.loads(subprocess.check_output([str(backend),'validate',str(project)],text=True));assert validation['valid']
 token=str(uuid.uuid4())
 app=subprocess.Popen([str(backend),'serve','--project',str(project),'--token',token],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  assert select.select([app.stdout],[],[],10)[0], 'backend readiness timeout'
  ready=json.loads(app.stdout.readline());endpoint=ready['endpoint'];assert ready['token']==token
  def call(route,value):
   request=urllib.request.Request(endpoint+route,json.dumps(value).encode(),{'Content-Type':'application/json','Authorization':'Bearer '+token})
   with urllib.request.urlopen(request,timeout=10) as response:return json.load(response)
  def rpc(method,params=None):return call('/mcp',{'jsonrpc':'2.0','id':1,'method':method,'params':params or {}})
  assert rpc('initialize',{'protocolVersion':'2025-11-25'})['result']['protocolVersion']=='2025-11-25'
  tools=rpc('tools/list')['result']['tools'];assert len(tools)==36
  created=rpc('tools/call',{'name':'threadwork.create_entity','arguments':{'name':'packaged worker'}});assert not created['result']['isError']
  saved=rpc('tools/call',{'name':'wirecraft.save_project','arguments':{}});assert not saved['result']['isError']
  exported=rpc('tools/call',{'name':'wirecraft.export_diagram','arguments':{'path':str(tmp/'diagram.pdf'),'format':'pdf'}});assert not exported['result']['isError']
  assert (tmp/'diagram.pdf').read_bytes().startswith(b'%PDF')
  call('/api',{'command':'system.shutdown','arguments':{}});app.wait(timeout=10);assert app.returncode==0
  assert json.loads(subprocess.check_output([str(backend),'validate',str(project)],text=True))['nodes']==2
  if '--desktop' in sys.argv[2:]:
   env=os.environ.copy();env['WIRECRAFT_SMOKE_BINARY']=str(desktop)
   subprocess.run([sys.executable,str(root/'scripts/desktop_smoke.py')],env=env,check=True)
  print(json.dumps({'package':'passed','archive':str(archive),'installedBackend':True,'mcpTools':len(tools),'pdfExport':True,'installedDesktop':'--desktop' in sys.argv[2:]}))
 finally:
  if app.poll() is None:app.terminate();app.wait(timeout=10)
