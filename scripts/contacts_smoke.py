#!/usr/bin/env python3
"""Contacts contract/denial/concurrency tests on a dedicated identity-check stack."""
import http.cookiejar
import json
import os
import secrets
import subprocess
from concurrent.futures import ThreadPoolExecutor
from urllib.error import HTTPError
from urllib.request import Request, urlopen, build_opener, HTTPCookieProcessor
from urllib.parse import urlencode
from dev import ROOT, configuration

config = configuration()
assert config['COMPOSE_PROJECT_NAME'].endswith('-identity-check'), 'Dedicated fixture stack required'
environment = {**os.environ, **config}
base = config['LOOSKI_PUBLIC_URL']
fixture = ROOT / '.local' / config['COMPOSE_PROJECT_NAME'] / 'identity-fixture.json'
owner = json.loads(fixture.read_text())

def sql(statement):
    return subprocess.check_output(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'], input=statement, text=True, cwd=ROOT, env=environment).strip()

def call(path, method='GET', body=None, expected=200, key=None, authenticated=True):
    headers = {'Content-Type':'application/json'}
    if authenticated: headers['Authorization'] = 'Bearer ' + token
    if key: headers['Idempotency-Key'] = key
    request = Request(base+path, method=method, headers=headers, data=None if body is None else json.dumps(body).encode())
    try: response = urlopen(request, timeout=20)
    except HTTPError as error: response = error
    with response:
        payload = json.loads(response.read())
        call.request_id = response.headers.get('X-Request-Id')
        assert response.status == expected, (method,path,response.status,expected,payload)
        return payload

# Existing fixture credentials never appear in argv or output.
jar=http.cookiejar.CookieJar()
browser=build_opener(HTTPCookieProcessor(jar))
with browser.open(Request(base+'/login', data=urlencode({'email':owner['email'],'password':owner['password']}).encode(),headers={'Origin':base,'Accept':'text/html'}),timeout=20) as response:
    assert response.status==200
assert any(c.name=='looski_access' for c in jar), 'Fixture sign-in failed'
token=next(c.value for c in jar if c.name=='looski_access')
workspace = call('/api/v1/workspaces')[0]['id']
path = '/api/v1/workspaces/'+workspace+'/contacts'
name = 'Contract '+secrets.token_hex(5)
body = {'display_name':name,'identifiers':[{'kind':'email','value':name.replace(' ','')+'@EXAMPLE.COM'},{'kind':'phone','value':'+1 (415) 555-'+str(secrets.randbelow(9000)+1000)}]}
key = secrets.token_hex(16)
call(path,expected=401,authenticated=False)
call(path,'POST',body,expected=400)
created = call(path,'POST',body,expected=201,key=key)
assert call.request_id and sql("SELECT request_id FROM suite.audit_events WHERE record_id='"+created['id']+"' AND action='contacts.created'")==call.request_id
assert call(path,'POST',body,expected=201,key=key)==created
call(path,'POST',{**body,'display_name':'Changed'},expected=409,key=key)
call(path,'POST',body,expected=409,key=secrets.token_hex(16))
contact_path=path+'/'+created['id']
assert call(contact_path)==created
assert any(c['id']==created['id'] for c in call(path+'?search='+name.replace(' ','%20'))['items'])
# Local-part case remains exact; domain case is normalized for matching.
email=body['identifiers'][0]['value']
case_contact=call(path,'POST',{'display_name':name+' lower','identifiers':[{'kind':'email','value':email.lower()}]},expected=201,key=secrets.token_hex(16))
shared=call(path,'POST',{**body,'display_name':name+' shared','allow_shared':True},expected=201,key=secrets.token_hex(16))
assert shared['id'] != created['id']
edit={'expected_version':created['version'],'display_name':name+' edited','notes':'Confirmed','identifiers':[]}
edit_key=secrets.token_hex(16)
updated=call(contact_path,'PATCH',edit,key=edit_key)
assert updated['version']==created['version']+1
assert call(contact_path,'PATCH',edit,key=edit_key)==updated
call(contact_path,'PATCH',edit,expected=409,key=secrets.token_hex(16))
archived=call(contact_path+'/archive','POST',{'expected_version':updated['version'],'archived':True},key=secrets.token_hex(16))
assert archived['archived']
assert not any(c['id']==created['id'] for c in call(path+'?search='+name.replace(' ','%20'))['items'])
assert any(c['id']==created['id'] for c in call(path+'?archived=true&search='+name.replace(' ','%20'))['items'])
restored=call(contact_path+'/archive','POST',{'expected_version':archived['version'],'archived':False},key=secrets.token_hex(16))
assert not restored['archived']
# Same-key concurrent writes replay one exact result; independent requests for
# the same new identifiers cannot both create a person without reviewed sharing.
concurrent={'display_name':name+' concurrent','identifiers':[{'kind':'email','value':secrets.token_hex(10)+'@example.com'}]}
concurrent_key=secrets.token_hex(16)
with ThreadPoolExecutor(max_workers=4) as executor:
    results=list(executor.map(lambda _:call(path,'POST',concurrent,expected=201,key=concurrent_key),range(4)))
assert all(result==results[0] for result in results)
# Distinct retry keys must still serialize matching, including partially
# overlapping multi-identifier sets. This cannot pass on the retry lock alone.
shared_email=secrets.token_hex(10)+'@example.com'
def race_create(index):
    payload={'display_name':name+' intake '+str(index),'identifiers':[{'kind':'email','value':shared_email},{'kind':'email','value':secrets.token_hex(10)+'@example.com'}]}
    try:
        return call(path,'POST',payload,expected=201,key=secrets.token_hex(16))['id']
    except AssertionError as failure:
        assert '409' in str(failure)
        return None
with ThreadPoolExecutor(max_workers=4) as executor:
    assert len([r for r in executor.map(race_create,range(4)) if r])==1
# Optimistic edits: exactly one competing version wins.
concurrent_path=path+'/'+results[0]['id']
def race_edit(index):
    payload={'expected_version':1,'display_name':name+str(index)}
    try:
        call(concurrent_path,'PATCH',payload,key=secrets.token_hex(16))
        return True
    except AssertionError as failure:
        assert '409' in str(failure)
        return False
with ThreadPoolExecutor(max_workers=2) as executor:
    assert sum(executor.map(race_edit,range(2)))==1
# Pagination has no duplicates and applies visibility before limiting.
# Scope this probe to its unique run so retained fixtures do not make it unbounded.
seen=set(); cursor=''; pages=0
while True:
    page=call(path+'?limit=2&search='+name.replace(' ','%20')+('&after='+cursor if cursor else ''))
    ids={c['id'] for c in page['items']}
    assert not ids & seen
    seen |= ids; pages+=1
    cursor=page['next_cursor']
    if not cursor: break
    assert pages<10
assert created['id'] in seen and pages>1
call(path+'?limit=10000',expected=400)
call(path+'?limit=wrong',expected=400)
call(path+'/not-a-uuid',expected=400)
call(path,'POST',{'unexpected':True},expected=400,key=secrets.token_hex(16))
call('/api/v1/workspaces/00000000-0000-0000-0000-000000000001/contacts',expected=403)
# Failure injected at the transactional audit boundary must roll back business
# data and the retry record; retry succeeds after restoring the dependency.
fault_key=secrets.token_hex(16)
fault={'display_name':name+' rollback'}
sql("CREATE FUNCTION suite.contacts_probe_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit outage'; END $$; CREATE TRIGGER contacts_probe_fail BEFORE INSERT ON suite.audit_events FOR EACH ROW EXECUTE FUNCTION suite.contacts_probe_fail();")
try:
    call(path,'POST',fault,expected=503,key=fault_key)
    assert call(path+'?search='+name.replace(' ','%20')+'%20rollback')['items']==[]
finally:
    sql('DROP TRIGGER contacts_probe_fail ON suite.audit_events; DROP FUNCTION suite.contacts_probe_fail();')
call(path,'POST',fault,expected=201,key=fault_key)
# Current permissions constrain replay, too.
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:
    assert call(contact_path)['id']==created['id']
finally:
    sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
sql("UPDATE suite.memberships SET state='suspended'")
try:
    call(path,expected=403)
    call(path,'POST',body,expected=403,key=key)
finally:
    sql("UPDATE suite.memberships SET state='active'")
# Assignment scope never borrows the broader read action's scope.
other_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES ('Scope fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+workspace+"',id FROM u RETURNING id") .splitlines()[0]
other=call(path,'POST',{'display_name':name+' restricted','owner_membership_id':other_member},expected=201,key=secrets.token_hex(16))
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key IN ('contacts.update','contacts.archive') AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:
    assert call(path+'/'+other['id'])['id']==other['id']  # workspace-wide reads
    call(path+'/'+other['id'],'PATCH',{'display_name':'Denied','expected_version':1},expected=404,key=secrets.token_hex(16))
    call(path+'/'+other['id']+'/archive','POST',{'archived':True,'expected_version':1},expected=404,key=secrets.token_hex(16))
finally:
    sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key IN ('contacts.update','contacts.archive') AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:
    call(path+'/'+other['id'],expected=404)
    assert call(path+'?search='+name.replace(' ','%20')+'%20restricted&limit=1')['items']==[]
finally:
    sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
# Preserve identifiers/history, flatten aliases, reject stale merge previews.
source=call(path,'POST',{'display_name':name+' duplicate','notes':'Original source notes','identifiers':[{'kind':'email','value':secrets.token_hex(10)+'@example.com'}]},expected=201,key=secrets.token_hex(16))
merge={'source_id':source['id'],'source_version':source['version'],'survivor_version':restored['version'],'resolved':{'display_name':name+' merged','notes':'Resolved notes'}}
merge_key=secrets.token_hex(16)
survivor=call(contact_path+'/merge','POST',merge,key=merge_key)
assert call(contact_path+'/merge','POST',merge,key=merge_key)==survivor
assert call(path+'/'+source['id'])==survivor
assert source['identifiers'][0] in survivor['identifiers']
assert sql("SELECT notes FROM contacts.contacts WHERE id='"+source['id']+"'")=='Original source notes'
assert sql("SELECT count(*) FROM suite.audit_events WHERE action='contacts.merged' AND record_id='"+source['id']+"'")=='1'
call(contact_path+'/merge','POST',merge,expected=409,key=secrets.token_hex(16))
# An oversized merge must leave both valid contacts editable and unchanged.
large=[]
for label in ('left','right'):
    large.append(call(path,'POST',{'display_name':name+label,'identifiers':[{'kind':'email','value':secrets.token_hex(8)+'@example.com'} for _ in range(11)]},expected=201,key=secrets.token_hex(16)))
call(path+'/'+large[0]['id']+'/merge','POST',{'source_id':large[1]['id'],'source_version':1,'survivor_version':1,'resolved':{'display_name':'Too many'}},expected=400,key=secrets.token_hex(16))
for contact in large: assert call(path+'/'+contact['id'])==contact
# Invalid metadata does not enter the registry, national format needs context.
call(path,'POST',{'display_name':'Invalid','identifiers':[{'kind':'phone','value':'+999999999999'}]},expected=400,key=secrets.token_hex(16))
national=call(path,'POST',{'display_name':name+' phone','identifiers':[{'kind':'phone','value':'020 7946 0958 ext. 123','country':'GB'}],'allow_shared':True},expected=201,key=secrets.token_hex(16))
assert national['identifiers'][0]['normalized_value']=='+442079460958'
assert national['identifiers'][0]['extension']=='123'
# All three programmatic interfaces observe the same contact.
token_file=fixture.parent/'contacts-access-token'
with token_file.open('w') as stream:
    os.chmod(stream.name,0o600); stream.write(token)
try:
    binary=str(ROOT/'target/debug/looski')
    command=[binary,'contacts','--api-url',base,'--token-file',str(token_file),'--workspace',workspace]
    cli=json.loads(subprocess.check_output([*command,'get',created['id']],text=True))
    assert cli==survivor
    cli_body={'display_name':name+' cli'}
    cli_created=json.loads(subprocess.check_output([*command,'create','--idempotency-key',secrets.token_hex(16)],input=json.dumps(cli_body),text=True))
    assert call(path+'/'+cli_created['id'])==cli_created
    denied=subprocess.run([*command,'get','00000000-0000-0000-0000-000000000001'],capture_output=True,text=True)
    assert denied.returncode==3
    failure=json.loads(denied.stderr)
    assert failure['code']=='not_found' and failure['request_id'] and not denied.stdout
    messages=[
      {'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'contacts-test','version':'1'}}},
      {'jsonrpc':'2.0','method':'notifications/initialized'},
      {'jsonrpc':'2.0','id':2,'method':'tools/call','params':{'name':'get_contact','arguments':{'workspace_id':workspace,'contact_id':created['id']}}},
    ]
    import selectors
    with subprocess.Popen([binary,'mcp','--api-url',base,'--token-file',str(token_file)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,bufsize=1) as process:
        selector=selectors.DefaultSelector(); selector.register(process.stdout,selectors.EVENT_READ)
        try:
            for message in messages:
                process.stdin.write(json.dumps(message)+'\n'); process.stdin.flush()
                if 'id' not in message: continue
                assert selector.select(15), 'MCP timed out'
                reply=json.loads(process.stdout.readline())
                assert reply.get('id')==message['id'] and 'error' not in reply
            assert not reply['result'].get('isError')
            assert json.loads(reply['result']['content'][0]['text'])==survivor
            denial={'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'get_contact','arguments':{'workspace_id':workspace,'contact_id':'00000000-0000-0000-0000-000000000001'}}}
            process.stdin.write(json.dumps(denial)+'\n'); process.stdin.flush()
            assert selector.select(15)
            denied=json.loads(process.stdout.readline())['result']
            assert denied['isError'] and denied['structuredContent']['code']=='not_found'
            assert denied['structuredContent']['request_id']
        finally:
            selector.close(); process.stdin.close(); process.wait(timeout=5)
finally:
    token_file.unlink()
assert int(sql("SELECT count(*) FROM suite.audit_events WHERE action='contacts.created'"))>=5
assert sql("SELECT source_interface FROM suite.audit_events WHERE record_id='"+cli_created['id']+"' AND action='contacts.created'")=='cli'

# Server-rendered browser journey exercises actual forms, redirects, and CSRF.
from html.parser import HTMLParser
class Fields(HTMLParser):
    def __init__(self):
        super().__init__(); self.values={}
    def handle_starttag(self, tag, attrs):
        attrs=dict(attrs)
        if tag=='input' and attrs.get('type')=='hidden' and attrs.get('name') not in self.values:
            self.values[attrs['name']]=attrs.get('value','')
def page(url):
    with browser.open(base+url,timeout=20) as response:
        html=response.read().decode(); fields=Fields(); fields.feed(html)
        return html,fields.values
web_path='/workspaces/'+workspace+'/contacts'
html,fields=page(web_path+'?search='+name.replace(' ','%20'))
assert 'People behind the business.' in html and name in html
web_name=name+' browser'
form={**fields,'display_name':web_name,'emails':'','phones':'','notes':'Browser fixture'}
try:
    browser.open(Request(base+web_path+'?/create',data=urlencode(form).encode(),headers={'Origin':'https://attacker.invalid','Accept':'text/html'}),timeout=20)
    raise AssertionError('Cross-origin mutation accepted')
except HTTPError as failure:
    assert failure.code==403
with browser.open(Request(base+web_path+'?/create',data=urlencode(form).encode(),headers={'Origin':base,'Accept':'text/html'}),timeout=20) as response:
    detail=response.geturl().removeprefix(base)
    html=response.read().decode()
    if web_name not in html:
        from pathlib import Path
        Path('/tmp/looski-contact-form-failure.html').write_text(html)
    assert web_name in html, (response.status,response.geturl(),html[:500])
html,fields=page(detail)
form={**fields,'display_name':web_name+' edited','notes':'Saved in browser','emails':'','phones':''}
with browser.open(Request(base+detail+'?/edit',data=urlencode(form).encode(),headers={'Origin':base,'Accept':'text/html'}),timeout=20) as response:
    assert 'Saved in browser' in response.read().decode()
assert sql("SELECT source_interface FROM suite.audit_events WHERE record_id='"+detail.rsplit('/',1)[1]+"' AND action='contacts.updated'")=='web'
print('PASS: Contacts web/API/CLI/MCP, scopes, merge/history, case/phone normalization, retry/concurrency, rollback/audit correlation, CSRF')
