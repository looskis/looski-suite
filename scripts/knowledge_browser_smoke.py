#!/usr/bin/env python3
"""The knowledge base through the gateway as a browser without scripting: application
selection, collections, documents, search, previews, conflicts, revisions, moves, members
with a second signed-in person, stored markup staying inert, session gates and framing."""
import base64
import hashlib
import hmac
import json
import http.cookiejar
import re
import secrets
import subprocess
import time
from html import unescape
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import HTTPCookieProcessor, HTTPRedirectHandler, Request, build_opener
from dev import ROOT
from forms_browser_html import Forms
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}';web=f.base+f'/workspaces/{f.workspace}/knowledge';api=base+'/knowledge'
run=secrets.token_hex(4);SECRET='zebrafinch'+run;SHARED='quarterlyclose'+run
def key():return secrets.token_hex(16)
def forms_on(html,action):return [x['pairs'] for x in Forms(html).forms if x['action']==action]
def form_on(html,action,containing=None):
 return next(p for p in forms_on(html,action) if containing is None or any(v==containing for _,v in p))
def replace(pairs,**changes):
 return [(k,v) for k,v in pairs if k not in changes]+[(k,v) for k,v in changes.items() if v is not None]
class Stay(HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
anonymous=build_opener(Stay,HTTPCookieProcessor())
def fetch(opener,url,data=None,origin=None):
 request=Request(url,data=None if data is None else urlencode(data).encode(),headers={'Accept':'text/html',**({'Origin':origin} if origin else {})})
 try:response=opener.open(request,timeout=20)
 except HTTPError as error:response=error
 with response:return response.status,response.read().decode(errors='replace'),response.headers,response.url
def staff(url,data=None,expected=200,opener=None):
 status,html,headers,final=fetch(opener or f.browser,url,data,f.base if data is not None else None)
 assert status==expected,(url,status,expected,html[:600])
 return re.sub('<!--.*?-->','',html,flags=re.S),final
def found(html,n):return re.search(rf'>\s*{n}\s+documents?\s+found',html)
def module(name_,enabled):
 current=next(m for m in f.call(base+'/modules') if m['key']==name_)
 if current['enabled']!=enabled:f.call(base+'/modules/'+name_,'PATCH',{'enabled':enabled,'expected_version':current['version']},key=key())
initial={m['key']:m['enabled'] for m in f.call(base+'/modules')}

# Selected on its own page, with every other optional application off.
for other in initial:module(other,False)
html,_=staff(web);assert 'The knowledge base is turned off' in html and 'Turn on the knowledge base' in html and 'Knowledge base' in html
html,_=staff(web+'?/module',form_on(html,'?/module'))
assert 'Turn off the knowledge base' in html and 'Create a collection' in html
# A rejected creation keeps what was typed and its retry key.
attempt=replace(form_on(html,'?/createCollection'),name='  ',description='Keep this draft')
rejected,_=staff(web+'?/createCollection',attempt,expected=400)
assert 'role="alert"' in rejected and 'Keep this draft' in rejected
html,final=staff(web+'?/createCollection',replace(form_on(rejected,'?/createCollection'),name='Operations '+run))
open_id=final.rstrip('/').rsplit('/',1)[1];assert 'Operations '+run in html
home,_=staff(web)
html,final=staff(web+'?/createCollection',replace(form_on(home,'?/createCollection'),name='Vault '+run,description='',restricted='on'))
vault_id=final.rstrip('/').rsplit('/',1)[1];assert 'Restricted: only members listed' in html
assert f.call(api+'/collections/'+vault_id)['restricted'] and not f.call(api+'/collections/'+open_id)['restricted']
print('PASS: application selection and collections, with a kept draft, with every other application off')

# Documents: a refused unsafe link keeps the draft; markup stays literal text.
PAYLOAD='<script>alert(1)</script> <img src=x onerror=alert(2)>'
TITLE='<svg onload=alert(3)> Handbook '+run
html,_=staff(web+'/documents/new?collection='+open_id)
assert any(v==open_id for _,v in form_on(html,'')) and 'selected' in html
draft=replace(form_on(html,''),title=TITLE,body='Keep me [x](javascript:alert(1))',kind='sop',collection_id=open_id)
rejected,_=staff(web+'/documents/new',draft,expected=400)
assert 'Links must start' in rejected and 'Keep me' in rejected
body=f'# Steps\n\n{PAYLOAD} the {SHARED} checklist with **bold**.\n\n- One\n- Two\n'
html,final=staff(web+'/documents/new',replace(form_on(rejected,''),body=body))
doc_id=final.rstrip('/').rsplit('/',1)[1]
assert '<strong>bold</strong>' in html and '<li>One</li>' in html and 'Standard operating procedure' in html
assert '<script>alert(1)' not in html and '<img src=x' not in html and '<svg onload' not in html
assert '&lt;script>alert(1)&lt;/script>' in html or '&lt;script&gt;alert(1)' in html
assert 'onerror=alert(2)' in unescape(html)
vault_doc=f.call(api+'/documents','POST',{'collection_id':vault_id,'title':'Vault paper '+run,'body':f'Pay {SECRET} and {SHARED}.'},expected=201,key=key())
linking=f.call(api+'/documents','POST',{'collection_id':open_id,'title':'Index '+run,'body':f"[one](/workspaces/{f.workspace}/knowledge/documents/{doc_id}) [two](/workspaces/{f.workspace}/knowledge/documents/{vault_doc['id']})"},expected=201,key=key())
print('PASS: documents through forms, refused links keep the draft, stored markup stays inert')

# Search and previews for the owner, as server-rendered pages.
html,_=staff(web+'?'+urlencode({'q':SHARED}))
assert found(html,2) and f'<mark>{SHARED}</mark>' in html
assert '<script>alert' not in html and '<img src=x' not in html
html,_=staff(web+'?'+urlencode({'q':'"unbalanced OR -- \\'}))
assert found(html,0)
html,_=staff(web+'/documents/'+linking['id'])
assert 'Vault paper '+run in html and SECRET in html
suggested=json.loads(staff(web+'/suggest?'+urlencode({'q':'Vault paper '+run}))[0])['items']
assert [s['id'] for s in suggested]==[vault_doc['id']]
assert json.loads(staff(web+'/suggest?q=')[0])['items']==[]
print('PASS: search with structural highlights, previews and suggestions')

# Editing: the second writer gets a conflict and keeps their draft; nothing is overwritten.
edit_url=web+'/documents/'+doc_id+'/edit'
first,_=staff(edit_url);second,_=staff(edit_url)
html,_=staff(edit_url,replace(form_on(first,''),body='First writer\n'))
assert 'First writer' in html
html,_=staff(edit_url,replace(form_on(second,''),body='Second writer draft\n'),expected=409)
assert 'Someone else changed this document' in html and 'Second writer draft' in html and 'First writer' in html
assert f.call(api+'/documents/'+doc_id)['body']=='First writer\n'
html,_=staff(edit_url,replace(form_on(html,''),body='Second writer again\n'),expected=409)
assert 'Second writer again' in html and f.call(api+'/documents/'+doc_id)['body']=='First writer\n'
fresh,_=staff(edit_url)
html,_=staff(edit_url,replace(form_on(fresh,''),body=body))
assert f.call(api+'/documents/'+doc_id)['revision_number']==3
print('PASS: a stale edit conflicts, keeps its draft and overwrites nothing')

# Revisions: view and restore as a copy.
revisions=f.call(api+'/documents/'+doc_id+'/revisions')['items'];second_revision=revisions[1]
page_url=web+'/documents/'+doc_id+'/revisions'
html,_=staff(page_url);assert 'Restoring copies a revision' in html
assert '<svg onload' not in html
view,_=staff(page_url+'/'+second_revision['id']);assert 'First writer' in view and 'Restore as a new revision' in view
html,final=staff(page_url+'/'+second_revision['id'],form_on(view,''))
assert final.rstrip('/').endswith(doc_id) and 'First writer' in html
current=f.call(api+'/documents/'+doc_id);assert current['revision_number']==4 and current['body']==second_revision['body']
listed=f.call(api+'/documents/'+doc_id+'/revisions')['items']
assert listed[0]['restored_from_number']==2 and listed[1]['id']==revisions[0]['id'] and listed[2]['body']==second_revision['body']
html,_=staff(page_url);first_rev=listed[-1]
html,_=staff(page_url+'?/restore',form_on(html,'?/restore',first_rev['id']))
assert f.call(api+'/documents/'+doc_id)['revision_number']==5
print('PASS: revision pages restore by copying, keeping history')

# A second signed-in person sees only what they may, in pages as well as the API.
def auth(path,payload,token=None):
 data={'path':path,'body':payload,'token':token}
 script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok)throw new Error(r.status+await r.text());console.log(await r.text())})"
 return json.loads(subprocess.check_output(['docker','compose','exec','-T','web','node','-e',script],input=json.dumps(data),text=True,cwd=ROOT,env=f.environment))
def administrator():
 def b64(value):return base64.urlsafe_b64encode(value).rstrip(b'=')
 secret=(f.directory/'secrets/jwt_secret').read_bytes().strip()
 unsigned=b64(b'{"alg":"HS256","typ":"JWT"}')+b'.'+b64(json.dumps({'role':'service_role','iss':f.base+'/auth/v1','exp':int(time.time())+120}).encode())
 return (unsigned+b'.'+b64(hmac.new(secret,unsigned,hashlib.sha256).digest())).decode()
issuer=f.sql("SELECT i.issuer FROM suite.auth_identities i JOIN suite.memberships m ON m.user_id=i.user_id WHERE m.workspace_id='"+f.workspace+"' ORDER BY i.linked_at LIMIT 1")
email='kb-browser-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
identity=auth('/admin/users',{'email':email,'password':password,'email_confirm':True},administrator())
role=f.call(base+'/access/roles','POST',{'name':'KB browser '+run,'grants':[{'action':'knowledge.read','scope':'workspace'},{'action':'knowledge.documents.edit','scope':'workspace'},{'action':'workspace.read','scope':'workspace'}]},expected=201,key=key())
user=f.sql("INSERT INTO suite.users(display_name) VALUES('Browser Reader "+run+"') RETURNING id").splitlines()[0]
f.sql("INSERT INTO suite.auth_identities(user_id,issuer,subject) VALUES('"+user+"','"+issuer+"','"+identity['id']+"')")
membership=f.sql("INSERT INTO suite.memberships(workspace_id,user_id) VALUES('"+f.workspace+"','"+user+"') RETURNING id").splitlines()[0]
f.sql("INSERT INTO suite.membership_roles(workspace_id,membership_id,role_id) VALUES('"+f.workspace+"','"+membership+"','"+role['id']+"')")
reader=build_opener(HTTPCookieProcessor(http.cookiejar.CookieJar()))
with reader.open(Request(f.base+'/login',data=urlencode({'email':email,'password':password}).encode(),headers={'Origin':f.base,'Accept':'text/html'}),timeout=20) as response:assert response.status==200
html,_=staff(web+'?'+urlencode({'q':SHARED}),opener=reader)
assert found(html,1) and doc_id in html and vault_doc['id'] not in html and 'Vault '+run not in html
# The search box echoes the words typed; the result area holds nothing about them.
html,_=staff(web+'?'+urlencode({'q':SECRET}),opener=reader);assert found(html,0) and 'Vault paper' not in html and '<mark' not in html
assert 'Turn off the knowledge base' not in staff(web,opener=reader)[0]
html,_=staff(web+'/documents/'+linking['id'],opener=reader)
assert 'A linked document you cannot open' in html and 'Vault paper' not in html and SECRET not in html
staff(web+'/documents/'+vault_doc['id'],expected=404,opener=reader)
staff(web+'/documents/'+vault_doc['id']+'/edit',expected=404,opener=reader)
staff(web+'/documents/'+vault_doc['id']+'/revisions',expected=404,opener=reader)
staff(web+'/collections/'+vault_id,expected=404,opener=reader)
assert json.loads(staff(web+'/suggest?'+urlencode({'q':'Vault paper '+run}),opener=reader)[0])['items']==[]
# Members through the form; listing applies at once and so does removal, on the same session.
page,_=staff(web+'/collections/'+vault_id)
html,_=staff(web+'/collections/'+vault_id+'?/add',replace(form_on(page,'?/add'),membership_id=membership))
assert 'Browser Reader '+run in html
assert SECRET in staff(web+'/documents/'+vault_doc['id'],opener=reader)[0]
assert found(staff(web+'?'+urlencode({'q':SHARED}),opener=reader)[0],2)
page,_=staff(web+'/collections/'+vault_id)
staff(web+'/collections/'+vault_id+'?/remove',form_on(page,'?/remove',membership))
staff(web+'/documents/'+vault_doc['id'],expected=404,opener=reader)
assert found(staff(web+'?'+urlencode({'q':SHARED}),opener=reader)[0],1)
print('PASS: a second person finds only permitted material; listing and removal apply at once')

# Moving through the form changes what that person can open, in both directions.
page,_=staff(web+'/documents/'+vault_doc['id'])
assert f'value="{open_id}"' in page
html,_=staff(web+'/documents/'+vault_doc['id']+'?/move',replace(form_on(page,'?/move'),collection_id=open_id))
assert 'Operations '+run in html
assert SECRET in staff(web+'/documents/'+vault_doc['id'],opener=reader)[0]
history,_=staff(web+'/documents/'+vault_doc['id']+'/revisions',opener=reader)
assert 'Moved' in history and 'Created' not in history
page,_=staff(web+'/documents/'+vault_doc['id'])
staff(web+'/documents/'+vault_doc['id']+'?/move',replace(form_on(page,'?/move'),collection_id=vault_id))
staff(web+'/documents/'+vault_doc['id'],expected=404,opener=reader)
print('PASS: moving changes access immediately in pages')

# Archive buttons, forged origins, settings.
page,_=staff(web+'/documents/'+linking['id'])
html,_=staff(web+'/documents/'+linking['id']+'?/archive',form_on(page,'?/archive'))
assert 'Restore document' in html and 'archived' in html
html,_=staff(web+'/documents/'+linking['id']+'?/archive',form_on(html,'?/archive'))
assert 'Archive document' in html
status,_,_,_=fetch(f.browser,web+'?/createCollection',form_on(staff(web)[0],'?/createCollection')+[('name','Forged')],'https://evil.example');assert status==403
page,_=staff(web+'/documents/'+doc_id)
status,_,_,_=fetch(f.browser,web+'/documents/'+doc_id+'?/archive',form_on(page,'?/archive'),'null');assert status==403
assert not f.call(api+'/documents/'+doc_id)['archived']
page,_=staff(web+'/collections/'+open_id)
html,_=staff(web+'/collections/'+open_id+'?/save',replace(form_on(page,'?/save'),name='Operations renamed '+run))
assert 'Operations renamed '+run in html
html,_=staff(web+'/collections/'+open_id+'?/archive',form_on(html,'?/archive'))
assert 'This collection is archived' in html and 'Restore collection' in html
staff(web+'/collections/'+open_id+'?/archive',form_on(html,'?/archive'))
print('PASS: archive and restore, forged origins refused, settings saved')

# Session gates and framing on every page; disabled pages close and say so.
pages=[web,web+'/documents/new',web+'/collections/'+open_id,web+'/documents/'+doc_id,web+'/documents/'+doc_id+'/edit',page_url,page_url+'/'+second_revision['id']]
for page in pages+[web+'/suggest?q=x']:
 status,_,headers,_=fetch(anonymous,page)
 assert status in (302,303) and 'login' in headers['Location'],(page,status)
for page in pages:
 status,_,headers,_=fetch(f.browser,page)
 assert status==200 and "frame-ancestors 'none'" in headers['Content-Security-Policy'] and headers['X-Frame-Options']=='DENY' and headers['Referrer-Policy']=='same-origin' and headers['Cache-Control']=='no-store',(page,dict(headers))
assert fetch(f.browser,web+'/documents/'+key()[:8])[0] in (400,404)
module('knowledge',False)
html,_=staff(web);assert 'The knowledge base is turned off' in html
assert fetch(f.browser,web+'/documents/'+doc_id)[0]==403
assert fetch(f.browser,web+'/collections/'+open_id)[0]==403
assert json.loads(staff(web+'/suggest?q=Handbook')[0])['items']==[]
module('knowledge',True)
assert f.call(api+'/documents/'+doc_id)['id']==doc_id
for k,enabled in initial.items():module(k,enabled)
print('PASS: session gates, framing headers and disabled pages')
print('PASS: Knowledge base browser journey')
