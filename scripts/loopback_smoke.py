#!/usr/bin/env python3
"""Both localhost address families must reach the gateway, never a TCP self-connect."""
import errno
import http.client
import json
import socket
from dev import configuration


def check_loopback(port):
    for family, address in [(socket.AF_INET, '127.0.0.1'), (socket.AF_INET6, '::1')]:
        connection = http.client.HTTPConnection(address, port, timeout=5)
        try:
            connection.request('GET', '/health/live')
            response = connection.getresponse()
            assert response.status == 200 and json.loads(response.read()) == {'status': 'ok'}, address
        finally:
            connection.close()
        # A client cannot select the listener's local endpoint as its source.
        # Without an IPv6 listener, macOS can connect ::1:port back to itself,
        # returning the HTTP request as the apparent response.
        with socket.socket(family, socket.SOCK_STREAM) as candidate:
            try:
                candidate.bind((address, port))
            except OSError as error:
                assert error.errno == errno.EADDRINUSE, (address, error.errno)
            else:
                raise AssertionError(f'Gateway does not reserve {address}:{port}; TCP self-connect is possible')


if __name__ == '__main__':
    check_loopback(int(configuration()['LOOSKI_HTTP_PORT']))
    print('PASS: IPv4 and IPv6 loopback HTTP and self-connect protection')
