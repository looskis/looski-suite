#!/usr/bin/env python3
"""Claim-owner/browser acceptance against an explicitly isolated fresh fixture."""
import http.cookiejar
import json
import secrets
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, build_opener, HTTPCookieProcessor, urlopen

import dev
import install


def check(root):
    dev.ROOT = Path(root).resolve()
    config = install.values()
    assert config['COMPOSE_PROJECT_NAME'].endswith('-identity-check'), 'Dedicated disposable fixture required'
    assert not install.owner_exists(config), 'Fresh unclaimed installation required'
    base = config['LOOSKI_PUBLIC_URL']
    link = install.claim_link(config)
    token = Path(link['private_link_file']).read_text().strip().split('#', 1)[1]
    password = secrets.token_urlsafe(24)
    data = {'email': 'installer-owner@example.com', 'password': password,
            'display_name': 'Installer Owner', 'workspace_name': 'Installer validation'}

    def request(path, *, body=None, origin=None, credential=None):
        headers = {'Content-Type': 'application/json'}
        if origin: headers['Origin'] = origin
        if credential: headers['Authorization'] = 'Bearer ' + credential
        try:
            response = urlopen(Request(base + path, data=json.dumps(body).encode() if body else None, headers=headers), timeout=30)
        except HTTPError as error:
            response = error
        with response:
            return response.status, response.headers, response.read()

    status, headers, html = request('/setup')
    assert status == 200 and b'Make yourself at home' in html
    assert headers['Referrer-Policy'] == 'no-referrer'
    assert headers['Cache-Control'] == 'no-store'
    assert token.encode() not in html
    assert request('/setup/owner', body=data, origin=base)[0] == 404
    assert request('/setup/owner', body=data, origin=base, credential='0' * 64)[0] == 404
    assert request('/setup/owner', body=data, origin='https://attacker.example', credential=token)[0] == 404
    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(lambda _: request('/setup/owner', body=data, origin=base, credential=token)[0], range(2)))
    assert results.count(201) == 1 and all(status in (201, 400, 404) for status in results), results
    assert request('/setup/owner', body=data, origin=base, credential=token)[0] == 404
    assert request('/setup')[0] == 404
    assert install.claim_link(config)['state'] == 'already_claimed'
    jar = http.cookiejar.CookieJar()
    browser = build_opener(HTTPCookieProcessor(jar))
    with browser.open(Request(base + '/login', data=urlencode({'email': data['email'], 'password': password}).encode(),
                              headers={'Origin': base, 'Accept': 'text/html'}), timeout=30) as response:
        assert response.status == 200
        assert b'Installer validation' in response.read()
    access = next(cookie.value for cookie in jar if cookie.name == 'looski_access')
    with urlopen(Request(base + '/api/v1/workspaces', headers={'Authorization': 'Bearer ' + access}), timeout=15) as response:
        workspaces = json.load(response)
        assert len(workspaces) == 1 and workspaces[0]['name'] == data['workspace_name']
    # Exercise changing browser origin without changing the stable issuer or identity mapping.
    original_issuer = config['LOOSKI_AUTH_ISSUER']
    config['LOOSKI_PUBLIC_URL'] = base.replace('localhost', '127.0.0.1')
    assert config['LOOSKI_PUBLIC_URL'] != base, 'Fixture must start on localhost'
    install.save_config(config)
    install.up(config)
    base = config['LOOSKI_PUBLIC_URL']
    assert install.values()['LOOSKI_AUTH_ISSUER'] == original_issuer
    promoted = build_opener(HTTPCookieProcessor(http.cookiejar.CookieJar()))
    with promoted.open(Request(base + '/login', data=urlencode({'email': data['email'], 'password': password}).encode(),
                               headers={'Origin': base, 'Accept': 'text/html'}), timeout=30) as response:
        assert response.status == 200 and b'Installer validation' in response.read()
    report = install.verify(config)
    assert report['ready'] and report['owner'] == 'created', report
    print(json.dumps({'passed': True, 'checks': ['claim denial', 'same-origin enforcement', 'concurrent single owner', 'permanent bootstrap latch', 'password sign-in', 'workspace authorization', 'origin promotion preserves access', 'readiness']}))


if __name__ == '__main__':
    check(sys.argv[1])
