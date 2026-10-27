#!/usr/bin/env python3
"""Install an owned archive into a temporary prefix and exercise real binary contracts."""
import base64,json,os,pathlib,select,subprocess,sys,tempfile,time,urllib.request,uuid
root=pathlib.Path(__file__).resolve().parent.parent
archive=pathlib.Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix='wirecraft-package-smoke-') as tmp:
 tmp=pathlib.Path(tmp)
 subprocess.run(['tar','-xzf',str(archive),'-C',str(tmp)],check=True)
 package=next(p for p in tmp.iterdir() if p.is_dir())
 prefix=tmp/'installed'
 subprocess.run([str(package/'install.sh'),str(prefix)],check=True,capture_output=True,text=True)
 backend=prefix/'bin/wirecraft';desktop=prefix/'bin/wirecraft-desktop'
 assert (prefix/'share/doc/wirecraft/gpui-component-Apache-2.0.txt').is_file()
 assert (prefix/'share/doc/wirecraft/gpui-component-patches.md').is_file()
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
  tools=rpc('tools/list')['result']['tools'];assert len(tools)==37
  created=rpc('tools/call',{'name':'threadwork.create_entity','arguments':{'name':'packaged worker'}});assert not created['result']['isError']
  saved=rpc('tools/call',{'name':'wirecraft.save_project','arguments':{}});assert not saved['result']['isError']
  exported=rpc('tools/call',{'name':'wirecraft.export_diagram','arguments':{'path':str(tmp/'diagram.pdf'),'format':'pdf'}});assert not exported['result']['isError']
  assert (tmp/'diagram.pdf').read_bytes().startswith(b'%PDF')
  node_id=created['result']['structuredContent']['entity']['id']
  def workbench(operation,arguments=None):
   value=rpc('tools/call',{'name':'wirecraft.workbench','arguments':{'operation':operation,'arguments':arguments or {}}})['result'];assert not value['isError'],value;return value['structuredContent']
  for kind in ['plan','documentation']:
   preview=workbench('preview_export',{'kind':kind,'options':{'format':'A4'}})
   assert preview['pages']>=1 and '<svg' in preview['svg']
   assert base64.b64decode(preview['pngBase64']).startswith(b'\x89PNG\r\n\x1a\n')
  combined=tmp/'combined.pdf';workbench('export',{'kind':'all','format':'pdf','path':str(combined),'sheetOptions':{'format':'A4'},'documentOptions':{'format':'A4'}});assert combined.read_bytes().startswith(b'%PDF')
  assert workbench('settings')['schemaVersion']==1
  prompt=workbench('prepare_ai_prompt',{'task':'tests','nodeIds':[node_id]});assert 'packaged worker' in prompt['prompt']
  edited=rpc('tools/call',{'name':'threadwork.update_entity','arguments':{'nodeId':node_id,'patch':{'text':{'source':'threadwork_runner__shutdown_request(&threadwork_runner);'}}}});assert not edited['result']['isError']
  generated_dir=tmp/'generated-c17'
  compiled=rpc('tools/call',{'name':'wirecraft.compile_c17','arguments':{'outputDir':str(generated_dir)}});assert not compiled['result']['isError']
  translation=generated_dir/compiled['result']['structuredContent']['files'][0]['path'];executable=tmp/'generated-network'
  def build(executable,args):
   started=rpc('tools/call',{'name':'wirecraft.build','arguments':{'operation':'start','arguments':{'executable':str(executable),'args':list(map(str,args)),'cwd':str(tmp),'timeoutSeconds':5}}});assert not started['result']['isError']
   job_id=started['result']['structuredContent']['id'];until=time.monotonic()+10
   while time.monotonic()<until:
    polled=rpc('tools/call',{'name':'wirecraft.build','arguments':{'operation':'get','arguments':{'id':job_id}}});assert not polled['result']['isError']
    state=polled['result']['structuredContent']
    if state['status']!='running':assert state['status']=='succeeded',state;return
    time.sleep(.02)
   raise TimeoutError('packaged compiler job')
  build('clang',['-std=c17',translation,'-o',executable]);build(executable,[])
  analyzed=rpc('tools/call',{'name':'wirecraft.analyze_c17','arguments':{}});assert not analyzed['result']['isError']
  assert all(d['severity']!=1 for d in analyzed['result']['structuredContent']['diagnostics'])
  configured=rpc('tools/call',{'name':'threadwork.update_entity','arguments':{'nodeId':'root','patch':{'technology':{'languageId':'c','technologyId':'c-native'},'text':{'sourceLanguageId':'c'}}}});assert not configured['result']['isError']
  body='threadwork_runner__shutdown_request(&threadwork_runner);'
  colored=workbench('semantic_tokens',{'nodeId':node_id,'section':'source','text':body})
  assert any(t['kind']=='function' for t in colored['tokens'])
  hovered=workbench('hover',{'nodeId':node_id,'section':'source','text':body,'offset':3})
  assert 'threadwork_runner__shutdown_request' in json.dumps(hovered['hover'])
  call('/api',{'command':'system.shutdown','arguments':{}});app.wait(timeout=10);assert app.returncode==0
  assert json.loads(subprocess.check_output([str(backend),'validate',str(project)],text=True))['nodes']==2
  suites={'desktop':'desktop_smoke.py','walkthrough':'walkthrough_smoke.py','editor':'editor_smoke.py','canvas':'canvas_smoke.py','startup':'startup_smoke.py','gesture':'gesture_release_smoke.py','followup':'followup_smoke.py','draft':'draft_smoke.py'}
  for option,script in suites.items():
   if '--'+option in sys.argv[2:]:
    env=os.environ.copy();env['WIRECRAFT_SMOKE_BINARY']=str(desktop)
    subprocess.run([sys.executable,str(root/'scripts'/script)],env=env,check=True)
  print(json.dumps({'package':'passed','archive':str(archive),'installedBackend':True,'mcpTools':len(tools),'pdfExport':True,'workbenchMcp':True,'nativeC17Graph':True,'generatedClangd':True,'semanticTokensAndHover':True,'installedDraft':'--draft' in sys.argv[2:],'installedStartup':'--startup' in sys.argv[2:],'installedGesture':'--gesture' in sys.argv[2:],'installedFollowup':'--followup' in sys.argv[2:],'installedDesktop':'--desktop' in sys.argv[2:],'installedWalkthrough':'--walkthrough' in sys.argv[2:],'installedEditor':'--editor' in sys.argv[2:],'installedCanvas':'--canvas' in sys.argv[2:]}))
 finally:
  if app.poll() is None:app.terminate();app.wait(timeout=10)
