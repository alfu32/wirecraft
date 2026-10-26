"""Controlled test peer for cancellation, stale replies, and server requests.
Real clangd is independently exercised by services.rs; this is never a shipped server.
"""
import json,sys
pending={}
def send(value):
 data=json.dumps(value).encode();sys.stdout.buffer.write(f'Content-Length: {len(data)}\r\n\r\n'.encode()+data);sys.stdout.buffer.flush()
while True:
 headers={}
 while True:
  line=sys.stdin.buffer.readline()
  if not line:sys.exit(0)
  if line in (b'\r\n',b'\n'):break
  key,value=line.decode().split(':',1);headers[key.lower()]=value.strip()
 message=json.loads(sys.stdin.buffer.read(int(headers['content-length'])))
 method=message.get('method');params=message.get('params') or {};identifier=message.get('id')
 if method=='initialize':
  send({'jsonrpc':'2.0','id':identifier,'result':{'capabilities':{'positionEncoding':'utf-8','textDocumentSync':1,'documentSymbolProvider':True}}})
 elif method=='initialized':
  send({'jsonrpc':'2.0','id':'configuration','method':'workspace/configuration','params':{'items':[{'section':'wirecraft'}]}})
 elif method in ('textDocument/didOpen','textDocument/didChange'):
  doc=params['textDocument'];send({'jsonrpc':'2.0','method':'textDocument/publishDiagnostics','params':{'uri':doc['uri'],'version':doc['version']-1,'diagnostics':[{'message':'stale'}]}})
  send({'jsonrpc':'2.0','method':'textDocument/publishDiagnostics','params':{'uri':doc['uri'],'version':doc['version'],'diagnostics':[]}})
  for ident in list(pending):send({'jsonrpc':'2.0','id':ident,'result':[{'name':'stale symbol'}]});del pending[ident]
 elif method in ('textDocument/documentSymbol','workspace/symbol'):
  pending[identifier]=params;send({'jsonrpc':'2.0','method':'fixture/pending','params':{'id':identifier}})
 elif method=='$/cancelRequest':
  pending.pop(params['id'],None);send({'jsonrpc':'2.0','id':params['id'],'error':{'code':-32800,'message':'cancelled'}});send({'jsonrpc':'2.0','method':'fixture/cancelled','params':{'id':params['id']}})
 elif method=='shutdown':send({'jsonrpc':'2.0','id':identifier,'result':None})
 elif method=='exit':sys.exit(0)
