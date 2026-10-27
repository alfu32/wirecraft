import json,pathlib,select,subprocess,tempfile,urllib.request,sys
binary=pathlib.Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix='wirecraft-layout-api-') as directory:
 project=pathlib.Path(directory)/'fixture.orch'
 subprocess.run([str(binary),'new',str(project)],check=True,capture_output=True)
 app=subprocess.Popen([str(binary),'serve','--project',str(project)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 try:
  assert select.select([app.stdout],[],[],10)[0]
  ready=json.loads(app.stdout.readline())
  def call(command,arguments=None):
   request=urllib.request.Request(ready['endpoint']+'/api',json.dumps({'command':command,'arguments':arguments or {}}).encode(),{'Content-Type':'application/json','Authorization':'Bearer '+ready['token']})
   with urllib.request.urlopen(request,timeout=10) as response: value=json.load(response)
   assert value['ok'],value.get('error')
   return value['result']
  ids=[call('threadwork.create_entity',{'name':'Box','attributes':{'layout':{'x':x,'y':x,'width':100,'height':50}}})['entity']['id'] for x in [0,50,1000]]
  link=call('threadwork.create_link',{'name':'Selected link','sourceNodeId':ids[0],'targetNodeId':ids[1]})['entity']['id']
  call('threadwork.update_entity',{'nodeId':link,'patch':{'layout':{'x':-9999,'y':9999,'width':5000,'height':5000}}})
  before=call('session.snapshot')['document']
  operations=[('left',[0,0,0]),('right',[1000,1000,1000]),('top',[0,0,0]),('bottom',[1000,1000,1000]),('distribute-horizontal',[0,500,1000]),('distribute-vertical',[0,500,1000])]
  for operation,expected in operations:
   result=call('entities.align',{'nodeIds':[link,*ids],'operation':operation})
   assert result['aligned']==ids
   document=call('session.snapshot')['document']
   horizontal=operation in ['left','right','distribute-horizontal']
   assert [document['nodes'][id]['layout']['x' if horizontal else 'y'] for id in ids]==expected
   assert document['nodes'][link]==before['nodes'][link]
   call('edit.undo')
   assert call('session.snapshot')['document']==before
  call('system.shutdown');app.wait(timeout=10);assert app.returncode==0
  print(json.dumps({'mixedSelectionHttp':'passed','operations':[o for o,_ in operations],'linksUnchanged':True,'responseContainsOnlyBoxes':True,'singleUndo':True,'temporaryFixture':True}))
 finally:
  if app.poll() is None:app.terminate();app.wait(timeout=10)
