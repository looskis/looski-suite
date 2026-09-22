#!/usr/bin/env python3
"""Optional CRM and independent opportunity/deal lifecycle acceptance."""
import secrets
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='CRM '+secrets.token_hex(6)
def key():return secrets.token_hex(16)
modules=call(base+'/modules');crm=next(m for m in modules if m['key']=='crm')
if crm['enabled']:
 crm=call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':crm['version']},key=key())
call(base+'/crm/opportunities',expected=403)
assert call(base+'/contacts')['items'] is not None,'Contacts survives CRM disable'
crm=call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':crm['version']},key=key())
def definition(kind,**body):return call(base+'/crm/'+kind,'POST',{'name':name,**body},expected=201,key=key())
pipeline=definition('pipelines');stage=definition('stages',parent_id=pipeline['id'],outcome='open')
won=definition('stages',parent_id=pipeline['id'],outcome='won',name=name+' Won')
category=definition('categories');status=definition('statuses',parent_id=category['id'])
def opp_body(**changes):return {'title':name,'pipeline_id':pipeline['id'],'stage_id':stage['id'],'expected_value':'1234.50','currency_code':'USD',**changes}
opportunity=call(base+'/crm/opportunities','POST',opp_body(),expected=201,key=key())
assert opportunity['outcome']=='open'
def deal_body(**changes):return {'title':name,'category_id':category['id'],'status_id':status['id'],**changes}
deals=[call(base+'/crm/deals','POST',deal_body(originating_opportunity_id=opportunity['id']),expected=201,key=key()) for _ in range(2)]
direct=call(base+'/crm/deals','POST',deal_body(),expected=201,key=key())
assert direct['originating_opportunity_id'] is None
assert all(d['originating_opportunity_id']==opportunity['id'] for d in deals)
# Renaming a definition preserves stable links and never creates a transition.
history=call(base+'/crm/opportunities/'+opportunity['id']+'/history')
call(base+'/crm/stages/'+stage['id'],'PATCH',{'name':name+' Qualified','expected_version':stage['version']},key=key())
assert call(base+'/crm/opportunities/'+opportunity['id']+'/history')==history
opportunity=call(base+'/crm/opportunities/'+opportunity['id'],'PATCH',{'expected_version':opportunity['version'],**opp_body(stage_id=won['id'])},key=key())
assert opportunity['outcome']=='won'
assert call(base+'/crm/deals/'+deals[0]['id'])['status_id']==status['id']
crm=call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':crm['version']},key=key())
call(base+'/crm/deals/'+direct['id'],expected=403)
crm=call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':crm['version']},key=key())
assert call(base+'/crm/deals/'+direct['id'])['id']==direct['id']
print('PASS: module selection, shared Contacts independence, separate opportunity/deal lifecycles, origin cardinality, definition rename versus transition and retained disabled data')
# Canonical numeric(20,6): no floating-point conversion, rounding or overflow.
from decimal import Decimal
for amount in ['0.000001','99999999999999.999999']:
 result=call(base+'/crm/opportunities','POST',opp_body(expected_value=amount),expected=201,key=key())
 assert Decimal(result['expected_value'])==Decimal(amount)
for amount in ['0.0000001','100000000000000','-1','NaN','1e3']:
 call(base+'/crm/opportunities','POST',opp_body(expected_value=amount),expected=400,key=key())
call(base+'/crm/opportunities','POST',opp_body(currency_code='ZZZ'),expected=400,key=key())
call(base+'/crm/opportunities','POST',opp_body(expected_close_on='2026-02-29'),expected=400,key=key())
call(base+'/crm/deals','POST',deal_body(starts_on='2026-03-01',ends_on='2026-02-28'),expected=400,key=key())
print('PASS: exact six-decimal estimates, amount boundaries, supported currency catalog and calendar validation')
# Version conflicts, retry input binding and concurrent edits.
import json
from urllib.request import Request,urlopen
from urllib.error import HTTPError
from concurrent.futures import ThreadPoolExecutor
body=deal_body(title=name+' retry');retry_key=key()
replayed=call(base+'/crm/deals','POST',body,expected=201,key=retry_key)
assert call(base+'/crm/deals','POST',body,expected=201,key=retry_key)==replayed
call(base+'/crm/deals','POST',{**body,'title':'Different'},expected=409,key=retry_key)
def race_edit(title):
 req=Request(f.base+base+'/crm/deals/'+replayed['id'],method='PATCH',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()},data=json.dumps({'expected_version':replayed['version'],**deal_body(title=title)}).encode())
 try:response=urlopen(req,timeout=20)
 except HTTPError as failure:response=failure
 with response:return response.status
with ThreadPoolExecutor(max_workers=2) as pool:assert sorted(pool.map(race_edit,[name+' race A',name+' race B']))==[200,409]
# Archived definitions retain existing links but cannot receive new assignments.
current_stage=call(base+'/crm/stages/'+stage['id'])
call(base+'/crm/stages/'+stage['id'],'PATCH',{'name':current_stage['name'],'archived':True,'expected_version':current_stage['version']},key=key())
call(base+'/crm/opportunities','POST',opp_body(),expected=409,key=key())
call(base+'/crm/stages/'+won['id'],'PATCH',{'name':won['name'],'outcome':'lost','expected_version':won['version']},expected=409,key=key())
other_pipeline=definition('pipelines',name=name+' Other pipeline')
call(base+'/crm/opportunities','POST',opp_body(pipeline_id=other_pipeline['id'],stage_id=won['id']),expected=400,key=key())
# Every origin projection/filter and original-result retry rechecks visibility.
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('CRM scoped fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
hidden=call(base+'/crm/opportunities','POST',opp_body(stage_id=won['id'],owner_membership_id=other),expected=201,key=key())
origin_body=deal_body(originating_opportunity_id=hidden['id']);origin_key=key()
visible_deal=call(base+'/crm/deals','POST',origin_body,expected=201,key=origin_key)
hidden_deal=call(base+'/crm/deals','POST',deal_body(owner_membership_id=other),expected=201,key=key())
owner_role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
def scope(action,value):f.sql("UPDATE suite.role_permissions SET record_scope='"+value+"' WHERE role_id='"+owner_role+"' AND permission_key='"+action+"'")
scope('crm.opportunities.read','assigned')
try:
 assert call(base+'/crm/deals/'+visible_deal['id'])['originating_opportunity_id'] is None
 call(base+'/crm/deals?originating_opportunity_id='+hidden['id'],expected=404)
 call(base+'/crm/deals','POST',origin_body,expected=404,key=key())
 call(base+'/crm/deals','POST',origin_body,expected=404,key=origin_key)
finally:scope('crm.opportunities.read','workspace')
scope('crm.deals.update','assigned')
try:call(base+'/crm/deals/'+hidden_deal['id'],'PATCH',{'expected_version':1,**deal_body()},expected=404,key=key())
finally:scope('crm.deals.update','workspace')
scope('crm.deals.read','assigned')
try:
 call(base+'/crm/deals/'+hidden_deal['id'],expected=404)
 assert all(d['owner_membership_id']!=other for d in call(base+'/crm/deals?search='+name.replace(' ','%20'))['items'])
finally:scope('crm.deals.read','workspace')
# Atomic audit/history rollback and real foreign-workspace definitions.
foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('CRM foreign fixture') RETURNING id").splitlines()[0]
foreign_pipeline=f.sql("INSERT INTO crm.opportunity_pipelines(workspace_id,name) VALUES('"+foreign+"','Foreign') RETURNING id").splitlines()[0]
foreign_stage=f.sql("INSERT INTO crm.opportunity_stages(workspace_id,pipeline_id,name,outcome) VALUES('"+foreign+"','"+foreign_pipeline+"','Foreign','open') RETURNING id").splitlines()[0]
call(base+'/crm/opportunities','POST',opp_body(pipeline_id=foreign_pipeline,stage_id=foreign_stage),expected=404,key=key())
count=f.sql('SELECT count(*) FROM crm.deals');history_count=f.sql('SELECT count(*) FROM crm.deal_status_changes');f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:call(base+'/crm/deals','POST',deal_body(),expected=503,key=key())
finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert f.sql('SELECT count(*) FROM crm.deals')==count and f.sql('SELECT count(*) FROM crm.deal_status_changes')==history_count
# A disabled module also denies replay of previously successful operations.
crm=call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':crm['version']},key=key())
try:call(base+'/crm/deals','POST',body,expected=403,key=retry_key)
finally:crm=call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':crm['version']},key=key())
assert f.cli(['crm','deal',direct['id']])['id']==direct['id']
assert not f.mcp('crm_opportunity_history',{'workspace_id':f.workspace,'id':opportunity['id'],'query':{}}).get('isError')
print('PASS: CRM retries, concurrent edits, definition lifecycle, hidden origin projections/filter/replay, scoped writes, foreign definitions, rollback and CLI/MCP')
# SSR forms preserve workflow origins, rejected drafts and stale versions.
from html.parser import HTMLParser
from urllib.parse import urlencode
class Forms(HTMLParser):
 def __init__(self,html):
  super().__init__();self.forms=[];self.current=None;self.select=None;self.textarea=None;self.feed(html)
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='form':self.current={'action':attrs.get('action',''),'method':attrs.get('method','GET').upper(),'values':{}};self.forms.append(self.current)
  if self.current is None:return
  name=attrs.get('name')
  if tag=='input' and name and attrs.get('type') not in ['submit','checkbox','radio']:self.current['values'][name]=attrs.get('value','')
  if tag=='input' and name and 'checked' in attrs:self.current['values'][name]=attrs.get('value','on')
  if tag=='select':self.select=name
  if tag=='option' and self.select:
   if self.select not in self.current['values'] or 'selected' in attrs:self.current['values'][self.select]=attrs.get('value','')
  if tag=='textarea':self.textarea=name;self.current['values'][name]=''
 def handle_data(self,data):
  if self.current is not None and self.textarea:self.current['values'][self.textarea]+=data
 def handle_endtag(self,tag):
  if tag=='form':self.current=None
  if tag=='select':self.select=None
  if tag=='textarea':self.textarea=None
 def form(self,field):return next(form for form in self.forms if field in form['values'])['values']
def browser(path,form=None,expected=200):
 request=Request(f.base+path,data=None if form is None else urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})
 try:response=f.browser.open(request,timeout=20)
 except HTTPError as failure:response=failure
 with response:
  html=response.read().decode();assert response.status==expected,(path,response.status,html[:500]);return response.geturl(),Forms(html),html
web='/workspaces/'+f.workspace+'/crm'
_,forms,_=browser(web+'/configuration/categories')
url,forms,_=browser(web+'/configuration/categories',{**forms.form('idempotency_key'),'name':name+' browser category','position':'0'})
web_category=url.rsplit('/',1)[1]
_,forms,_=browser(web+'/configuration/categories/'+web_category,{**forms.form('idempotency_key'),'name':name+' renamed category','position':'2'})
assert call(base+'/crm/categories/'+web_category)['name']==name+' renamed category'
_,forms,_=browser(web+'/configuration/statuses')
status_draft={**forms.form('idempotency_key'),'name':' ','parent_id':web_category,'position':'3'}
_,forms,_=browser(web+'/configuration/statuses',status_draft,400)
assert forms.form('idempotency_key')['parent_id']==web_category
assert forms.form('idempotency_key')['idempotency_key']==status_draft['idempotency_key']
url,_,_=browser(web+'/configuration/statuses',{**forms.form('idempotency_key'),'name':name+' browser status'})
web_status=call(base+'/crm/statuses/'+url.rsplit('/',1)[1]);assert web_status['parent_id']==web_category
_,forms,_=browser(web+'/opportunities?parent='+pipeline['id'])
url,forms,_=browser(web+'/opportunities?parent='+pipeline['id'],{**forms.form('idempotency_key'),'title':name+' browser opportunity','parent_id':pipeline['id'],'state_id':won['id'],'notes':'Browser origin','amount':'1.000001','currency':'USD'})
web_opportunity=url.rsplit('/',1)[1]
_,forms,_=browser(web+'/deals?origin='+web_opportunity)
workflow_form=forms.form('parent');assert workflow_form['origin']==web_opportunity
_,forms,_=browser(web+'/deals?'+urlencode({**workflow_form,'parent':web_category}))
draft={**forms.form('idempotency_key'),'title':name+' retained draft','notes':'Retain these notes & symbols','amount':'3.000001','currency':'ZZZ','date1':'2026-10-12','date2':'2026-10-13','owner':other,'parent_id':web_category,'state_id':web_status['id']}
assert draft['origin']==web_opportunity
path=web+'/deals?parent='+web_category+'&origin='+web_opportunity
_,forms,_=browser(path,draft,400)
retained=forms.form('idempotency_key')
for field,value in draft.items():assert retained.get(field)==value,(field,retained.get(field),value)
url,forms,_=browser(path,{**retained,'currency':'USD'})
web_deal=url.rsplit('/',1)[1];saved=call(base+'/crm/deals/'+web_deal)
assert saved['originating_opportunity_id']==web_opportunity and saved['owner_membership_id']==other
edit=forms.form('title')
call(base+'/crm/deals/'+web_deal,'PATCH',{'expected_version':saved['version'],**deal_body(title=name+' concurrent winner',category_id=web_category,status_id=web_status['id'])},key=key())
_,forms,html=browser(web+'/deals/'+web_deal+'?/save',{**edit,'title':name+' conflicting draft','notes':'Preserved after conflict'},409)
assert forms.form('title')['expected_version']==edit['expected_version']
assert forms.form('title')['title']==name+' conflicting draft' and forms.form('title')['idempotency_key']==edit['idempotency_key']
assert 'Discard draft and reload saved values' in html
assert call(base+'/crm/deals/'+web_deal)['title']==name+' concurrent winner'
# Archive and restore are explicit, versioned and do not create fake status transitions.
history=call(base+'/crm/deals/'+web_deal+'/history')
for archived in [True,False]:
 _,forms,_=browser(web+'/deals/'+web_deal)
 browser(web+'/deals/'+web_deal+'?/archive',forms.form('archived'))
 assert call(base+'/crm/deals/'+web_deal)['archived']==archived
assert call(base+'/crm/deals/'+web_deal+'/history')==history
print('PASS: browser configuration, opportunity/deal creation, origin across workflow navigation, retained invalid drafts, stale edit protection and archive/restore')
