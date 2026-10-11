#!/usr/bin/env python3
"""CMS: content types, drafts, revisions, media and public pages (Milestone 3 acceptance)
on an isolated installation, including draft visibility, publish permission, file access,
revision recovery, a form embedded in published content and its CRM result."""
import base64
import json
import secrets
import time
from urllib.error import HTTPError
from urllib.request import Request,urlopen
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace
PNG=b'\x89PNG\r\n\x1a\n'+b'\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\rIDATx\x9cc\xf8\xcf\xc0\xf0\x1f\x00\x05\x00\x01\xff\x89\x99=\x1d\x00\x00\x00\x00IEND\xaeB`\x82'
def key():return secrets.token_hex(16)
def client():return {'CF-Connecting-IP':'198.51.100.'+str(secrets.randbelow(250)+1)}
def module(k,enabled):
 m=next(m for m in call(base+'/modules') if m['key']==k)
 if m['enabled']!=enabled:m=call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
 return m
owner_role="(SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner')"
class without:
 """The owner lacks these permissions inside the block, restored on any exit."""
 def __init__(self,*permissions):self.permissions=permissions
 def __enter__(self):
  sql("DELETE FROM suite.role_permissions WHERE role_id="+owner_role+" AND permission_key IN ("+','.join("'"+p+"'" for p in self.permissions)+")")
 def __exit__(self,*_):
  for p in self.permissions:sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"',"+owner_role+",'"+p+"') ON CONFLICT DO NOTHING")
def raw(path):
 """Anonymous bytes and headers, as a browser requests a public address."""
 try:response=urlopen(Request(f.base+path,headers=client()),timeout=20)
 except HTTPError as error:response=error
 with response:return response.status,response.read(),response.headers
def public(path,expected=200):return call(path,expected=expected,authenticated=False,headers=client())
def upload(content,media_type='image/png',name=None):
 return call(base+'/files','POST',{'filename':name or 'CMS '+key(),'media_type':media_type,'content_base64':base64.b64encode(content).decode()},expected=201,key=key())

initial={m['key']:m['enabled'] for m in call(base+'/modules')}
# CMS works with both Forms and CRM disabled: nothing until the embed section needs them.
module('crm',False);module('forms',False)
module('cms',False)
call(base+'/cms/types',expected=403);call(base+'/cms/entries',expected=403);call(base+'/cms/media',expected=403)
module('cms',True)

# Content types.
fields=[
 {'key':'summary','label':'Summary','kind':'text','required':True},
 {'key':'body','label':'Body','kind':'rich_text'},
 {'key':'hero','label':'Hero','kind':'media'},
 {'key':'rating','label':'Rating','kind':'number'},
 {'key':'featured','label':'Featured','kind':'boolean'},
 {'key':'event_on','label':'Event on','kind':'date'},
 {'key':'more','label':'More','kind':'url'},
 {'key':'signup','label':'Sign up','kind':'form'},
]
name='Pages '+key()
call(base+'/cms/types','POST',{'name':name,'fields':[{'key':'Bad Key','label':'x','kind':'text'}]},expected=400,key=key())
call(base+'/cms/types','POST',{'name':name,'fields':[{'key':'a','label':'x','kind':'html'}]},expected=400,key=key())
retry=key();content_type=call(base+'/cms/types','POST',{'name':name,'description':'Our pages','fields':fields},expected=201,key=retry)
assert call(base+'/cms/types','POST',{'name':name,'description':'Our pages','fields':fields},expected=201,key=retry)==content_type
tpath=base+'/cms/types/'+content_type['id'];site='/api/v1/public/cms/'+content_type['public_key']
call(tpath,'PATCH',{'name':name,'fields':fields},expected=400,key=key())
call(tpath,'PATCH',{'name':name,'fields':fields,'expected_version':99},expected=409,key=key())
changed=[{**fields[0],'kind':'number'},*fields[1:]]
call(tpath,'PATCH',{'name':name,'fields':changed,'expected_version':content_type['version']},expected=400,key=key())
other_type=call(base+'/cms/types','POST',{'name':'Other '+key()},expected=201,key=key())
print('PASS: content types validate fields, retry idempotently, detect stale edits and keep field kinds')

# Drafts stay private until published.
slug='about-'+secrets.token_hex(3)
xss='<script>alert(1)</script><img src=x onerror=alert(1)>'
create={'type_id':content_type['id'],'title':'First draft','slug':slug,'values':{'body':'# Heading\n\nSecret **draft** words'}}
call(base+'/cms/entries','POST',{**create,'slug':'Not A Slug'},expected=400,key=key())
call(base+'/cms/entries','POST',{**create,'values':{'nope':'x'}},expected=400,key=key())
call(base+'/cms/entries','POST',{**create,'type_id':f.workspace},expected=404,key=key())
for bad in [{'body':'[x](javascript:alert(1))'},{'more':'javascript:alert(1)'},{'more':'data:text/html,x'},{'rating':'1'},{'event_on':'2026-02-30'},{'featured':'yes'}]:
 call(base+'/cms/entries','POST',{**create,'values':bad},expected=400,key=key())
retry=key();entry=call(base+'/cms/entries','POST',create,expected=201,key=retry)
assert call(base+'/cms/entries','POST',create,expected=201,key=retry)==entry
epath=base+'/cms/entries/'+entry['id'];public_page=site+'/entries/'+slug
assert entry['published'] is None and entry['draft']['revision_number']==1 and entry['unpublished_changes']
public(public_page,404)
assert public(site)['entries']==[],'A draft is not listed publicly'
preview=call(epath+'/preview');assert preview['title']=='First draft' and preview['published_at'] is None
# Required fields block publishing only; an unfinished draft saves fine.
call(epath+'/publish','POST',{'expected_version':entry['version']},expected=409,key=key())
def save(entry,title,values,slug_value=None,expected=200):
 return call(epath,'PATCH',{'expected_version':entry['version'],'title':title,'slug':slug_value or entry['draft']['slug'],'values':values},expected=expected,key=key())
v1={'summary':xss,'body':'# Welcome\n\n'+xss+'\n\nRead **more** at [our site](https://example.com) or [here](/c/x).','rating':4.5,'featured':True,'event_on':'2026-02-28','more':'https://example.com/more'}
entry=save(entry,'Welcome '+xss,v1)
assert entry['draft']['revision_number']==2
entry=save(entry,'Stale',v1)
stale={'expected_version':1,'title':'x','slug':slug,'values':{}}
call(epath,'PATCH',stale,expected=409,key=key())
# Publishing makes exactly the saved draft public.
retry=key();published=call(epath+'/publish','POST',{'expected_version':entry['version']},key=retry)
assert call(epath+'/publish','POST',{'expected_version':entry['version']},key=retry)==published
call(epath+'/publish','POST',{'expected_version':published['version']},expected=409,key=key())
entry=published
live=public(public_page)
assert set(live)=={'title','slug','published_at','fields'}
by={x['key']:x['value'] for x in live['fields']}
assert live['title']=='Stale' and by['summary']=={'kind':'text','text':xss} and by['rating']=={'kind':'number','number':4.5} and by['featured']=={'kind':'boolean','checked':True}
assert by['event_on']=={'kind':'date','date':'2026-02-28'} and by['more']=={'kind':'url','href':'https://example.com/more'}
# Markup remains inert text inside typed blocks; there is no HTML anywhere in the contract.
blocks=by['body']['blocks']
assert blocks[0]['kind']=='heading' and blocks[1]=={'kind':'paragraph','inlines':[{'text':xss,'bold':False,'italic':False,'code':False,'href':None}]}
links=[i['href'] for i in blocks[2]['inlines'] if i['href']];assert links==['https://example.com','/c/x']
# A stored unsafe link (older data, direct database writes) still never renders as a link.
sql("UPDATE cms.revisions SET content=jsonb_set(content,'{body}','\"[x](javascript:alert(1))\"') WHERE entry_id='"+entry['id']+"' AND number="+str(entry['published']['revision_number']))
assert all(i['href'] is None for i in public(public_page)['fields'][1]['value']['blocks'][0]['inlines'])
sql("UPDATE cms.revisions SET content=jsonb_set(content,'{body}',to_jsonb('"+v1['body'].replace("'","''")+"'::text)) WHERE entry_id='"+entry['id']+"' AND number="+str(entry['published']['revision_number']))
# Further edits stay private until published again.
entry=save(entry,'Second title',{**v1,'summary':'Unpublished words'})
assert entry['unpublished_changes'] and entry['published']['title']=='Stale'
assert b'Unpublished words' not in json.dumps(public(public_page)).encode() and b'Second title' not in json.dumps(public(site)).encode()
assert public(site)['entries'][0]['title']=='Stale'
preview=call(epath+'/preview');assert preview['title']=='Second title'
print('PASS: drafts are private, publish exposes exactly the saved revision, markup stays inert, unsafe links are refused and never render')

# Other readers: without draft access a member sees only what is published.
with without('cms.drafts.read'):
 reader=call(epath)
 assert reader['draft'] is None and reader['published']['title']=='Stale' and not reader['unpublished_changes']
 listed=call(base+'/cms/entries?limit=100')['items']
 assert [x['title'] for x in listed if x['id']==entry['id']]==['Stale'] and all('Second title'!=x['title'] for x in listed)
 for path in [epath+'/preview',epath+'/revisions']:call(path,expected=403)
 call(epath,'PATCH',{'expected_version':entry['version'],'title':'x','slug':slug,'values':{}},expected=403,key=key())
 call(epath+'/publish','POST',{'expected_version':entry['version']},expected=403,key=key())
draft_only=call(base+'/cms/entries','POST',{'type_id':content_type['id'],'title':'Hidden draft '+key(),'slug':'hidden-'+secrets.token_hex(3)},expected=201,key=key())
with without('cms.drafts.read'):
 call(base+'/cms/entries/'+draft_only['id'],expected=404)
 assert all(x['id']!=draft_only['id'] for x in call(base+'/cms/entries?limit=100')['items'])
 call(base+'/cms/entries?after='+draft_only['id'],expected=404)
with without('cms.read'):
 for path in [base+'/cms/types',base+'/cms/entries',base+'/cms/media',epath]:call(path,expected=403)
# Publishing is its own permission; editors can draft but not change what is live.
with without('cms.entries.publish'):
 call(epath+'/publish','POST',{'expected_version':entry['version']},expected=403,key=key())
 call(epath+'/unpublish','POST',{'expected_version':entry['version']},expected=403,key=key())
 call(epath+'/archive','POST',{'expected_version':entry['version'],'archived':True},expected=403,key=key())
 call(tpath+'/archive','POST',{'expected_version':content_type['version'],'archived':True},expected=403,key=key())
 entry=save(entry,'Second title',{**v1,'summary':'Edited without publishing right'})
 assert public(public_page)['title']=='Stale'
with without('cms.entries.edit'):
 save(entry,'x',v1,expected=403)
with without('cms.types.manage'):
 call(tpath,'PATCH',{'name':name,'fields':fields,'expected_version':content_type['version']},expected=403,key=key())
print('PASS: draft visibility and publish permission hold across reads, lists, previews, edits and archive')

# Addresses: drafts may share one; only one entry may be live at it.
rival=call(base+'/cms/entries','POST',{'type_id':content_type['id'],'title':'Rival','slug':slug,'values':{'summary':'Rival'}},expected=201,key=key())
call(base+'/cms/entries/'+rival['id']+'/publish','POST',{'expected_version':rival['version']},expected=409,key=key())
rival=call(base+'/cms/entries/'+rival['id'],'PATCH',{'expected_version':rival['version'],'title':'Rival','slug':'rival-'+secrets.token_hex(3),'values':{'summary':'Rival'}},key=key())
rival=call(base+'/cms/entries/'+rival['id']+'/publish','POST',{'expected_version':rival['version']},key=key())
# The address is fixed by publishing; a renamed draft does not move the live page.
assert len(public(site)['entries'])==2
print('PASS: one live entry per address, per content type')

# Media: Files stay private; only media named by a live revision has a public address.
call(base+'/cms/media','POST',{'file_id':f.workspace,'title':'x'},expected=404,key=key())
for content,media_type in [(b'<svg onload=alert(1)>','image/svg+xml'),(b'<script>alert(1)</script>','image/png'),(b'<html>','text/html'),(PNG,'image/jpeg')]:
 bad=upload(content,media_type)
 call(base+'/cms/media','POST',{'file_id':bad['id'],'title':'x'},expected=400,key=key())
private=upload(PNG,name='Private '+key())
status,_,_=raw(site+'/media/'+private['id']);assert status==404,'A file id is not a public address'
file=upload(PNG,name='Hero '+key())
retry=key();media=call(base+'/cms/media','POST',{'file_id':file['id'],'title':'Hero image','alt_text':'A hero'},expected=201,key=retry)
assert call(base+'/cms/media','POST',{'file_id':file['id'],'title':'Hero image','alt_text':'A hero'},expected=201,key=retry)==media
call(base+'/cms/media','POST',{'file_id':file['id'],'title':'Again'},expected=409,key=key())
mpath=base+'/cms/media/'+media['id'];murl=site+'/media/'+media['id']
assert base64.b64decode(call(mpath+'/content')['content_base64'])==PNG
with without('cms.media.manage'):
 call(base+'/cms/media','POST',{'file_id':private['id'],'title':'x'},expected=403,key=key())
 call(mpath,'PATCH',{'expected_version':media['version'],'title':'x'},expected=403,key=key())
with without('files.read'):
 call(base+'/cms/media','POST',{'file_id':private['id'],'title':'x'},expected=403,key=key())
# A file another member owns stays out of reach to an assigned-scope reader.
hidden_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('CMS hidden member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
theirs=call(base+'/files','POST',{'filename':'Theirs '+key(),'media_type':'image/png','content_base64':base64.b64encode(PNG).decode(),'owner_membership_id':hidden_member},expected=201,key=key())
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id="+owner_role+" AND permission_key='files.read'")
try:call(base+'/cms/media','POST',{'file_id':theirs['id'],'title':'x'},expected=404,key=key())
finally:sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id="+owner_role+" AND permission_key='files.read'")
assert raw(murl)[0]==404,'Unused media has no public address'
entry=save(entry,'Second title',{**v1,'hero':media['id']})
assert raw(murl)[0]==404,'Media named only by a draft has no public address'
assert 'hero' not in {x['key'] for x in public(public_page)['fields']}
assert any(x['key']=='hero' for x in call(epath+'/preview')['fields'])
entry=call(epath+'/publish','POST',{'expected_version':entry['version']},key=key())
assert public(public_page)['title']=='Second title'
status,body,headers=raw(murl)
assert status==200 and body==PNG and headers['Content-Type']=='image/png' and headers['X-Content-Type-Options']=='nosniff' and 'sandbox' in headers['Content-Security-Policy'] and headers['Content-Disposition']=='inline'
hero=next(x['value'] for x in public(public_page)['fields'] if x['key']=='hero')
assert hero=={'kind':'media','id':media['id'],'title':'Hero image','alt_text':'A hero','media_type':'image/png'},'Public media exposes no file identity'
assert raw(site+'/media/'+file['id'])[0]==404 and raw('/api/v1/public/cms/'+other_type['public_key']+'/media/'+media['id'])[0]==404,'Media is public only under the type that publishes it'
call(base+'/files/'+file['id']+'/content',authenticated=False,expected=401)
# Later Files metadata edits cannot change how the bytes are served.
current=call(base+'/files/'+file['id'])
current=call(base+'/files/'+file['id'],'PATCH',{'expected_version':current['version'],'filename':current['filename'],'media_type':'text/html'},key=key())
assert raw(murl)[2]['Content-Type']=='image/png'
# Archiving in either place withdraws the bytes at once and the page renders without them.
media=call(mpath+'/archive','POST',{'expected_version':media['version'],'archived':True},key=key())
assert raw(murl)[0]==404 and 'hero' not in {x['key'] for x in public(public_page)['fields']}
media=call(mpath+'/archive','POST',{'expected_version':media['version'],'archived':False},key=key())
assert raw(murl)[0]==200
current=call(base+'/files/'+file['id']+'/archive','POST',{'expected_version':current['version'],'archived':True},key=key())
assert raw(murl)[0]==404 and 'hero' not in {x['key'] for x in public(public_page)['fields']}
assert call(mpath)['file_archived']
blocked=save(entry,'Second title',{**v1,'hero':media['id'],'rating':5})
call(epath+'/publish','POST',{'expected_version':blocked['version']},expected=409,key=key())
current=call(base+'/files/'+file['id']+'/archive','POST',{'expected_version':current['version'],'archived':False},key=key())
assert raw(murl)[0]==200
# A live revision that no longer names the media stops serving it.
entry=save(blocked,'Second title',{**v1,'rating':5})
assert raw(murl)[0]==200
entry=call(epath+'/publish','POST',{'expected_version':entry['version']},key=key())
assert raw(murl)[0]==404,'Old revisions grant nothing'
print('PASS: media adoption needs Files access, bytes are checked, only live revisions publish them, Files stay private and archiving withdraws at once')

# Revision recovery: an earlier published revision comes back without rewriting history.
page=call(epath+'/revisions?limit=100')
numbers=[r['number'] for r in page['items']];assert numbers==sorted(numbers,reverse=True) and len(set(numbers))==len(numbers)
first_live=next(r for r in page['items'] if r['title']=='Stale')
assert first_live['last_published_at'] and first_live['values']['summary']==xss
before=call(epath);count=len(numbers)
retry=key();restored=call(epath+'/revisions/'+first_live['id']+'/restore','POST',{'expected_version':before['version']},key=retry)
assert call(epath+'/revisions/'+first_live['id']+'/restore','POST',{'expected_version':before['version']},key=retry)==restored
assert restored['dropped_fields']==[] and restored['entry']['draft']['title']=='Stale' and restored['entry']['draft']['revision_number']==max(numbers)+1
page=call(epath+'/revisions?limit=100');assert len(page['items'])==count+1 and page['items'][0]['restored_from_number']==first_live['number']
assert call(epath+'/revisions/'+first_live['id'])==first_live,'History is immutable'
assert public(public_page)['title']=='Second title','Restoring changes the draft, not the live page'
with without('cms.entries.edit'):
 call(epath+'/revisions/'+first_live['id']+'/restore','POST',{'expected_version':restored['entry']['version']},expected=403,key=key())
call(epath+'/revisions/'+first_live['id']+'/restore','POST',{'expected_version':restored['entry']['version']},expected=409,key=key())
entry=call(epath+'/publish','POST',{'expected_version':restored['entry']['version']},key=key())
assert public(public_page)['title']=='Stale'
# Published pages and revisions keep their own fields when the type changes.
kept=[x for x in fields if x['key']!='rating']+[{'key':'rating','label':'Rating words','kind':'text'}]
reshaped=call(tpath,'PATCH',{'name':name,'description':'Our pages','fields':[x for x in fields if x['key']!='rating'],'expected_version':call(tpath)['version']},key=key())
assert any(x['key']=='rating' for x in public(public_page)['fields']),'The live revision keeps the fields it was published with'
reshaped=call(tpath,'PATCH',{'name':name,'description':'Our pages','fields':kept,'expected_version':reshaped['version']},key=key())
current=call(epath)
assert 'rating' in current['draft']['values']
edited=save(current,'Stale',{'summary':'Reshaped'})
back=call(epath+'/revisions/'+first_live['id']+'/restore','POST',{'expected_version':edited['version']},key=key())
assert back['dropped_fields']==['rating'] and 'rating' not in back['entry']['draft']['values'],'A value whose field changed kind is reported, not coerced'
assert first_live['values']['rating']==4.5 and call(epath+'/revisions/'+first_live['id'])['values']['rating']==4.5
entry=back['entry']
print('PASS: restoring a revision recovers published content through the draft, keeps history immutable and reports fields that cannot return')

# Withdrawal: unpublish, archive and type archive close public addresses and retain data.
entry=call(epath+'/unpublish','POST',{'expected_version':entry['version']},key=key())
public(public_page,404);assert entry['published'] is None and entry['draft']
call(epath+'/unpublish','POST',{'expected_version':entry['version']},expected=409,key=key())
entry=call(epath+'/publish','POST',{'expected_version':entry['version']},key=key())
public(public_page)
entry=call(epath+'/archive','POST',{'expected_version':entry['version'],'archived':True},key=key())
public(public_page,404);assert entry['archived'] and entry['published'] is None
save(entry,'x',{},expected=409)
call(epath+'/publish','POST',{'expected_version':entry['version']},expected=409,key=key())
entry=call(epath+'/archive','POST',{'expected_version':entry['version'],'archived':False},key=key())
public(public_page,404)
entry=call(epath+'/publish','POST',{'expected_version':entry['version']},key=key())
content_type=call(tpath)
content_type=call(tpath+'/archive','POST',{'expected_version':content_type['version'],'archived':True},key=key())
public(public_page,404);public(site,404)
call(base+'/cms/entries','POST',{'type_id':content_type['id'],'title':'x','slug':'x'},expected=409,key=key())
content_type=call(tpath+'/archive','POST',{'expected_version':content_type['version'],'archived':False},key=key())
public(public_page)
# Disabling CMS closes staff and public routes, keeping every record for re-enabling.
module('cms',False)
public(public_page,404);public(site,404)
call(epath,expected=403);call(tpath,expected=403);call(epath+'/revisions',expected=403);call(mpath,expected=403)
upload(PNG)  # Shared Files remain usable without CMS.
module('cms',True)
assert public(public_page)['title']==call(epath)['published']['title']
public('/api/v1/public/cms/'+'x'*22,404);public('/api/v1/public/cms/short',404)
# Hostile addresses are refused as such rather than reaching the database.
public('/api/v1/public/cms/'+'x'*21+'%00',404);public(site+'/entries/a%00b',404);public(site+'/entries/%2e%2e',404);public(site+'?after=%00',400)
call(base+'/cms/entries?search=%00',expected=400)
call(base+'/cms/types','POST',{'name':'x','description':'a\u0000b'},expected=400,key=key())
print('PASS: unpublish, archive, type archive and application disablement close public addresses and retain records')

# An embedded form degrades gracefully while Forms is off, then reaches CRM.
assert not next(m for m in call(base+'/modules') if m['key']=='forms')['enabled']
form_fields=[{'key':'name','label':'Name','kind':'short_text','required':True},{'key':'email','label':'Email','kind':'email'}]
module('forms',True)
form=call(base+'/forms','POST',{'name':'CMS lead '+key(),'confirmation_message':'Thanks from the page.','draft':{'fields':form_fields}},expected=201,key=key())
fpath=base+'/forms/'+form['id']
module('forms',False)
entry=call(epath)
call(epath,'PATCH',{'expected_version':entry['version'],'title':'With form','slug':slug,'values':{'summary':'Join us','signup':form['id']}},expected=400,key=key())
module('forms',True)
with without('forms.read'):
 call(epath,'PATCH',{'expected_version':entry['version'],'title':'With form','slug':slug,'values':{'summary':'Join us','signup':form['id']}},expected=400,key=key())
entry=call(epath,'PATCH',{'expected_version':entry['version'],'title':'With form','slug':slug,'values':{'summary':'Join us','signup':form['id']}},key=key())
entry=call(epath+'/publish','POST',{'expected_version':entry['version']},key=key())
def signup():return next(x['value'] for x in public(public_page)['fields'] if x['key']=='signup')
assert signup()=={'kind':'form','public_key':None},'An unpublished form is not embedded'
form=call(fpath+'/publish','POST',{'expected_version':form['version']},key=key())
assert signup()=={'kind':'form','public_key':form['public_key']}
# Only the key reaches the page, so an unpublished form rename cannot leak through it.
assert 'name' not in signup()
form=call(fpath+'/accepting','POST',{'expected_version':form['version'],'accepting':False},key=key())
assert signup()['public_key'] is None
form=call(fpath+'/accepting','POST',{'expected_version':form['version'],'accepting':True},key=key())
module('forms',False)
page_off=public(public_page)
assert page_off['title']=='With form' and {'kind':'form','public_key':None} in [x['value'] for x in page_off['fields']],'The page still serves with Forms off'
call('/api/v1/public/forms/'+form['public_key'],expected=404,authenticated=False,headers=client())
# An existing choice survives editing while Forms is off.
entry=call(epath)
entry=call(epath,'PATCH',{'expected_version':entry['version'],'title':'With form','slug':slug,'values':{'summary':'Join us again','signup':form['id']}},key=key())
entry=call(epath+'/publish','POST',{'expected_version':entry['version']},key=key())
assert call(epath)['published']['values']['signup']==form['id']
module('forms',True);module('crm',True)
pipeline=call(base+'/crm/pipelines','POST',{'name':'CMS leads '+key()},expected=201,key=key())
stage=call(base+'/crm/stages','POST',{'name':'New '+key(),'parent_id':pipeline['id'],'outcome':'open'},expected=201,key=key())
recipe=call(fpath+'/recipe','PUT',{'mapping':{'name_fields':['name'],'email_field':'email','pipeline_id':pipeline['id'],'stage_id':stage['id'],'title_prefix':'Website lead'}},key=key())
call(fpath+'/recipe/activation','POST',{'expected_version':recipe['version'],'active':True},key=key())
embed=signup()['public_key'];assert embed==form['public_key']
email='cms.'+secrets.token_hex(4)+'@example.com'
receipt=call('/api/v1/public/forms/'+embed+'/submissions','POST',{'submission_key':secrets.token_hex(12),'answers':{'name':'Page Visitor','email':email}},expected=201,authenticated=False,headers=client())
assert receipt['confirmation_message']=='Thanks from the page.'
deadline=time.monotonic()+30
while True:
 run=call(fpath+'/submissions')['items'][0]['recipe_run']
 if run and run['state']=='completed':break
 assert time.monotonic()<deadline,run
 time.sleep(.2)
opportunity=call(base+'/crm/opportunities/'+run['opportunity_id'])
assert opportunity['title']=='Website lead: Page Visitor' and opportunity['pipeline_id']==pipeline['id']
contact=call(base+'/contacts/'+run['contact_id'])
assert any(i['value']==email for i in contact['identifiers'])
print('PASS: an embedded form degrades while unavailable, and a submission from published content produces the permitted CRM result')

# Cross-workspace: records and references never cross.
foreign=sql("INSERT INTO suite.workspaces(name) VALUES('CMS foreign fixture') RETURNING id").splitlines()[0]
foreign_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('CMS foreign member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
foreign_type=sql("INSERT INTO cms.content_types(workspace_id,name,fields,public_key) VALUES('"+foreign+"','Foreign','[]','"+secrets.token_urlsafe(16)[:22]+"') RETURNING id").splitlines()[0]
# A complete file with its bytes: backups verify every file against its content.
foreign_file=sql("WITH x AS (INSERT INTO files.files(workspace_id,owner_membership_id,filename,media_type,byte_length,sha256) VALUES('"+foreign+"','"+foreign_member+"','foreign.png','image/png',0,encode(pg_catalog.sha256(''),'hex')) RETURNING id,workspace_id,sha256,byte_length), y AS (INSERT INTO files.contents(workspace_id,file_id,sha256,byte_length,content) SELECT workspace_id,id,sha256,byte_length,'' FROM x) SELECT id FROM x").splitlines()[0]
foreign_media=sql("INSERT INTO cms.media(workspace_id,file_id,title,media_type) VALUES('"+foreign+"','"+foreign_file+"','Foreign','image/png') RETURNING id").splitlines()[0]
foreign_form=sql("INSERT INTO forms.forms(workspace_id,name,draft,public_key) VALUES('"+foreign+"','Foreign','{\"fields\":[]}','"+secrets.token_urlsafe(16)[:22]+"') RETURNING id").splitlines()[0]
foreign_entry=sql("INSERT INTO cms.entries(workspace_id,type_id,title,slug,draft_values) VALUES('"+foreign+"','"+foreign_type+"','Foreign','foreign','{}') RETURNING id").splitlines()[0]
foreign_base='/api/v1/workspaces/'+foreign
for path in ['/cms/types','/cms/entries','/cms/media','/cms/types/'+foreign_type,'/cms/entries/'+foreign_entry]:call(foreign_base+path,expected=403)
call(base+'/cms/types/'+foreign_type,expected=404);call(base+'/cms/entries/'+foreign_entry,expected=404);call(base+'/cms/media/'+foreign_media,expected=404)
call(base+'/cms/entries/'+foreign_entry+'/revisions',expected=404)
call(base+'/cms/entries','POST',{'type_id':foreign_type,'title':'x','slug':'x'},expected=404,key=key())
call(base+'/cms/media','POST',{'file_id':foreign_file,'title':'x'},expected=404,key=key())
entry=call(epath)
for values in [{'hero':foreign_media},{'signup':foreign_form}]:
 call(epath,'PATCH',{'expected_version':entry['version'],'title':'x','slug':slug,'values':values},expected=400,key=key())
assert raw(site+'/media/'+foreign_media)[0]==404
print('PASS: cross-workspace routes, records and references are denied')

# Business export covers CMS with its permissions.
catalog={c['collection']:c for c in call(base+'/exports/collections')}
assert catalog['cms_revisions']['read_permission']=='cms.drafts.read' and catalog['cms_entries']['dependent_read_permissions']==['cms.read']
def export(collection,expected=200,**body):return call(base+'/exports/page','POST',{'collection':collection,**body},expected=expected)
def every(collection,**body):
 items=[];after=None
 while True:
  result=export(collection,limit=2,**body,**({'after':after} if after else {}))
  items+=result['items']
  if not result['next']:return items
  after=result['next']['after']
exported=every('cms_entries')
mine=next(x for x in exported if x['id']==entry['id']);assert mine['draft'] and mine['published']
assert any(x['id']==content_type['id'] for x in every('cms_content_types'))
assert any(x['id']==media['id'] for x in every('cms_media'))
assert len(every('cms_revisions',parent_id=entry['id']))==len(call(epath+'/revisions?limit=100')['items'])
export('cms_revisions',400)
export('cms_revisions',404,parent_id='00000000-0000-0000-0000-000000000000')
with without('cms.drafts.read'):
 export('cms_entries',403);export('cms_revisions',403,parent_id=entry['id'])
 export('cms_content_types');export('cms_media')
with without('cms.read'):
 export('cms_content_types',403);export('cms_media',403);export('cms_entries',403)
module('cms',False)
export('cms_entries',403);export('cms_media',403)
module('cms',True)
print('PASS: business export pages CMS records with their permissions and module state')

# CLI and MCP share the same rules.
cli_type=f.cli(['cms','type-create','--idempotency-key',key()],{'name':'CLI '+key(),'fields':[{'key':'body','label':'Body','kind':'rich_text','required':True}]})
cli_entry=f.cli(['cms','entry-create','--idempotency-key',key()],{'type_id':cli_type['id'],'title':'From the CLI','slug':'from-cli','values':{'body':'**Hello**'}})
assert f.cli(['cms','entry-get',cli_entry['id']])==call(base+'/cms/entries/'+cli_entry['id'])
assert f.cli(['cms','preview',cli_entry['id']])['title']=='From the CLI'
cli_entry=f.cli(['cms','publish',cli_entry['id'],'--idempotency-key',key(),'--expected-version',str(cli_entry['version'])])
assert cli_entry['published']['slug']=='from-cli'
assert f.cli(['cms','revisions',cli_entry['id']])['items'][0]['number']==1
assert f.cli(['cms','type-list','--limit','100'])['items']==call(base+'/cms/types?limit=100&archived=false')['items']
assert f.cli(['cms','media-get',media['id']])==call(mpath)
assert base64.b64decode(f.cli(['cms','media-content',media['id']])['content_base64'])==PNG
import subprocess
from dev import ROOT
anonymous=json.loads(subprocess.check_output([str(ROOT/'target/debug/looski'),'public-content','--api-url',f.base,cli_type['public_key'],'entry','from-cli'],text=True))
assert anonymous['fields'][0]['value']['blocks'][0]['inlines'][0]['bold']
assert json.loads(subprocess.check_output([str(ROOT/'target/debug/looski'),'public-content','--api-url',f.base,cli_type['public_key'],'index'],text=True))['entries'][0]['slug']=='from-cli'
def mcp(name,**arguments):
 result=f.mcp(name,{'workspace_id':f.workspace,**arguments})
 return result,json.loads(result['content'][0]['text'])
result,value=mcp('get_cms_entry',entry_id=cli_entry['id']);assert not result.get('isError') and value==call(base+'/cms/entries/'+cli_entry['id'])
result,draft=mcp('create_cms_entry',idempotency_key=key(),entry={'type_id':cli_type['id'],'title':'From MCP','slug':'from-mcp','values':{'body':'x'}})
assert not result.get('isError') and draft['draft']['title']=='From MCP'
result,value=mcp('update_cms_entry',entry_id=draft['id'],idempotency_key=key(),entry={'expected_version':draft['version'],'title':'From MCP 2','slug':'from-mcp','values':{'body':'[x](javascript:alert(1))'}})
assert result.get('isError')
result,value=mcp('publish_cms_entry',entry_id=draft['id'],idempotency_key=key(),expected_version=draft['version']);assert not result.get('isError') and value['published']['slug']=='from-mcp'
result,value=mcp('list_cms_revisions',entry_id=draft['id']);assert value['items'][0]['number']==1
assert public('/api/v1/public/cms/'+cli_type['public_key']+'/entries/from-mcp')['title']=='From MCP'
result,value=mcp('restore_cms_revision',entry_id=draft['id'],revision_id=call(base+'/cms/entries/'+draft['id']+'/revisions')['items'][0]['id'],idempotency_key=key(),expected_version=value and call(base+'/cms/entries/'+draft['id'])['version']);assert result.get('isError'),'Same draft refuses restore'
result,value=mcp('list_cms_media');assert any(x['id']==media['id'] for x in value['items'])
with without('cms.entries.publish'):
 result,_=mcp('unpublish_cms_entry',entry_id=draft['id'],idempotency_key=key(),expected_version=call(base+'/cms/entries/'+draft['id'])['version']);assert result.get('isError')
print('PASS: CLI and MCP share CMS rules, including denial and the anonymous public read')

# Audit and retained records.
for action in ['cms.type_saved','cms.entry_saved','cms.entry_published','cms.entry_unpublished','cms.entry_archived','cms.revision_restored','cms.media_registered','cms.media_archived']:
 assert int(sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"' AND action='"+action+"'"))>0,action
assert sql("SELECT has_column_privilege('looski_runtime','cms.revisions','content','UPDATE')")=='f'
assert sql("SELECT has_table_privilege('looski_runtime','cms.revisions','DELETE')")=='f' and sql("SELECT has_table_privilege('anon','cms.entries','SELECT')")=='f'
for k,enabled in initial.items():module(k,enabled)
print('PASS: CMS audit, immutable revision grants and application selection restored')
