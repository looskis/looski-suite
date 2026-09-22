#!/usr/bin/env python3
"""Multipart upload and failure/retry state in the actual SSR browser boundary."""
import secrets
from html.parser import HTMLParser
from urllib.request import Request
from urllib.parse import urlencode
from urllib.error import HTTPError
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}/files';web=f.base+f'/workspaces/{f.workspace}/files'
def key():return secrets.token_hex(16)
class Inputs(HTMLParser):
 def __init__(self,html):super().__init__();self.fields={};self.feed(html)
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='input' and 'name' in attrs:self.fields[attrs['name']]=attrs.get('value','')
def post(url,data,content_type='application/x-www-form-urlencoded',status=200):
 try:response=f.browser.open(Request(url,data=data,headers={'Origin':f.base,'Accept':'text/html','Content-Type':content_type}))
 except HTTPError as error:response=error
 with response:
  html=response.read().decode();assert response.status==status,(response.status,status)
  return html,response.url

def multipart(fields,filename,content):
 boundary='looski-'+key();parts=[]
 for name,value in fields.items():parts.append(('--'+boundary+'\r\nContent-Disposition: form-data; name="'+name+'"\r\n\r\n'+value+'\r\n').encode())
 parts.append(('--'+boundary+'\r\nContent-Disposition: form-data; name="file"; filename="'+filename+'"\r\nContent-Type: application/octet-stream\r\n\r\n').encode()+content+b'\r\n')
 parts.append(('--'+boundary+'--\r\n').encode());return b''.join(parts),'multipart/form-data; boundary='+boundary
retry=key();note='Keep my upload notes '+key();name='Browser upload '+key()+'.bin'
body,content_type=multipart({'idempotency_key':retry,'notes':note,'owner_membership_id':''},name,b'\x00\xffbrowser upload')
f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:html,_=post(web,body,content_type,503)
finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert note in html and Inputs(html).fields['idempotency_key']==retry and 'Reselect the same file' in html
html,url=post(web,body,content_type);assert name in html
file=f.call(base+'/'+url.rsplit('/',1)[-1]);assert file['notes']==note
html,url2=post(web,body,content_type);assert url2==url
print('PASS: multipart binary upload, late-failure draft/key preservation and stable browser retry')

# Preserve submitted version after another writer advances the file.
f.call(base+'/'+file['id'],'PATCH',{'expected_version':file['version'],'filename':'Concurrent browser winner','media_type':'application/octet-stream'},key=key())
draft={'intent':'edit','expected_version':str(file['version']),'idempotency_key':key(),'filename':'Keep my rejected filename','media_type':'text/plain','notes':'Keep my rejected notes','owner_membership_id':file['owner_membership_id']}
html,_=post(url,urlencode(draft).encode(),status=409);fields=Inputs(html).fields
assert fields['expected_version']==draft['expected_version'] and fields['idempotency_key']==draft['idempotency_key']
assert fields['filename']==draft['filename'] and draft['notes'] in html and 'Discard draft and reload current file' in html
assert f.call(base+'/'+file['id'])['filename']=='Concurrent browser winner'
print('PASS: stale browser edits keep draft, original version and retry evidence without changing server state')
