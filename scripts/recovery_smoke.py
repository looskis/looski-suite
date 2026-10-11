#!/usr/bin/env python3
"""Portable snapshot and isolated restore rehearsal on an explicit fixture stack."""
import argparse
import json
import secrets
from pathlib import Path
from integration_fixture import Fixture
from recovery import Installation,create,verify
from recovery_restore import restore
import recovery_archive as encryption

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--project',required=True)
parser.add_argument('--port',required=True,type=int)
parser.add_argument('--archive',type=Path)
parser.add_argument('--existing-target',type=Path)
args=parser.parse_args()
assert args.project.endswith('-identity-check')
f=Fixture();i=Installation(f.config)
directory=f.directory/'recovery-fixture';directory.mkdir(mode=0o700,exist_ok=True)
identity=directory/'identity'
recipient=encryption.recipient(identity) if identity.exists() else encryption.generate_identity(identity)
if args.archive:
 archive=args.archive
 job=None
else:
 archive=directory/(args.project+'.age')
 # Seed the representative reads even on a newly bootstrapped installation.
 def fixture_key():return secrets.token_hex(16)
 base_path=f'/api/v1/workspaces/{f.workspace}'
 crm_module=next(m for m in f.call(base_path+'/modules') if m['key']=='crm')
 if not crm_module['enabled']:f.call(base_path+'/modules/crm','PATCH',{'enabled':True,'expected_version':crm_module['version']},key=fixture_key())
 def seed(path,body):return f.call(base_path+path,'POST',body,expected=201,key=fixture_key())
 seed('/contacts',{'display_name':'Recovery contact '+fixture_key()})
 seed('/organizations',{'name':'Recovery organization '+fixture_key()})
 pipeline=seed('/crm/pipelines',{'name':'Recovery pipeline '+fixture_key()})
 stage=seed('/crm/stages',{'name':'New','parent_id':pipeline['id'],'outcome':'open'})
 seed('/crm/opportunities',{'title':'Recovery opportunity','pipeline_id':pipeline['id'],'stage_id':stage['id']})
 category=seed('/crm/categories',{'name':'Recovery category '+fixture_key()})
 status=seed('/crm/statuses',{'name':'Active','parent_id':category['id']})
 seed('/crm/deals',{'title':'Recovery deal','category_id':category['id'],'status_id':status['id']})
 # Published content and its media must come back readable by anyone.
 cms_module=next(m for m in f.call(f'/api/v1/workspaces/{f.workspace}/modules') if m['key']=='cms')
 if not cms_module['enabled']:f.call(f'/api/v1/workspaces/{f.workspace}/modules/cms','PATCH',{'enabled':True,'expected_version':cms_module['version']},key=fixture_key())
 cms_base=f'/api/v1/workspaces/{f.workspace}/cms'
 png=b'\x89PNG\r\n\x1a\nrecovery-'+secrets.token_hex(8).encode()
 cms_file=f.call(f'/api/v1/workspaces/{f.workspace}/files','POST',{'filename':'Recovery media '+fixture_key(),'media_type':'image/png','content_base64':__import__('base64').b64encode(png).decode()},expected=201,key=fixture_key())
 cms_media=f.call(cms_base+'/media','POST',{'file_id':cms_file['id'],'title':'Recovery media'},expected=201,key=fixture_key())
 cms_type=f.call(cms_base+'/types','POST',{'name':'Recovery pages','fields':[{'key':'body','label':'Body','kind':'rich_text'},{'key':'hero','label':'Hero','kind':'media'}]},expected=201,key=fixture_key())
 cms_entry=f.call(cms_base+'/entries','POST',{'type_id':cms_type['id'],'title':'Recovered page','slug':'recovered','values':{'body':'Published words','hero':cms_media['id']}},expected=201,key=fixture_key())
 cms_entry=f.call(cms_base+'/entries/'+cms_entry['id']+'/publish','POST',{'expected_version':cms_entry['version']},key=fixture_key())
 cms_entry=f.call(cms_base+'/entries/'+cms_entry['id'],'PATCH',{'expected_version':cms_entry['version'],'title':'Unpublished change','slug':'recovered','values':{'body':'Draft words','hero':cms_media['id']}},key=fixture_key())
 (directory/(args.project+'-cms.json')).write_text(json.dumps({'public_key':cms_type['public_key'],'entry':cms_entry['id'],'media':cms_media['id'],'png':png.hex()}))
 # A board keeps its places, shared tasks and comments.
 projects_module=next(m for m in f.call(f'/api/v1/workspaces/{f.workspace}/modules') if m['key']=='projects')
 if not projects_module['enabled']:f.call(f'/api/v1/workspaces/{f.workspace}/modules/projects','PATCH',{'enabled':True,'expected_version':projects_module['version']},key=fixture_key())
 board_project=f.call(f'/api/v1/workspaces/{f.workspace}/projects','POST',{'name':'Recovered delivery'},expected=201,key=fixture_key())
 board_path=f'/api/v1/workspaces/{f.workspace}/projects/'+board_project['id']
 board_cards=[f.call(board_path+'/cards','POST',{'task':{'title':title}},expected=201,key=fixture_key())['id'] for title in ['Recovered one','Recovered two']]
 f.call(board_path+'/cards/'+board_cards[0]+'/comments','POST',{'body':'Recovered comment'},expected=201,key=fixture_key())
 (directory/(args.project+'-projects.json')).write_text(json.dumps({'project':board_project['id'],'cards':board_cards}))
 # A restricted collection keeps its members, documents, immutable history and search index.
 knowledge_module=next(m for m in f.call(f'/api/v1/workspaces/{f.workspace}/modules') if m['key']=='knowledge')
 if not knowledge_module['enabled']:f.call(f'/api/v1/workspaces/{f.workspace}/modules/knowledge','PATCH',{'enabled':True,'expected_version':knowledge_module['version']},key=fixture_key())
 knowledge_base=f'/api/v1/workspaces/{f.workspace}/knowledge'
 shelf=f.call(knowledge_base+'/collections','POST',{'name':'Recovered shelf','restricted':True},expected=201,key=fixture_key())
 # Repeated rehearsals share this fixture; each searches for its own word.
 recoverable='recoverable'+secrets.token_hex(6)
 sop=f.call(knowledge_base+'/documents','POST',{'collection_id':shelf['id'],'title':'Recovered procedure','body':f'Original {recoverable} text','kind':'sop'},expected=201,key=fixture_key())
 sop=f.call(knowledge_base+'/documents/'+sop['id'],'PATCH',{'expected_version':sop['version'],'title':sop['title'],'body':'Later wording','kind':'sop'},key=fixture_key())
 first_revision=f.call(knowledge_base+'/documents/'+sop['id']+'/revisions')['items'][1]
 sop=f.call(knowledge_base+'/documents/'+sop['id']+'/revisions/'+first_revision['id']+'/restore','POST',{'expected_version':sop['version']},key=fixture_key())
 (directory/(args.project+'-knowledge.json')).write_text(json.dumps({'collection':shelf['id'],'document':sop['id'],'version':sop['version'],'word':recoverable}))
 # Helpdesk retains public/private history, revocation and the hashed ticket code.
 helpdesk_base=f'/api/v1/workspaces/{f.workspace}/helpdesk'
 helpdesk_module=next(m for m in f.call(f'/api/v1/workspaces/{f.workspace}/modules') if m['key']=='helpdesk')
 if not helpdesk_module['enabled']:f.call(f'/api/v1/workspaces/{f.workspace}/modules/helpdesk','PATCH',{'enabled':True,'expected_version':helpdesk_module['version']},key=fixture_key())
 hd_settings=f.call(helpdesk_base+'/settings')
 hd_settings=f.call(helpdesk_base+'/settings','PUT',{'accepting':True,'expected_version':hd_settings['version']},key=fixture_key())
 hd_code=secrets.token_hex(32)
 hd_ticket=f.call('/api/v1/public/helpdesk/intake/'+hd_settings['public_key'],'POST',{'subject':'Recovered support','requester_name':'Recovery requester','body':'Original customer question'},authenticated=False,headers={'X-Ticket-Key':hd_code})
 hd_path=helpdesk_base+'/tickets/'+hd_ticket['id']
 f.call(hd_path+'/messages','POST',{'body':'Private recovery note','internal':True},key=fixture_key())
 f.call(hd_path+'/messages','POST',{'body':'Public recovery answer'},key=fixture_key())
 hd_revoked_code=secrets.token_hex(32)
 hd_revoked=f.call('/api/v1/public/helpdesk/intake/'+hd_settings['public_key'],'POST',{'subject':'Revoked before recovery','requester_name':'Recovery requester','body':'Revoked request'},authenticated=False,headers={'X-Ticket-Key':hd_revoked_code})
 f.call(helpdesk_base+'/tickets/'+hd_revoked['id'],'PATCH',{'expected_version':1,'status':'open','archived':False,'revoke_customer_access':True},key=fixture_key())
 hd_marker=directory/(args.project+'-helpdesk.json');hd_marker.write_text(json.dumps({'ticket':hd_ticket['id'],'code':hd_code,'revoked_ticket':hd_revoked['id'],'revoked_code':hd_revoked_code}));hd_marker.chmod(0o600)
 # Time and versioned appointment intake survive the same portable snapshot.
 import datetime
 for selected in ['time','scheduling','forms']:
  module=next(m for m in f.call(base_path+'/modules') if m['key']==selected)
  if not module['enabled']:f.call(base_path+'/modules/'+selected,'PATCH',{'enabled':True,'expected_version':module['version']},key=fixture_key())
 tracked=f.call(base_path+'/time/entries','POST',{'description':'Recovered exact time','work_on':'2026-11-01','billable':True,'duration_ms':3456789},key=fixture_key())
 budget=f.call(base_path+'/time/budgets','POST',{'name':'Recovered budget','from_on':'2026-11-01','through_on':'2026-11-30','budget_ms':7200000,'billable_only':True,'archived':False},key=fixture_key())
 intake=f.call(base_path+'/forms','POST',{'name':'Recovered booking intake','draft':{'fields':[{'key':'detail','label':'Detail','kind':'long_text','required':True}]}},expected=201,key=fixture_key())
 intake=f.call(base_path+'/forms/'+intake['id']+'/publish','POST',{'expected_version':intake['version']},key=fixture_key())
 appointment_type=f.call(base_path+'/scheduling/types','POST',{'settings':{'name':'Recovered appointment','timezone':'UTC','duration_minutes':15,'weekly':[{'weekday':d,'start':0,'end':1440} for d in range(1,8)],'accepting':True,'form_id':intake['id'],'reminder_minutes':60}},key=fixture_key())
 day=(datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(days=4)).date().isoformat()
 slots=f.call('/api/v1/public/scheduling/types/'+appointment_type['public_key']+'/slots?date='+day,authenticated=False)
 booking_code=secrets.token_hex(32)
 booked=f.call('/api/v1/public/scheduling/types/'+appointment_type['public_key']+'/book','POST',{'starts_at':slots[0]['starts_at'],'attendee_name':'Recovery attendee','attendee_email':'recovery@example.invalid','form_version_id':intake['published_version']['id'],'answers':{'detail':'Recovered versioned answers'}},authenticated=False,headers={'X-Booking-Key':booking_code})
 booking=f.call(base_path+'/scheduling/appointments/'+booked['id'])
 marker=directory/(args.project+'-time-scheduling.json');marker.write_text(json.dumps({'time':tracked['id'],'budget':budget['id'],'booking':booking['id'],'code':booking_code,'submission':booking['submission_id'],'starts_at':booking['starts_at']}));marker.chmod(0o600)
 original=set(i.running());paused=['worker','web','auth']
 i.run('stop','worker')
 try:
  role=f.call(f'/api/v1/workspaces/{f.workspace}/access/roles','POST',{'name':'Recovery '+secrets.token_hex(6),'grants':[{'action':'workspace.read','scope':'workspace'}]},expected=201,key=secrets.token_hex(16))
  inv=f.call(f'/api/v1/workspaces/{f.workspace}/access/invitations','POST',{'email':'recovery-'+secrets.token_hex(6)+'@example.invalid','role_ids':[role['id']]},expected=201,key=secrets.token_hex(16))
  job=f.call(f'/api/v1/workspaces/{f.workspace}/access/invitations/'+inv['invitation']['id']+'/send','POST',{'expected_version':1},expected=202,key=secrets.token_hex(16))
  i.run('stop','web','auth')
  result=create(i,archive,recipient)
  assert not set(paused)&set(i.running()),'Backup started originally stopped services'
  print('PASS: portable backup includes initially stopped Auth/web/worker',json.dumps(result),flush=True)
 finally:
  i.run('start','--wait','--wait-timeout','60',*[s for s in paused if s in original])
manifest=verify(archive,identity)
assert manifest['metadata']['table_counts']['pgmq.q_looski_jobs']>0
print('PASS: authenticated payload includes live queue state and exact service images',flush=True)
result={'target':str(args.existing_target),'report':str(args.existing_target/'recovery-report.json')} if args.existing_target else restore(archive,identity,args.project,args.port)
print('PASS: isolated database, Files and queue restore',json.dumps(result),flush=True)
report=json.loads(Path(result['report']).read_text())
if job is None:job=next(j for j in report['quarantined_jobs'] if j['kind']=='workspace.invitation_email')
assert any(j['id']==job['id'] for j in report['quarantined_jobs'])
# Credential data stays in memory and is never part of diagnostic output.
from recovery_restore import preview
restored=preview(Path(result['target']))
from urllib.request import Request,urlopen,build_opener,HTTPCookieProcessor
from urllib.parse import urlencode
from urllib.error import HTTPError
import http.cookiejar,base64,hashlib
url=restored['url'];owner=json.loads((f.directory/'identity-fixture.json').read_text())
jar=http.cookiejar.CookieJar();browser=build_opener(HTTPCookieProcessor(jar))
with browser.open(Request(url+'/login',data=urlencode({'email':owner['email'],'password':owner['password']}).encode(),headers={'Origin':url,'Accept':'text/html'}),timeout=30) as response:assert response.status==200
new_token=next(c.value for c in jar if c.name=='looski_access')
def api(path,token=new_token,expected=200):
 try:r=urlopen(Request(url+path,headers={'Authorization':'Bearer '+token}),timeout=30)
 except HTTPError as error:r=error
 with r:
  assert r.status==expected,(path,r.status,expected)
  return json.loads(r.read())
api('/api/v1/workspaces',f.token,401)
workspaces=api('/api/v1/workspaces');assert any(w['id']==f.workspace for w in workspaces)
base='/api/v1/workspaces/'+f.workspace
for resource in ['contacts','organizations','crm/opportunities','crm/deals','tasks','files']:
 response=api(base+'/'+resource)
 assert response['items'],resource
files=api(base+'/files')['items'];content=api(base+'/files/'+files[0]['id']+'/content')
assert hashlib.sha256(base64.b64decode(content['content_base64'])).hexdigest()==content['file']['sha256']
assert api(base+'/jobs/'+job['id'])['state']=='blocked'
for path in ['/workspaces',f'/workspaces/{f.workspace}/contacts',f'/workspaces/{f.workspace}/files']:
 with browser.open(url+path,timeout=30) as response:assert response.status==200
print('PASS: restored password login, stable workspace identity, CRM/Tasks/Files reads, old-token denial and quarantined invitation; isolated loopback browser works',flush=True)
cms_marker=directory/(args.project+'-cms.json')
if cms_marker.exists():
 seeded=json.loads(cms_marker.read_text());site='/api/v1/public/cms/'+seeded['public_key']
 page=api(site+'/entries/recovered');assert page['title']=='Recovered page' and page['published_at']
 assert [x['value']['blocks'][0]['inlines'][0]['text'] for x in page['fields'] if x['key']=='body']==['Published words']
 with urlopen(Request(url+site+'/media/'+seeded['media']),timeout=30) as media_response:
  assert media_response.read()==bytes.fromhex(seeded['png']) and media_response.headers['Content-Type']=='image/png'
 with urlopen(url+'/c/'+seeded['public_key']+'/recovered',timeout=30) as rendered:
  assert 'Published words' in rendered.read().decode()
 restored_entry=api(base+'/cms/entries/'+seeded['entry'])
 assert restored_entry['draft']['title']=='Unpublished change' and restored_entry['unpublished_changes']
 assert len(api(base+'/cms/entries/'+seeded['entry']+'/revisions')['items'])==2
 print('PASS: published page, rendered address, media bytes, unpublished draft and revision history survive restore',flush=True)
projects_marker=directory/(args.project+'-projects.json')
if projects_marker.exists():
 seeded=json.loads(projects_marker.read_text());board_path=base+'/projects/'+seeded['project']
 restored_board=api(board_path+'/board')
 assert restored_board['project']['name']=='Recovered delivery'
 assert [c['id'] for c in restored_board['columns'][0]['cards']]==seeded['cards'] and restored_board['columns'][0]['cards'][1]['task']['title']=='Recovered two'
 assert [c['body'] for c in api(board_path+'/cards/'+seeded['cards'][0]+'/comments')['items']]==['Recovered comment']
 print('PASS: project board order, shared tasks and comments survive restore',flush=True)
knowledge_marker=directory/(args.project+'-knowledge.json')
if knowledge_marker.exists():
 seeded=json.loads(knowledge_marker.read_text());document_path=base+'/knowledge/documents/'+seeded['document']
 restored_sop=api(document_path)
 assert restored_sop['version']==seeded['version'] and restored_sop['body']==f"Original {seeded['word']} text" and restored_sop['collection_restricted'] and restored_sop['kind']=='sop'
 restored_revisions=api(document_path+'/revisions')['items']
 assert [r['number'] for r in restored_revisions]==[3,2,1] and restored_revisions[0]['restored_from_number']==1 and restored_revisions[1]['body']=='Later wording'
 assert [m['id'] for m in api(base+'/knowledge/collections/'+seeded['collection']+'/members')]
 found=api(base+'/knowledge/search?q='+seeded['word']);assert found['total']==1 and found['items'][0]['id']==seeded['document'] and any(p['matched'] for p in found['items'][0]['snippet'])
 print('PASS: restricted collection, document, revision history and search index survive restore',flush=True)
helpdesk_marker=directory/(args.project+'-helpdesk.json')
if helpdesk_marker.exists():
 seeded=json.loads(helpdesk_marker.read_text());ticket_path=base+'/helpdesk/tickets/'+seeded['ticket']
 assert api(ticket_path)['subject']=='Recovered support'
 assert [m['kind'] for m in api(ticket_path+'/messages')['items']]==['customer','note','reply']
 public_path='/api/v1/public/helpdesk/tickets/'+seeded['ticket']
 with urlopen(Request(url+public_path+'/messages',headers={'X-Ticket-Key':seeded['code']}),timeout=30) as response:customer_history=json.loads(response.read())
 assert [m['kind'] for m in customer_history['items']]==['customer','reply'] and 'Private recovery note' not in json.dumps(customer_history)
 if 'revoked_ticket' in seeded:
  assert api(base+'/helpdesk/tickets/'+seeded['revoked_ticket'])['customer_revoked']
  try:reply=urlopen(Request(url+'/api/v1/public/helpdesk/tickets/'+seeded['revoked_ticket'],headers={'X-Ticket-Key':seeded['revoked_code']}),timeout=30)
  except HTTPError as denied:assert denied.code==404
  else:reply.close();raise AssertionError('Revoked ticket code regained access after restore')
 print('PASS: Helpdesk history, scoped customer code and revocation survive restore without exposing internal notes',flush=True)
remaining_marker=directory/(args.project+'-time-scheduling.json')
if remaining_marker.exists():
 seeded=json.loads(remaining_marker.read_text())
 assert api(base+'/time/entries/'+seeded['time'])['duration_ms']==3456789
 assert any(b['id']==seeded['budget'] and b['budget_ms']==7200000 for b in api(base+'/time/budgets')['items'])
 booking=api(base+'/scheduling/appointments/'+seeded['booking'])
 assert booking['submission_id']==seeded['submission'] and booking['intake']['answers']=={'detail':'Recovered versioned answers'}
 assert booking['starts_at']==seeded['starts_at'] and booking['state']=='confirmed'
 with urlopen(Request(url+'/api/v1/public/scheduling/appointments/'+seeded['booking'],headers={'X-Booking-Key':seeded['code']}),timeout=30) as response:customer_booking=json.loads(response.read())
 assert customer_booking['id']==booking['id'] and 'intake' not in customer_booking
 assert len(api(base+'/scheduling/appointments/'+booking['id']+'/events')['items'])==1
 assert any(j['kind']=='scheduling.reminder' for j in report['quarantined_jobs'])
 print('PASS: exact time, budgets, atomic versioned booking intake, reservation history and scoped booking code survive restore; reminders quarantined',flush=True)
from recovery_journey import exercise
exercise(Path(result['target']),url,f.workspace,new_token,f.base+'/auth/v1')
