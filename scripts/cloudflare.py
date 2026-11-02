"""Resumable Cloudflare provisioning; credentials never enter argv or reports."""
import json
import re
import secrets
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlsplit
from urllib.request import Request, build_opener, HTTPRedirectHandler

from dev import write_private
from recovery_archive import private_regular


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


class CloudflareError(RuntimeError):
    def __init__(self, status):
        self.status = status
        super().__init__(f'Cloudflare returned HTTP {status}; check account permissions and retry')


class Cloudflare:
    def __init__(self, token_file):
        self.token = private_regular(token_file).decode().strip()
        if not re.fullmatch(r'[A-Za-z0-9_-]{20,512}', self.token):
            raise ValueError('Invalid Cloudflare API token file')
        self.http = build_opener(NoRedirect())

    def request(self, method, path, data=None):
        request = Request('https://api.cloudflare.com/client/v4' + path,
                          data=json.dumps(data).encode() if data is not None else None,
                          headers={'Authorization': 'Bearer ' + self.token,
                                   'Content-Type': 'application/json'}, method=method)
        try:
            with self.http.open(request, timeout=30) as response:
                document = json.loads(response.read(2 * 1024 * 1024))
        except HTTPError as error:
            raise CloudflareError(error.code) from None
        except (URLError, TimeoutError, ValueError):
            raise RuntimeError('Cloudflare response unavailable; retry the same command to reconcile progress') from None
        if not document.get('success'):
            raise RuntimeError('Cloudflare rejected the operation; check account permissions and retry')
        return document['result']


def hostname_for(origin):
    parsed = urlsplit(origin)
    hostname = parsed.hostname or ''
    if (parsed.scheme != 'https' or parsed.port or parsed.path or parsed.query or parsed.fragment
            or parsed.username is not None or parsed.password is not None or not re.fullmatch(r'[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?', hostname)):
        raise ValueError('Provide an HTTPS hostname without a port or path')
    return hostname


def discover(client, origin, account=None, zone=None):
    """Resolve the most-specific authorized DNS zone without changing anything."""
    hostname = hostname_for(origin)
    for value in (account, zone):
        if value is not None and not re.fullmatch(r'[a-f0-9]{32}', value):
            raise ValueError('Cloudflare IDs must be 32 lowercase hexadecimal characters')
    if zone:
        candidates = [client.request('GET', '/zones/' + zone)]
    else:
        candidates = []
        labels = hostname.split('.')
        # Include the exact hostname to detect a delegated zone apex: do not
        # accidentally create its record in a less-specific parent zone.
        for offset in range(len(labels) - 1):
            query = {'name': '.'.join(labels[offset:]), 'per_page': 50}
            if account:
                query['account.id'] = account
            candidates = client.request('GET', '/zones?' + urlencode(query))
            if candidates:
                break
    if len(candidates) != 1:
        raise ValueError('No unique authorized Cloudflare zone found; check Zone Read access or supply --zone-id')
    selected = candidates[0]
    if account is not None and selected['account']['id'] != account:
        raise ValueError('The selected DNS zone belongs to a different account')
    if selected['status'] != 'active':
        raise ValueError('Cloudflare zone is not active; finish nameserver delegation and retry')
    if not hostname.endswith('.' + selected['name']):
        raise ValueError('Choose a subdomain in the selected Cloudflare zone')
    return selected['account']['id'], selected['id']


def provision(client, state, account, zone, origin):
    if any(not re.fullmatch(r'[a-f0-9]{32}', value) for value in (account, zone)):
        raise ValueError('Cloudflare account and zone IDs must be 32 lowercase hexadecimal characters')
    hostname = hostname_for(origin)
    zone_info = client.request('GET', '/zones/' + zone)
    if zone_info['account']['id'] != account or zone_info['status'] != 'active':
        raise ValueError('Cloudflare zone must be active and belong to the selected account; finish DNS delegation first')
    if not hostname.endswith('.' + zone_info['name']) or hostname == zone_info['name']:
        raise ValueError('Choose a subdomain in the selected Cloudflare zone')
    path = state / 'cloudflare.json'
    intent = {'account': account, 'zone': zone, 'hostname': hostname}
    if path.exists():
        journal = json.loads(private_regular(path))
        if any(journal.get(key) != value for key, value in intent.items()):
            raise ValueError('This installation already owns a different Cloudflare route; migration must be explicit')
    else:
        journal = {**intent, 'name': 'looski-' + secrets.token_hex(16)}
    records_path = '/zones/' + zone + '/dns_records'
    records = client.request('GET', records_path + '?' + urlencode({'name': hostname, 'per_page': 100}))
    # Reject occupied names before creating a tunnel. On resume accept only our target.
    target = journal.get('tunnel_id', '') + '.cfargotunnel.com'
    if records and (len(records) != 1 or records[0]['type'] != 'CNAME'
                    or records[0]['content'] != target or not records[0].get('proxied')):
        raise ValueError('The requested hostname already has a DNS record; choose a free hostname')
    write_private(path, json.dumps(journal)) # Intent precedes every external creation.
    tunnels = '/accounts/' + account + '/cfd_tunnel'
    if 'tunnel_id' not in journal:
        matches = client.request('GET', tunnels + '?' + urlencode({'name': journal['name'], 'is_deleted': 'false'}))
        if len(matches) > 1:
            raise ValueError('Ambiguous tunnel identity; inspect Cloudflare before retrying')
        tunnel = matches[0] if matches else client.request('POST', tunnels, {'name': journal['name'], 'config_src': 'cloudflare'})
        journal['tunnel_id'] = tunnel['id']
        write_private(path, json.dumps(journal))
    tunnel_path = tunnels + '/' + journal['tunnel_id']
    tunnel = client.request('GET', tunnel_path)
    if tunnel.get('name') != journal['name'] or tunnel.get('deleted_at') or tunnel.get('config_src') != 'cloudflare':
        raise ValueError('Recorded Cloudflare tunnel no longer matches this installation')
    desired = {'ingress': [{'hostname': hostname, 'service': 'http://gateway:8080'}, {'service': 'http_status:404'}]}
    try:
        current = client.request('GET', tunnel_path + '/configurations')
    except CloudflareError as error:
        if error.status != 404 or journal.get('state') == 'configured':
            raise
        current = {}
    config = (current or {}).get('config') or {}
    ingress = config.get('ingress') or []
    if ingress and ingress != desired['ingress'] and not (journal.get('state') != 'configured' and ingress == [{'service': 'http_status:404'}]):
        raise ValueError('Tunnel routing was changed outside the installer; refusing to overwrite it')
    if ingress != desired['ingress']:
        client.request('PUT', tunnel_path + '/configurations', {'config': desired})
    token = client.request('GET', tunnel_path + '/token')
    if not isinstance(token, str) or not token or len(token) > 16384:
        raise RuntimeError('Cloudflare returned an invalid connector token')
    write_private(state / 'cloudflare-token', token + '\n')
    target = journal['tunnel_id'] + '.cfargotunnel.com'
    if not records:
        record = client.request('POST', records_path, {'type': 'CNAME', 'name': hostname, 'content': target, 'proxied': True, 'ttl': 1})
    else:
        record = records[0]
    journal['dns_record_id'] = record['id']
    journal['state'] = 'configured'
    write_private(path, json.dumps(journal))
    return {'hostname': hostname, 'tunnel_id': journal['tunnel_id'], 'dns_record_id': record['id']}
