#!/usr/bin/env python3
"""Knowledge base: collections, documents, search, revisions and permissions (Milestone 5
acceptance), with real second users and every other optional application turned off."""
import base64
import hashlib
import hmac
import json
import secrets
import subprocess
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from urllib.parse import urlencode
from dev import ROOT
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace;kb=base+'/knowledge'
run=secrets.token_hex(4)
SHARED='quarterlyclose'+run;SECRET='zebrafinch'+run;SECRET_NAME='Payroll vault '+run;SECRET_TITLE='Severance schedule '+run;OLD='bygoneword'+run
def key():return secrets.token_hex(16)
def module(k,enabled):
 m=next(m for m in call(base+'/modules') if m['key']==k)
 if m['enabled']!=enabled:m=call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
 return m
def auth(path,body,token=None):
 payload={'path':path,'body':body,'token':token}
 script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok)throw new Error(r.status+await r.text());console.log(await r.text())})"
 return json.loads(subprocess.check_output(['docker','compose','exec','-T','web','node','-e',script],input=json.dumps(payload),text=True,cwd=ROOT,env=f.environment))
def administrator():
 def b64(value):return base64.urlsafe_b64encode(value).rstrip(b'=')
 secret=(f.directory/'secrets/jwt_secret').read_bytes().strip()
 unsigned=b64(b'{"alg":"HS256","typ":"JWT"}')+b'.'+b64(json.dumps({'role':'service_role','iss':f.base+'/auth/v1','exp':int(time.time())+120}).encode())
 return (unsigned+b'.'+b64(hmac.new(secret,unsigned,hashlib.sha256).digest())).decode()
issuer=sql("SELECT i.issuer FROM suite.auth_identities i JOIN suite.memberships m ON m.user_id=i.user_id WHERE m.workspace_id='"+f.workspace+"' ORDER BY i.linked_at LIMIT 1")
class Person:
 """A real signed-in member with one role, so denials come from the running API."""
 def __init__(self,label,grants):
  email='knowledge-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
  user=auth('/admin/users',{'email':email,'password':password,'email_confirm':True},administrator())
  self.token=auth('/token?grant_type=password',{'email':email,'password':password})['access_token']
  self.role=call(base+'/access/roles','POST',{'name':label+' '+key()[:8],'grants':self.expand(grants)},expected=201,key=key())
  self.user=sql("INSERT INTO suite.users(display_name) VALUES('"+label+"') RETURNING id").splitlines()[0]
  sql("INSERT INTO suite.auth_identities(user_id,issuer,subject) VALUES('"+self.user+"','"+issuer+"','"+user['id']+"')")
  self.membership=sql("INSERT INTO suite.memberships(workspace_id,user_id) VALUES('"+f.workspace+"','"+self.user+"') RETURNING id").splitlines()[0]
  sql("INSERT INTO suite.membership_roles(workspace_id,membership_id,role_id) VALUES('"+f.workspace+"','"+self.membership+"','"+self.role['id']+"')")
 @staticmethod
 def expand(grants):return [{'action':a.split(':')[0],'scope':(a.split(':')+['workspace'])[1]} for a in grants]
 def grants(self,grants):
  self.role=call(base+'/access/roles/'+self.role['id'],'PATCH',{'name':self.role['name'],'grants':self.expand(grants),'expected_version':self.role['version']},key=key())
 def call(self,path,method='GET',body=None,expected=200,retry=None):
  return call(path,method,body,expected=expected,key=retry or (key() if method!='GET' else None),authenticated=False,headers={'Authorization':'Bearer '+self.token})
def q(**params):return '?'+urlencode(params)
def search(words,who=None,expected=200,**params):return (who.call if who else call)(kb+'/search'+q(q=words,**params),expected=expected)
def suggest(words,who=None):return (who.call if who else call)(kb+'/suggest'+q(q=words))['items']
def collection(name,restricted=False,who=None,**extra):
 return (who.call if who else lambda *a,**k:call(*a,**k,key=key()))(kb+'/collections','POST',{'name':name,'restricted':restricted,**extra},expected=201)
def document(collection_id,title,body='',kind='article'):
 return call(kb+'/documents','POST',{'collection_id':collection_id,'title':title,'body':body,'kind':kind},expected=201,key=key())
def edit(doc,title=None,body=None,who=None,expected=200,retry=None):
 payload={'expected_version':doc['version'],'title':title or doc['title'],'body':doc['body'] if body is None else body,'kind':doc['kind']}
 if who:return who.call(kb+'/documents/'+doc['id'],'PATCH',payload,expected=expected,retry=retry)
 return call(kb+'/documents/'+doc['id'],'PATCH',payload,expected=expected,key=retry or key())
def fetch(doc):return call(kb+'/documents/'+doc['id'])
def members(cid):return call(kb+'/collections/'+cid+'/members')
def add(cid,person):return call(kb+'/collections/'+cid+'/members','POST',{'membership_id':person.membership},expected=201,key=key())
def remove(cid,person):return call(kb+'/collections/'+cid+'/members/'+person.membership+'/remove','POST',key=key())
def export(who,collection_name,expected=200,**selection):return who.call(base+'/exports/page','POST',{'collection':collection_name,**selection},expected=expected)
def export_all(who,collection_name,**selection):
 """Every page, because the fixture accumulates records across runs."""
 items=[];page=export(who,collection_name,limit=25,**selection)
 while True:
  items+=page['items']
  if not page['next']:return items
  page=who.call(base+'/exports/page','POST',page['next'])
def listed_all(who,path):
 items=[];after=None
 while True:
  page=who.call(kb+path+('&' if '?' in path else '?')+urlencode({'limit':100,**({'after':after} if after else {})}))
  items+=page['items'];after=page['next_cursor']
  if not after:return items
def call_all(path):
 items=[];after=None
 while True:
  page=call(kb+path+('&' if '?' in path else '?')+urlencode({'limit':100,**({'after':after} if after else {})}))
  items+=page['items'];after=page['next_cursor']
  if not after:return items
def absent(value,*words):
 text=json.dumps(value)
 for word in words:assert word not in text,(word,text[:800])
initial={m['key']:m['enabled'] for m in call(base+'/modules')}

# Every other optional application is off for the whole journey; disabled closes everything.
for other in initial:module(other,False)
call(kb+'/collections',expected=403)
call(kb+'/search?q=x',expected=403)
call(kb+'/documents','POST',{'collection_id':str(uuid.uuid4()),'title':'x'},expected=403,key=key())
call(base+'/exports/page','POST',{'collection':'knowledge_documents'},expected=403)
module('knowledge',True)
assert not any(m['enabled'] for m in call(base+'/modules') if m['key']!='knowledge')

# Validation and contract basics.
call(kb+'/collections','POST',{'name':' '},expected=400,key=key())
call(kb+'/collections','POST',{'name':'x','expected_version':1},expected=400,key=key())
call(kb+'/search?q=',expected=400)
call(kb+'/search?q=x&limit=500',expected=400)
call(kb+'/search?q=x&after=notacursor',expected=400)
open_c=collection('Operations '+run,description='Everyone reads this')
vault=collection(SECRET_NAME,restricted=True,description='Holds '+SECRET)
assert vault['is_member'] and vault['restricted'] and not open_c['restricted']
assert len(members(vault['id']))==1
call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'  '},expected=400,key=key())
call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'ok','kind':'memo'},expected=400,key=key())
call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'ok','body':'a\u0001b'},expected=400,key=key())
call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'ok','body':'x'*100001},expected=400,key=key())
call(kb+'/documents','POST',{'collection_id':str(uuid.uuid4()),'title':'ok'},expected=404,key=key())
public=document(open_c['id'],'Closing the books '+run,'# Steps\n\nThe '+SHARED+' checklist.\n\n- Reconcile\n- Report',kind='sop')
assert public['kind']=='sop' and public['revision_number']==1 and public['blocks'][0]['kind']=='heading'
retry=key();once=call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'Retried','body':'once'},expected=201,key=retry)
again=call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'Retried','body':'once'},expected=201,key=retry)
assert again['id']==once['id'] and again['version']==once['version']
call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'Different','body':'once'},expected=409,key=retry)
assert sql("SELECT count(*) FROM knowledge.revisions WHERE document_id='"+once['id']+"'")=='1'
hidden=document(vault['id'],SECRET_TITLE,'The '+SHARED+' pay '+SECRET+' figures.')
hidden_rev=call(kb+'/documents/'+hidden['id']+'/revisions')['items'][0]
assert search(SHARED)['total']==2 and {i['id'] for i in search(SHARED)['items']}=={public['id'],hidden['id']}
assert any(p['matched'] and SECRET in p['text'] for i in search(SECRET)['items'] for p in i['snippet'])
assert [i['id'] for i in search(SHARED,kind='sop')['items']]==[public['id']]
assert [i['id'] for i in search(SHARED,collection_id=vault['id'])['items']]==[hidden['id']]
assert [i['id'] for i in suggest(SECRET_TITLE[:-2])]==[hidden['id']]
print('PASS: validation, retries and permitted search with every other application off')

reader=Person('KB reader',['knowledge.read','workspace.data.export'])
author=Person('KB author',['knowledge.read','knowledge.documents.edit','knowledge.collections.manage'])
listed=Person('KB listed',['knowledge.read','knowledge.documents.edit','workspace.data.export'])
scoped=Person('KB scoped',['knowledge.read:assigned','knowledge.documents.edit:assigned'])
nobody=Person('KB nobody',['workspace.read'])

# Someone without access: absent from everything, and shaped exactly like nothing at all.
random_id=str(uuid.uuid4())
words=(SECRET,SECRET_TITLE,SECRET_NAME,hidden['id'],vault['id'],hidden_rev['id'])
page=search(SHARED,reader)
assert page['total']==1 and [i['id'] for i in page['items']]==[public['id']] and page['next_cursor'] is None
absent(page,*words)
for query in (SECRET,SECRET_TITLE,SECRET_NAME):
 assert search(query,reader)=={'items':[],'total':0,'next_cursor':None}
assert suggest(SECRET_TITLE[:-2],reader)==[] and suggest(SECRET,reader)==[]
assert [i['id'] for i in suggest('Closing the books '+run[:6],reader)]==[public['id']]
absent(listed_all(reader,'/collections'),*words)
absent(listed_all(reader,'/documents'),*words)
assert reader.call(kb+'/documents'+q(collection_id=vault['id']))['items']==reader.call(kb+'/documents'+q(collection_id=random_id))['items']==[]
assert search(SHARED,reader,collection_id=vault['id'])['total']==0
absent(reader.call(kb+'/activity?limit=100'),*words)
missing=reader.call(kb+'/documents/'+random_id,expected=404)
for path in ['/documents/'+hidden['id'],'/collections/'+vault['id'],'/collections/'+vault['id']+'/members','/documents/'+hidden['id']+'/revisions','/documents/'+hidden['id']+'/revisions/'+hidden_rev['id'],'/documents'+q(after=hidden['id']),'/collections'+q(after=vault['id']),'/activity'+q(after=hidden_rev['id'])]:
 assert reader.call(kb+path,expected=404)==missing,path
assert reader.call(kb+'/documents'+q(after=random_id),expected=404)==missing
assert reader.call(kb+'/activity'+q(after=random_id),expected=404)==missing
# A cursor naming a hidden document is compared by value, so it finds no more than any other.
forged=base64.urlsafe_b64encode(f"1:{hidden['id']}".encode()).rstrip(b'=').decode()
absent(search(SHARED,reader,after=forged),*words)
assert search(SECRET,reader,after=forged)['total']==0
print('PASS: restricted material is absent from search, counts, snippets, cursors, autocomplete, lists and activity')

# Linked previews tell a reader nothing beyond what they may see.
link_doc=document(open_c['id'],'Linking page '+run,f"See [this one](/workspaces/{f.workspace}/knowledge/documents/{hidden['id']}), [that](/workspaces/{f.workspace}/knowledge/documents/{random_id}) and [open](/workspaces/{f.workspace}/knowledge/documents/{public['id']}).")
assert [l['id'] for l in link_doc['linked']]==[hidden['id'],random_id,public['id']]
owner_view={l['id']:l['preview'] for l in fetch(link_doc)['linked']}
assert owner_view[hidden['id']]['title']==SECRET_TITLE and SECRET in owner_view[hidden['id']]['excerpt'] and owner_view[random_id] is None
reader_view=reader.call(kb+'/documents/'+link_doc['id'])
previews={l['id']:l['preview'] for l in reader_view['linked']}
assert previews[hidden['id']] is None and previews[random_id] is None and previews[public['id']]['title']=='Closing the books '+run
absent(reader_view,SECRET,SECRET_TITLE,SECRET_NAME)
# A link's target is not searchable: the index holds what readers see, not addresses.
assert search(hidden['id'],reader)['total']==0 and search('workspaces',reader,collection_id=open_c['id'])['total']==0
print('PASS: linked previews reveal nothing about hidden or missing documents')

# Exports hold only what the exporter may read.
# The linking page's own source names the target's ID, as its author wrote it; nothing else does.
absent(export_all(reader,'knowledge_documents'),SECRET,SECRET_TITLE,SECRET_NAME,vault['id'],hidden_rev['id'])
absent(export_all(reader,'knowledge_collections'),*words)
assert public['id'] in [i['id'] for i in export_all(reader,'knowledge_documents')]
for selection in ({'collection':'knowledge_revisions','parent_id':hidden['id']},{'collection':'knowledge_collection_members','parent_id':vault['id']}):
 assert reader.call(base+'/exports/page','POST',selection,expected=404)==missing
owner_export=[i for i in call(base+'/exports/page','POST',{'collection':'knowledge_documents','limit':25})['items']]
assert owner_export and all('blocks' in i and 'body' in i for i in owner_export)
catalog={c['collection']:c for c in call(base+'/exports/collections')}
assert catalog['knowledge_revisions']['parent_required'] and catalog['knowledge_documents']['supports_archive_selection'] and catalog['knowledge_documents']['read_permission']=='knowledge.read'
pages=call(base+'/exports/page','POST',{'collection':'knowledge_documents','limit':1})
assert pages['next'] and call(base+'/exports/page','POST',pages['next'])['items'][0]['id']!=pages['items'][0]['id']
reader.grants(['knowledge.read']);export(reader,'knowledge_documents',expected=403);reader.grants(['knowledge.read','workspace.data.export'])
# Timelines: the activity feed gates each revision, and CRM's timeline composes no knowledge.
module('crm',True)
contact=call(base+'/contacts','POST',{'display_name':'KB contact '+run,'identifiers':[]},expected=201,key=key())
timeline=call(base+'/crm/timeline?'+urlencode({'anchor_kind':'contact','anchor_id':contact['id']}))
absent(timeline,SECRET,SECRET_TITLE,SHARED)
module('crm',False)
print('PASS: exports and timelines hold only permitted material')

# Mutations cannot reach or probe a hidden document or collection.
for method,path,body in [('PATCH','/documents/'+hidden['id'],{'expected_version':hidden['version'],'title':'x','body':'y'}),('PATCH','/documents/'+hidden['id'],{'expected_version':999,'title':'x','body':'y'}),('POST','/documents/'+hidden['id']+'/archive',{'expected_version':hidden['version'],'archived':True}),('POST','/documents/'+hidden['id']+'/move',{'expected_version':hidden['version'],'collection_id':open_c['id']}),('POST','/documents/'+hidden['id']+'/revisions/'+hidden_rev['id']+'/restore',{'expected_version':hidden['version']}),('POST','/documents',{'collection_id':vault['id'],'title':'in'}),('PATCH','/collections/'+vault['id'],{'name':'x','expected_version':vault['version']}),('POST','/collections/'+vault['id']+'/archive',{'expected_version':vault['version'],'archived':True}),('POST','/collections/'+vault['id']+'/members',{'membership_id':author.membership}),('POST','/collections/'+vault['id']+'/members/'+author.membership+'/remove',None),('POST','/documents/'+public['id']+'/move',{'expected_version':fetch(public)['version'],'collection_id':vault['id']})]:
 assert author.call(kb+path,method,body,expected=404)==missing,(method,path)
author.call(kb+'/documents/'+public['id']+'/move','POST',{'expected_version':fetch(public)['version'],'collection_id':random_id},expected=404)
# No uniqueness oracle: the same name and title as a hidden record are simply accepted.
clash=collection(SECRET_NAME,who=author)
assert clash['id']!=vault['id'] and author.call(kb+'/documents','POST',{'collection_id':clash['id'],'title':SECRET_TITLE},expected=201)['title']==SECRET_TITLE
author.call(kb+'/collections/'+clash['id']+'/archive','POST',{'expected_version':clash['version'],'archived':True})
# Permissions: no read, assigned scope, listing without a role, and writers needing read.
nobody.call(kb+'/collections',expected=403);nobody.call(kb+'/search?q=x',expected=403);nobody.call(kb+'/suggest?q=x',expected=403);nobody.call(kb+'/documents/'+public['id'],expected=403)
nobody.call(base+'/exports/page','POST',{'collection':'knowledge_documents'},expected=403)
add(vault['id'],nobody);nobody.call(kb+'/documents/'+hidden['id'],expected=403);remove(vault['id'],nobody)
reader.call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'x'},expected=403)
reader.call(kb+'/collections','POST',{'name':'x'},expected=403)
reader.call(kb+'/directory',expected=403)
scoped.call(kb+'/collections','POST',{'name':'x'},expected=403)
assert listed_all(scoped,'/collections')==[] and search(SHARED,scoped)['total']==0
scoped.call(kb+'/documents/'+public['id'],expected=404)
add(open_c['id'],scoped);assert search(SHARED,scoped)['total']==1
assert scoped.call(kb+'/documents/'+public['id'])['id']==public['id'];remove(open_c['id'],scoped)
scoped.call(kb+'/documents/'+public['id'],expected=404)
print('PASS: hidden records cannot be changed or probed; scopes and roles hold')

# Removal from a restricted collection applies to the same token, on the next request.
add(vault['id'],listed)
assert listed.call(kb+'/documents/'+hidden['id'])['body']==hidden['body'] and search(SECRET,listed)['total']==1 and suggest(SECRET_TITLE[:-2],listed)
assert hidden['id'] in [i['id'] for i in export_all(listed,'knowledge_documents')]
retried=key();by_listed=listed.call(kb+'/documents','POST',{'collection_id':vault['id'],'title':'Listed note '+run,'body':'Note '+SECRET},expected=201,retry=retried)
listed_link=listed.call(kb+'/documents/'+link_doc['id'])
assert next(l for l in listed_link['linked'] if l['id']==hidden['id'])['preview']
page_one=listed.call(kb+'/documents'+q(collection_id=vault['id'],limit=1))
remove(vault['id'],listed)
assert listed.call(kb+'/documents/'+hidden['id'],expected=404)==missing
assert listed.call(kb+'/collections/'+vault['id'],expected=404)==missing
assert search(SECRET,listed)=={'items':[],'total':0,'next_cursor':None} and suggest(SECRET_TITLE[:-2],listed)==[]
assert listed.call(kb+'/documents'+q(after=page_one['items'][0]['id']),expected=404)==missing
assert next(l for l in listed.call(kb+'/documents/'+link_doc['id'])['linked'] if l['id']==hidden['id'])['preview'] is None
absent(listed.call(kb+'/activity?limit=100'),SECRET_TITLE,'Listed note '+run)
listed.call(base+'/exports/page','POST',{'collection':'knowledge_revisions','parent_id':hidden['id']},expected=404)
# A retry after removal cannot read back what it once wrote, and the retry table holds no text.
assert listed.call(kb+'/documents','POST',{'collection_id':vault['id'],'title':'Listed note '+run,'body':'Note '+SECRET},expected=404,retry=retried)==missing
assert sql("SELECT count(*) FROM suite.idempotency_records WHERE workspace_id='"+f.workspace+"' AND operation_key LIKE 'knowledge.documents.%' AND (request::text LIKE '%"+SECRET+"%' OR result::text LIKE '%"+SECRET+"%')")=='0'
assert sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"' AND action LIKE 'knowledge.%' AND details::text LIKE '%"+SECRET+"%'")=='0'
# Loss of the role applies the same way.
add(vault['id'],listed);assert listed.call(kb+'/documents/'+hidden['id'])
listed.grants(['workspace.read']);listed.call(kb+'/documents/'+hidden['id'],expected=403)
listed.grants(['knowledge.read','knowledge.documents.edit','workspace.data.export']);remove(vault['id'],listed)
# The override reaches restricted collections, only on a workspace-wide grant it widens.
admin=Person('KB override',['knowledge.read','knowledge.restricted.access'])
assert admin.call(kb+'/documents/'+hidden['id'])['id']==hidden['id'] and search(SECRET,admin)['total']>=1
admin.call(kb+'/documents/'+hidden['id'],'PATCH',{'expected_version':hidden['version'],'title':'x','body':''},expected=403)
admin.grants(['knowledge.read:assigned','knowledge.restricted.access']);admin.call(kb+'/documents/'+hidden['id'],expected=404)
admin.grants(['knowledge.restricted.access']);admin.call(kb+'/documents/'+hidden['id'],expected=403)
print('PASS: removal and role loss apply on the same token, retries included; the override is explicit')

# Moving changes access at once, in both directions, without releasing older history.
draft=document(vault['id'],'Draft policy '+run,'Early '+SECRET+' wording.')
draft_first=call(kb+'/documents/'+draft['id']+'/revisions')['items'][0]
reader.call(kb+'/documents/'+draft['id'],expected=404)
final=edit(draft,body='Approved wording '+SHARED+'.')
assert search('Approved',reader)['total']==0
call(kb+'/documents/'+final['id']+'/move','POST',{'expected_version':draft['version'],'collection_id':open_c['id']},expected=409,key=key())
call(kb+'/documents/'+final['id']+'/move','POST',{'expected_version':final['version'],'collection_id':vault['id']},expected=400,key=key())
move_key=key()
moved=call(kb+'/documents/'+final['id']+'/move','POST',{'expected_version':final['version'],'collection_id':open_c['id']},key=move_key)
assert moved['collection_id']==open_c['id'] and moved['revision_number']==3 and moved['body']==final['body']
assert call(kb+'/documents/'+final['id']+'/move','POST',{'expected_version':final['version'],'collection_id':open_c['id']},key=move_key)['revision_number']==3
assert reader.call(kb+'/documents/'+final['id'])['body']==final['body']
assert final['id'] in [i['id'] for i in search('Approved',reader)['items']]
history=reader.call(kb+'/documents/'+final['id']+'/revisions')
assert [r['number'] for r in history['items']]==[3] and history['items'][0]['change']=='moved'
absent(history,SECRET,'Early',vault['id'],SECRET_NAME)
assert reader.call(kb+'/documents/'+final['id']+'/revisions/'+draft_first['id'],expected=404)==missing
absent(reader.call(base+'/knowledge/activity?limit=100'),SECRET,SECRET_NAME,draft_first['id'])
# The owner, who reaches both, keeps the whole story.
assert [r['number'] for r in call(kb+'/documents/'+final['id']+'/revisions')['items']]==[3,2,1]
# Revision restore cannot read what the user may not see, even for an editor of this document.
author_view=author.call(kb+'/documents/'+final['id'])
assert author.call(kb+'/documents/'+final['id']+'/revisions/'+draft_first['id']+'/restore','POST',{'expected_version':author_view['version']},expected=404)==missing
assert fetch(moved)['revision_number']==3
absent(author.call(kb+'/documents/'+final['id']),SECRET)
# And back: readers of the old collection lose it immediately.
back=call(kb+'/documents/'+final['id']+'/move','POST',{'expected_version':moved['version'],'collection_id':vault['id']},key=key())
assert reader.call(kb+'/documents/'+final['id'],expected=404)==missing
assert final['id'] not in [i['id'] for i in search('Approved',reader)['items']]
reader.call(kb+'/documents/'+final['id']+'/revisions',expected=404)
absent(reader.call(kb+'/activity?limit=100'),final['id'])
# Restricting or opening a collection applies the same way, with its history.
team=collection('Team notes '+run,who=author)
note=author.call(kb+'/documents','POST',{'collection_id':team['id'],'title':'Team note','body':'Team '+OLD},expected=201)
assert reader.call(kb+'/documents/'+note['id'])
restricted_team=author.call(kb+'/collections/'+team['id'],'PATCH',{'name':team['name'],'restricted':True,'expected_version':team['version']})
assert restricted_team['is_member']
reader.call(kb+'/documents/'+note['id'],expected=404)
assert search(OLD,reader)['total']==0
author.call(kb+'/collections/'+team['id'],'PATCH',{'name':team['name'],'restricted':False,'expected_version':restricted_team['version']})
assert reader.call(kb+'/documents/'+note['id'])
print('PASS: moving and restricting change access at once; older history stays with its collection')

# Search index and ranking: current text only, and hidden documents change nothing visible.
history_doc=document(open_c['id'],'Index check '+run,'Contains '+OLD+' here and '+SHARED+'.')
assert search(OLD,reader)['total']==2
history_doc=edit(history_doc,body='Now only '+SHARED+'.')
assert search(OLD)['total']==1 and history_doc['id'] not in [i['id'] for i in search(OLD)['items']]
before=search(SHARED,reader,limit=1);before_all=search(SHARED,reader)
assert before_all['total']>=2 and before['next_cursor']
for n in range(5):document(vault['id'],SHARED+' stuffed '+str(n),(SHARED+' ')*200+SECRET)
after_all=search(SHARED,reader);after=search(SHARED,reader,limit=1)
assert after_all==before_all and after==before,'hidden documents changed what a reader sees'
seen=[];cursor=None
while True:
 page=search(SHARED,reader,limit=1,**({'after':cursor} if cursor else {}))
 seen+=[i['id'] for i in page['items']];cursor=page['next_cursor']
 assert page['total']==before_all['total']
 if not cursor:break
assert len(seen)==len(set(seen))==before_all['total'] and seen==[i['id'] for i in before_all['items']]
assert sql("SELECT count(*) FROM knowledge.documents WHERE collection_id='"+open_c['id']+"' AND search_vector::text LIKE '%"+SECRET+"%'")=='0'
assert 'search_vector' not in json.dumps(search(SHARED)) and 'search_text' not in json.dumps(fetch(public))
assert sql("SELECT string_agg(x,',') FROM (SELECT indexdef x FROM pg_indexes WHERE schemaname='knowledge' AND indexname='documents_search') s").find('gin')>0
print('PASS: the index holds current text only; ranks, counts and cursors ignore hidden documents')

# Revisions: restoring copies, leaves history alone, and honours versions.
story=document(open_c['id'],'Story '+run,'First words')
story2=edit(story,body='Second words')
story3=edit(story2,title='Story renamed '+run,body='Third words')
revisions=call(kb+'/documents/'+story['id']+'/revisions')['items']
assert [r['number'] for r in revisions]==[3,2,1] and [r['change'] for r in revisions]==['edited','edited','created']
snapshot=sql("SELECT string_agg(md5(id::text||number||title||body||created_at::text),',' ORDER BY number) FROM knowledge.revisions WHERE document_id='"+story['id']+"'")
first=revisions[2]
call(kb+'/documents/'+story['id']+'/revisions/'+first['id']+'/restore','POST',{'expected_version':story2['version']},expected=409,key=key())
restore_key=key()
restored=call(kb+'/documents/'+story['id']+'/revisions/'+first['id']+'/restore','POST',{'expected_version':story3['version']},key=restore_key)
assert restored['body']=='First words' and restored['title']=='Story '+run and restored['revision_number']==4
assert call(kb+'/documents/'+story['id']+'/revisions/'+first['id']+'/restore','POST',{'expected_version':story3['version']},key=restore_key)['revision_number']==4
assert sql("SELECT count(*) FROM knowledge.revisions WHERE document_id='"+story['id']+"'")=='4'
after_restore=call(kb+'/documents/'+story['id']+'/revisions')['items']
assert after_restore[0]['change']=='restored' and after_restore[0]['restored_from_number']==1 and after_restore[0]['id']!=first['id']
assert sql("SELECT string_agg(md5(id::text||number||title||body||created_at::text),',' ORDER BY number) FROM knowledge.revisions WHERE document_id='"+story['id']+"' AND number<=3")==snapshot
assert search('First',reader,collection_id=open_c['id'])['total']>=1 and search('Third',reader,collection_id=open_c['id'])['total']==0
assert sql("SELECT has_table_privilege('looski_runtime','knowledge.revisions','UPDATE'),has_table_privilege('looski_runtime','knowledge.revisions','DELETE'),has_table_privilege('looski_runtime','knowledge.documents','DELETE')")=='f|f|f'
assert sql("SELECT has_column_privilege('looski_runtime','knowledge.documents','workspace_id','UPDATE')")=='f'
# A revision of another document, however visible, is not one of this document's.
other_rev=call(kb+'/documents/'+public['id']+'/revisions')['items'][0]
call(kb+'/documents/'+story['id']+'/revisions/'+other_rev['id'],expected=404)
call(kb+'/documents/'+story['id']+'/revisions/'+other_rev['id']+'/restore','POST',{'expected_version':restored['version']},expected=404,key=key())
# Saving unchanged text adds nothing.
same=edit(restored);assert same['revision_number']==4 and same['version']==restored['version']
print('PASS: revision recovery copies, keeps history immutable and checks versions')

# Concurrent edits conflict rather than overwrite.
contested=document(open_c['id'],'Contested '+run,'Start')
def attempt(n):
 try:
  return ('won',call(kb+'/documents/'+contested['id'],'PATCH',{'expected_version':contested['version'],'title':contested['title'],'body':'Writer '+str(n),'kind':'article'},key=key())['body'])
 except AssertionError as failure:
  assert ', 409, 200,' in str(failure),failure
  return ('lost',None)
with ThreadPoolExecutor(8) as pool:outcomes=list(pool.map(attempt,range(8)))
winners=[o[1] for o in outcomes if o[0]=='won']
assert len(winners)==1 and fetch(contested)['body']==winners[0] and fetch(contested)['revision_number']==2
assert sql("SELECT count(*) FROM knowledge.revisions WHERE document_id='"+contested['id']+"'")=='2'
stale=edit(contested,body='Stale overwrite',expected=409)
assert fetch(contested)['body']==winners[0]
# One retry key applies once however many copies race.
racing=document(open_c['id'],'Racing '+run,'Start');racing_key=key()
with ThreadPoolExecutor(6) as pool:results=list(pool.map(lambda _:call(kb+'/documents/'+racing['id'],'PATCH',{'expected_version':racing['version'],'title':racing['title'],'body':'Once','kind':'article'},key=racing_key),range(6)))
assert {r['revision_number'] for r in results}=={2} and sql("SELECT count(*) FROM knowledge.revisions WHERE document_id='"+racing['id']+"'")=='2'
print('PASS: concurrent edits conflict and retries apply once')

# Stored markup stays inert: the same parser and typed output as CMS.
payload='<script>alert(1)</script> <img src=x onerror=alert(2)>'
marked=document(open_c['id'],'<svg onload=alert(3)> '+run,payload+' '+OLD+'markup')
assert marked['blocks'][0]['inlines'][0]['text'].startswith('<script>')
for bad in ['[x](javascript:alert(1))','[x](data:text/html,x)','[x](//evil.example)','[x](java\tscript:alert(1))']:
 call(kb+'/documents','POST',{'collection_id':open_c['id'],'title':'bad','body':bad},expected=400,key=key())
 call(kb+'/documents/'+marked['id'],'PATCH',{'expected_version':marked['version'],'title':marked['title'],'body':bad},expected=400,key=key())
# Text smuggled into storage still renders with no link.
sql("UPDATE knowledge.documents SET body='[x](javascript:alert(1))' WHERE id='"+marked['id']+"'")
smuggled=fetch(marked)['blocks'][0]['inlines'][0];assert smuggled['text']=='x' and smuggled.get('href') is None
sql("UPDATE knowledge.documents SET body='"+payload.replace("'","''")+"' WHERE id='"+marked['id']+"'")
snippet=search(OLD+'markup')['items'][0]['snippet']
assert all(set(p)=={'text','matched'} for p in snippet) and any(p['matched'] and p['text']==(OLD+'markup').lower() for p in snippet)
print('PASS: markup stays literal and unsafe links are refused or inert')

# Archive leaves search and freezes; restoring brings it back.
gone=document(open_c['id'],'Archivable '+run,OLD+'archive')
archived=call(kb+'/documents/'+gone['id']+'/archive','POST',{'expected_version':gone['version'],'archived':True},key=key())
assert archived['archived'] and search(OLD+'archive')['total']==0 and suggest('Archivable '+run)==[]
assert gone['id'] in [i['id'] for i in call(kb+'/documents'+q(archived='true',collection_id=open_c['id']))['items']]
edit(archived,body='no',expected=409)
assert reader.call(kb+'/documents/'+gone['id'])['archived']
call(kb+'/documents/'+gone['id']+'/archive','POST',{'expected_version':gone['version'],'archived':False},expected=409,key=key())
back_doc=call(kb+'/documents/'+gone['id']+'/archive','POST',{'expected_version':archived['version'],'archived':False},key=key())
assert search(OLD+'archive')['total']==1
shelf=collection('Shelf '+run);shelved=document(shelf['id'],'Shelved '+run,OLD+'shelved')
shelf2=call(kb+'/collections/'+shelf['id']+'/archive','POST',{'expected_version':shelf['version'],'archived':True},key=key())
assert search(OLD+'shelved')['total']==0 and fetch(shelved)['collection_archived']
edit(shelved,body='no',expected=409)
call(kb+'/documents','POST',{'collection_id':shelf['id'],'title':'in'},expected=409,key=key())
call(kb+'/documents/'+public['id']+'/move','POST',{'expected_version':fetch(public)['version'],'collection_id':shelf['id']},expected=409,key=key())
assert shelf['id'] not in [c['id'] for c in call_all('/collections')] and shelf['id'] in [c['id'] for c in call_all('/collections?archived=true')]
call(kb+'/collections/'+shelf['id']+'/archive','POST',{'expected_version':shelf2['version'],'archived':False},key=key())
assert search(OLD+'shelved')['total']==1
print('PASS: archive lifecycle')

# Cross-workspace: other workspaces\' records do not exist here, and nobody crosses over.
foreign=sql("INSERT INTO suite.workspaces(name) VALUES('Knowledge foreign fixture') RETURNING id").splitlines()[0]
foreign_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Knowledge foreign member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
foreign_collection=sql("INSERT INTO knowledge.collections(workspace_id,name,created_by_membership_id) VALUES('"+foreign+"','Foreign "+SECRET+"','"+foreign_member+"') RETURNING id").splitlines()[0]
foreign_doc=sql("INSERT INTO knowledge.documents(workspace_id,collection_id,title,body,search_text) VALUES('"+foreign+"','"+foreign_collection+"','Foreign doc "+run+"','foreignonly"+run+"','foreignonly"+run+"') RETURNING id").splitlines()[0]
call('/api/v1/workspaces/'+foreign+'/knowledge/collections',expected=403)
call('/api/v1/workspaces/'+foreign+'/knowledge/search?q=foreignonly'+run,expected=403)
call(kb+'/documents/'+foreign_doc,expected=404)
call(kb+'/collections/'+foreign_collection,expected=404)
assert search('foreignonly'+run)['total']==0 and suggest('Foreign doc '+run)==[]
call(kb+'/documents','POST',{'collection_id':foreign_collection,'title':'x'},expected=404,key=key())
call(kb+'/documents/'+public['id']+'/move','POST',{'expected_version':fetch(public)['version'],'collection_id':foreign_collection},expected=404,key=key())
call(kb+'/collections/'+open_c['id']+'/members','POST',{'membership_id':foreign_member},expected=400,key=key())
cross=document(open_c['id'],'Cross link '+run,f'[f](/workspaces/{foreign}/knowledge/documents/{foreign_doc}) [g](/workspaces/{f.workspace}/knowledge/documents/{foreign_doc})')
assert [(l['id'],l['preview']) for l in cross['linked']]==[(foreign_doc,None)]
print('PASS: cross-workspace denial and isolation')

# CLI and MCP reach the same business rules.
cli_c=f.cli(['knowledge','collection-create','--idempotency-key',key()],{'name':'CLI '+run})
cli_d=f.cli(['knowledge','document-create','--idempotency-key',key()],{'collection_id':cli_c['id'],'title':'CLI doc '+run,'body':'clionly'+run,'kind':'sop'})
assert f.cli(['knowledge','search','clionly'+run])['total']==1 and f.cli(['knowledge','search','clionly'+run,'--sop'])['total']==1
assert f.cli(['knowledge','suggest','CLI doc '+run])['items'][0]['id']==cli_d['id']
cli_e=f.cli(['knowledge','document-update',cli_d['id'],'--idempotency-key',key()],{'expected_version':cli_d['version'],'title':cli_d['title'],'body':'cli edit','kind':'sop'})
revs=f.cli(['knowledge','revisions',cli_d['id']])['items'];assert [r['number'] for r in revs]==[2,1]
assert f.cli(['knowledge','revision',cli_d['id'],revs[1]['id']])['body']=='clionly'+run
cli_r=f.cli(['knowledge','restore',cli_d['id'],revs[1]['id'],'--idempotency-key',key(),'--expected-version',str(cli_e['version'])])
assert cli_r['body']=='clionly'+run
cli_m=f.cli(['knowledge','document-move',cli_d['id'],'--collection',open_c['id'],'--idempotency-key',key(),'--expected-version',str(cli_r['version'])])
assert cli_m['collection_id']==open_c['id'] and f.cli(['knowledge','document-get',cli_d['id']])['revision_number']==4
assert f.cli(['knowledge','member-list',cli_c['id']]) and f.cli(['knowledge','directory'])
f.cli(['knowledge','member-add',cli_c['id'],reader.membership,'--idempotency-key',key()]);f.cli(['knowledge','member-remove',cli_c['id'],reader.membership,'--idempotency-key',key()])
assert f.cli(['knowledge','activity','--limit','5'])['items']
def tool(name,arguments):
 result=f.mcp(name,{'workspace_id':f.workspace,**arguments});assert not result.get('isError'),result
 return json.loads(result['content'][0]['text'])
assert tool('search_knowledge',{'q':'clionly'+run})['total']==1
assert tool('get_knowledge_document',{'document_id':cli_d['id']})['id']==cli_d['id']
mcp_d=tool('create_knowledge_document',{'idempotency_key':key(),'input':{'collection_id':open_c['id'],'title':'MCP doc '+run,'body':'mcponly'+run}})
mcp_e=tool('update_knowledge_document',{'document_id':mcp_d['id'],'idempotency_key':key(),'input':{'expected_version':mcp_d['version'],'title':mcp_d['title'],'body':'mcp edit'}})
assert tool('suggest_knowledge_documents',{'q':'MCP doc '+run})['items'][0]['id']==mcp_d['id'] and len(tool('list_knowledge_revisions',{'document_id':mcp_d['id']})['items'])==2
stale_tool=f.mcp('update_knowledge_document',{'workspace_id':f.workspace,'document_id':mcp_d['id'],'idempotency_key':key(),'input':{'expected_version':mcp_d['version'],'title':'x','body':'y'}})
assert stale_tool.get('isError')
print('PASS: CLI and MCP parity')

# Disabling closes every route and keeps the data; other applications are untouched by it.
counts=sql("SELECT (SELECT count(*) FROM knowledge.collections WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM knowledge.documents WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM knowledge.revisions WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM knowledge.collection_members WHERE workspace_id='"+f.workspace+"')")
current=fetch(public)
module('knowledge',False)
denied=[('GET','/collections',None),('POST','/collections',{'name':'x'}),('GET','/collections/'+open_c['id'],None),('PATCH','/collections/'+open_c['id'],{'name':'x','expected_version':1}),('POST','/collections/'+open_c['id']+'/archive',{'expected_version':1,'archived':True}),('GET','/collections/'+open_c['id']+'/members',None),('POST','/collections/'+open_c['id']+'/members',{'membership_id':reader.membership}),('POST','/collections/'+open_c['id']+'/members/'+reader.membership+'/remove',None),('GET','/directory',None),('GET','/documents',None),('POST','/documents',{'collection_id':open_c['id'],'title':'x'}),('GET','/documents/'+public['id'],None),('PATCH','/documents/'+public['id'],{'expected_version':current['version'],'title':'x'}),('POST','/documents/'+public['id']+'/archive',{'expected_version':current['version'],'archived':True}),('POST','/documents/'+public['id']+'/move',{'expected_version':current['version'],'collection_id':vault['id']}),('GET','/documents/'+public['id']+'/revisions',None),('GET','/documents/'+public['id']+'/revisions/'+other_rev['id'],None),('POST','/documents/'+public['id']+'/revisions/'+other_rev['id']+'/restore',{'expected_version':current['version']}),('GET','/search?q=x',None),('GET','/suggest?q=x',None),('GET','/activity',None)]
for method,path,body in denied:
 call(kb+path,method,body,expected=403,key=key() if method!='GET' else None)
 reader.call(kb+path,method,body,expected=403)
for selection in ('knowledge_collections','knowledge_documents'):call(base+'/exports/page','POST',{'collection':selection},expected=403)
call(base+'/exports/page','POST',{'collection':'knowledge_revisions','parent_id':public['id']},expected=403)
assert sql("SELECT (SELECT count(*) FROM knowledge.collections WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM knowledge.documents WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM knowledge.revisions WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM knowledge.collection_members WHERE workspace_id='"+f.workspace+"')")==counts
try:f.cli(['knowledge','collection-list']);raise SystemExit('CLI read while disabled')
except subprocess.CalledProcessError:pass
assert f.mcp('list_knowledge_collections',{'workspace_id':f.workspace}).get('isError')
module('knowledge',True)
assert fetch(public)==current and public['id'] in [i['id'] for i in search(SHARED,reader)['items']]
assert reader.call(kb+'/documents/'+hidden['id'],expected=404)==missing
print('PASS: disabling closes every route, CLI and MCP and retains the data')

# Restricted material must not depend on any other application to stay closed.
assert not any(m['enabled'] for m in call(base+'/modules') if m['key']!='knowledge')
for k,enabled in initial.items():module(k,enabled)
print('PASS: Knowledge base journey')
