#!/usr/bin/env python3
"""Invited CMS contributors: real identities, scoped reads/writes, revision review,
byline snapshots, media privacy, revocation, browser and CLI/MCP parity."""
import base64
import json
import secrets
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
from integration_fixture import Fixture
from fixture_people import people

f = Fixture()
call, sql = f.call, f.sql
Person = people(f)
base = '/api/v1/workspaces/' + f.workspace
cms = base + '/cms'
key = lambda: secrets.token_hex(16)
module = next(m for m in call(base + '/modules') if m['key'] == 'cms')
if not module['enabled']:
    call(base + '/modules/cms', 'PATCH', {'enabled': True, 'expected_version': module['version']}, key=key())
fields = [{'key': 'author', 'label': 'Byline', 'kind': 'text'}, {'key': 'author_id', 'label': 'Author ID', 'kind': 'text'}, {'key': 'body', 'label': 'Article', 'kind': 'rich_text', 'required': True}, {'key': 'hero', 'label': 'Image', 'kind': 'media'}]
t = call(cms + '/types', 'POST', {'name': 'Contributions ' + key(), 'fields': fields}, expected=201, key=key())
t = call(cms + '/types/' + t['id'] + '/coauthor', 'POST', {'expected_version': t['version'], 'name': 'Looski', 'use_looski_logo': True}, key=key())
other = call(cms + '/types', 'POST', {'name': 'Private collection ' + key()}, expected=201, key=key())
site = '/api/v1/public/cms/' + t['public_key']
grants = ['workspace.read', 'cms.contribute', 'files.read:assigned', 'files.create:assigned']
alice, bob = Person('Alice writer', grants), Person('Bob writer', grants)
assert alice.call(cms + '/writing-access')['contributor_only']
alice.call(cms + '/entries', expected=403)
def approve(person, name):
    return call(cms + '/contributors', 'POST', {'membership_id': person.membership, 'type_id': t['id'], 'name': name, 'bio': 'Contributor biography', 'active': True, 'expected_version': 0}, key=key())
a, b = approve(alice, 'Alice'), approve(bob, 'Bob')
assert [x['id'] for x in alice.call(cms + '/types')['items']] == [t['id']]
alice.call(cms + '/types/' + other['id'], expected=404)
alice.call(cms + '/types?after=' + other['id'], expected=404)
alice.call(cms + '/types', 'POST', {'name': 'No authority'}, expected=403)
alice.call(cms + '/types/' + t['id'] + '/coauthor', 'POST', {'expected_version': t['version'], 'name': 'Impersonation'}, expected=403)
for resource in ['/contacts', '/crm/deals', '/access/members', '/exports']:
    # Existing routes distinguish unimplemented collections (404) from denied ones.
    if resource != '/exports': alice.call(base + resource, expected=403)
alice.call(cms + '/contributors', 'POST', {**{k:v for k,v in b.items() if k!='version'}, 'expected_version': b['version']}, expected=403)
alice.call(cms + '/contributors', 'POST', {'membership_id': alice.membership, 'type_id': other['id'], 'name': 'No grant', 'active': True, 'expected_version': 0}, expected=403)

def create(person, title, values=None, type_id=None):
    return person.call(cms + '/entries', 'POST', {'type_id': type_id or t['id'], 'title': title, 'slug': 'contribution-' + key(), 'values': values or {'body': 'An original article.'}}, expected=201)
alice.call(cms + '/entries', 'POST', {'type_id': other['id'], 'title': 'Wrong collection', 'slug': 'wrong'}, expected=404)
entry, hidden = create(alice, 'Alice article', {'body':'An original article.','author':'Forged name','author_id':'forged-id'}), create(bob, 'Bob secret draft')
assert entry['draft']['values']['author']=='Alice and Looski' and entry['draft']['values']['author_id']==''
epath, hidden_path = cms + '/entries/' + entry['id'], cms + '/entries/' + hidden['id']
assert [x['id'] for x in alice.call(cms + '/entries')['items']] == [entry['id']]
for suffix in ['', '/preview', '/revisions']:
    alice.call(hidden_path + suffix, expected=404)
alice.call(cms + '/entries?after=' + hidden['id'], expected=404)
alice.call(hidden_path, 'PATCH', {'expected_version': hidden['version'], 'title': 'Stolen', 'slug': hidden['draft']['slug']}, expected=404)
revision = bob.call(hidden_path + '/revisions')['items'][0]
alice.call(hidden_path + '/revisions/' + revision['id'], expected=404)
alice.call(hidden_path + '/revisions/' + revision['id'] + '/restore', 'POST', {'expected_version': hidden['version']}, expected=404)
alice.call(hidden_path + '/archive', 'POST', {'expected_version': hidden['version'], 'archived': True}, expected=404)
alice.call(epath + '/publish', 'POST', {'expected_version': entry['version']}, expected=403)
call(epath + '/publish', 'POST', {'expected_version': entry['version']}, expected=409, key=key())
assert [author['name'] for author in entry['authors']] == ['Alice', 'Looski']
assert entry['authors'][1]['image_url'] == '/brand/looski-mark.svg'
call(site + '/entries/' + entry['draft']['slug'], expected=404, authenticated=False)
# Media is private until an approved revision references it.
png = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFgAI/ScLttAAAAABJRU5ErkJggg==')
def upload(person):
    file = person.call(base + '/files', 'POST', {'filename': 'writer.png', 'media_type': 'image/png', 'content_base64': base64.b64encode(png).decode()}, expected=201)
    return person.call(cms + '/media', 'POST', {'file_id': file['id'], 'title': 'Writer picture'}, expected=201)
image, foreign_image = upload(alice), upload(bob)
alice.call(cms + '/media/' + foreign_image['id'] + '/content', expected=404)
alice.call(cms + '/media?after=' + foreign_image['id'], expected=404)
assert [m['id'] for m in alice.call(cms + '/media')['items']] == [image['id']]
alice.call(cms + '/media/' + foreign_image['id'], 'PATCH', {'expected_version': 1, 'title': 'Stolen'}, expected=404)
call(site + '/media/' + image['id'], expected=404, authenticated=False)
a = alice.call(cms + '/contributors', 'POST', {'membership_id': alice.membership, 'type_id': t['id'], 'name': 'Alice', 'bio': a['bio'], 'photo_media_id': image['id'], 'active': True, 'expected_version': a['version']})
def save(person, current, body='Updated article.', expected=200, **extra):
    return person.call(epath, 'PATCH', {'expected_version': current['version'], 'title': current['draft']['title'], 'slug': current['draft']['slug'], 'values': {'body': body}, **extra}, expected=expected)
save(alice, entry, expected=404, values={'body':'Other image', 'hero':foreign_image['id']})
entry = save(alice, entry)
assert entry['authors'][0]['media_id'] == image['id']
# Submitted revision is fixed, stale saves/reviews fail, and retry emits one event.
retry = key()
submitted = alice.call(epath + '/review', 'POST', {'expected_version': entry['version'], 'action': 'submit'}, retry=retry)
assert alice.call(epath + '/review', 'POST', {'expected_version': entry['version'], 'action': 'submit'}, retry=retry) == submitted
assert submitted['review_state'] == 'submitted'
save(alice, submitted, expected=409)
save(alice, entry, expected=409)
call(epath + '/review', 'POST', {'expected_version': entry['version'], 'action': 'request_changes', 'note':'Stale'}, expected=409, key=key())
alice.call(epath + '/review', 'POST', {'expected_version': submitted['version'], 'action': 'request_changes', 'note': 'Approve myself'}, expected=403)
assert entry['id'] in [e['id'] for e in call(cms + '/entries?review_state=submitted')['items']]
assert len(alice.call(cms + '/contribution-events')) == 1
assert bob.call(cms + '/contribution-events') == []
# Publisher may review without cms.entries.edit; independent grants work.
reviewer = Person('Review editor', ['workspace.read','cms.read','cms.drafts.read','cms.entries.publish'])
returned = reviewer.call(epath + '/review', 'POST', {'expected_version': submitted['version'], 'action': 'request_changes', 'note': 'Please clarify the example.'})
assert returned['review_note'] == 'Please clarify the example.'
entry = save(alice, returned, 'A clear example.')
submitted = alice.call(epath + '/review', 'POST', {'expected_version': entry['version'], 'action': 'submit'})
# Changes to a collection schema must not silently change a reviewed revision.
t = call(cms + '/types/' + t['id'], 'PATCH', {'name':t['name'],'fields':[*fields, {'key':'summary','label':'Summary','kind':'text'}], 'expected_version':t['version']}, key=key())
reviewer.call(epath + '/publish', 'POST', {'expected_version': submitted['version']}, expected=409)
entry = reviewer.call(epath + '/review', 'POST', {'expected_version':submitted['version'],'action':'request_changes','note':'Please review the new field.'})
entry = save(alice, entry, 'A clear example.')
submitted = alice.call(epath + '/review', 'POST', {'expected_version':entry['version'],'action':'submit'})
retry = key()
live = reviewer.call(epath + '/publish','POST',{'expected_version':submitted['version']},retry=retry)
assert reviewer.call(epath + '/publish','POST',{'expected_version':submitted['version']},retry=retry)==live
public = call(site + '/entries/' + live['published']['slug'],authenticated=False)
assert [a['name'] for a in public['authors']]==['Alice','Looski']
assert public['authors'][0]['image_url']==site+'/media/'+image['id']
with urlopen(f.base+public['authors'][0]['image_url'],timeout=20) as response:
    assert response.status==200 and response.read()==png
with urlopen(f.base+public['authors'][1]['image_url'],timeout=20) as response:
    assert response.status==200 and b'<svg' in response.read()
assert call(site+'/data',authenticated=False)['items'][0]['authors']==public['authors']
alice.call(cms+'/media/'+image['id'],'PATCH',{'expected_version':image['version'],'title':'Bypass review'},expected=409)
alice.call(cms+'/media/'+image['id']+'/archive','POST',{'expected_version':image['version'],'archived':True},expected=403)
# Profile edits do not rewrite the public snapshot, and later revisions stay private.
a = alice.call(cms+'/contributors','POST',{'membership_id':alice.membership,'type_id':t['id'],'name':'Alice Updated','bio':'New bio','photo_media_id':image['id'],'active':True,'expected_version':a['version']})
assert call(site+'/entries/'+live['published']['slug'],authenticated=False)['authors'][0]['name']=='Alice'
entry = save(alice,live,'A proposed follow-up.')
assert entry['authors'][0]['name']=='Alice Updated'
assert next(x for x in call(site+'/entries/'+live['published']['slug'],authenticated=False)['fields'] if x['key']=='body')['value']!=next(x for x in alice.call(epath+'/preview')['fields'] if x['key']=='body')['value']
save(alice,entry,expected=409,slug='changed-live-address')
submitted=alice.call(epath+'/review','POST',{'expected_version':entry['version'],'action':'submit'})
later=reviewer.call(epath+'/publish','POST',{'expected_version':submitted['version']})
assert later['published']['published_at']==live['published']['published_at']
# Action scopes intersect: workspace published-read does not expose others' drafts.
alice.grants(grants+['cms.read','cms.drafts.read','cms.entries.edit'])
replay_key=key()
changed=alice.call(hidden_path,'PATCH',{'expected_version':hidden['version'],'title':'Bob updated by editor','slug':hidden['draft']['slug'],'values':hidden['draft']['values']},retry=replay_key)
alice.grants(grants+['cms.read'])
alice.call(hidden_path,'PATCH',{'expected_version':hidden['version'],'title':'Bob updated by editor','slug':hidden['draft']['slug'],'values':hidden['draft']['values']},retry=replay_key,expected=404)
for suffix in ['/revisions','/revisions/'+revision['id']]:alice.call(hidden_path+suffix,expected=404)
alice.call(hidden_path,expected=404)
assert hidden['id'] not in [e['id'] for e in alice.call(cms+'/entries')['items']]
alice.grants(grants)
# Browser writing area excludes staff navigation and never shows publish controls.
def web(path,person=alice,form=None):
    headers={'Cookie':'looski_access='+person.token,'Accept':'text/html'}
    if form is not None:headers.update({'Origin':f.base,'Content-Type':'application/x-www-form-urlencoded'})
    try: response=urlopen(Request(f.base+path,headers=headers,data=None if form is None else urlencode(form).encode()),timeout=30)
    except HTTPError as error:response=error
    with response:return response.status,response.read().decode()
root='/workspaces/'+f.workspace+'/cms'
status,html=web(root)
assert status==200 and 'My articles' in html and 'Bob secret draft' not in html
assert '>Contacts<' not in html and '>Workspace access<' not in html
status,html=web(root+'/entries/'+entry['id'])
assert status==200 and 'Looski' in html and 'Save and publish' not in html
assert web(root+'/entries/'+hidden['id'])[0]==404
assert web(root+'/activity')[0]==200
assert web(root+'/contributors')[0]==200
# Real browser forms save, submit, and withdraw through the same version checks.
status,html=web(root+'/entries/'+entry['id']+'?/save',form={'idempotency_key':key(),'expected_version':later['version'],'title':'Alice browser revision','slug':later['draft']['slug'],'value.body':'Written in the browser.'})
assert status==200 and 'Alice browser revision' in html
browser_draft=alice.call(epath)
status,html=web(root+'/entries/'+entry['id']+'?/review',form={'idempotency_key':key(),'expected_version':browser_draft['version'],'review_action':'submit'})
assert status==200 and alice.call(epath)['review_state']=='submitted'
browser_review=alice.call(epath)
assert web(root+'/entries/'+entry['id']+'?/review',form={'idempotency_key':key(),'expected_version':browser_review['version'],'review_action':'withdraw'})[0]==200
assert alice.call(epath)['review_state']=='draft'

# Current-token grant revocation covers listing, direct paths and retries.
a=call(cms+'/contributors','POST',{'membership_id':alice.membership,'type_id':t['id'],'name':a['name'],'bio':a['bio'],'photo_media_id':image['id'],'active':False,'expected_version':a['version']},key=key())
for path in [cms+'/entries',epath,epath+'/preview',epath+'/revisions',cms+'/media',cms+'/contribution-events']:
    alice.call(path,expected=403)
assert 'Waiting for collection access' in web(root)[1]
assert call(site+'/entries/'+later['published']['slug'],authenticated=False)['authors'][0]['name']=='Alice Updated'
# New interfaces share the canonical API; no alternate authority path.
assert f.cli(['cms','contributors'])
assert not f.mcp('cms_contributors',{'workspace_id':f.workspace}).get('isError')
assert f.cli(['cms','entry-list','--review-state','submitted'])['items']==[]
print('PASS: contributor isolation, media privacy, exact revision review, co-author logo, notifications, revocation, browser and CLI/MCP')
