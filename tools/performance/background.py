#!/usr/bin/env python3
"""Measure foreground listing and background progress on the dedicated local fixture."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
import re
from pathlib import Path
import time
from urllib.parse import urlencode, urlsplit
from urllib.request import Request, urlopen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--origin', required=True)
    parser.add_argument('--library', default='perf')
    parser.add_argument('--path', default='/twitter')
    parser.add_argument('--log-file', type=Path, required=True)
    parser.add_argument('--fixture-dir', type=Path, required=True)
    parser.add_argument('--snapshot-state', choices=['empty', 'unknown'], default='unknown')
    parser.add_argument('--condition', choices=['quiet', 'active'], required=True)
    parser.add_argument('--label', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    origin = urlsplit(args.origin)
    if (origin.scheme != 'http' or origin.hostname != '127.0.0.1' or not origin.port
            or not 16050 <= origin.port < 17000 or origin.path not in ('', '/')
            or origin.username or origin.password or origin.query or origin.fragment
            or args.library != 'perf' or args.path != '/twitter'):
        parser.error('only the dedicated loopback performance daemon and perf:/twitter are allowed')
    base = Path(os.environ.get('AGENT_TEMP_DIR', '/tmp')).resolve()
    fixture = args.fixture_dir.resolve()
    if not fixture.is_relative_to(base) or fixture == base:
        parser.error('fixture must be under AGENT_TEMP_DIR')
    manifest = json.loads((fixture / 'manifest.json').read_text())
    if manifest.get('folders') != 22432 or manifest.get('files') != 1:
        parser.error('fixed 22432-folder fixture required; verify with fixture.py before the suite')
    if not args.log_file.resolve().is_relative_to(base):
        parser.error('only a dedicated daemon log under AGENT_TEMP_DIR is allowed')
    token = os.environ.get('PXNA_PERF_TOKEN')
    if not token:
        parser.error('PXNA_PERF_TOKEN is required')
    headers = {'Authorization': 'Bearer ' + token}
    api = args.origin.rstrip('/') + '/api/v1/libraries/perf'

    def request(path, method='GET'):
        with urlopen(Request(api + path, headers=headers, method=method), timeout=240) as response:
            return response.status, json.load(response)

    def status():
        return request('/timeline/status')[1]

    before = status()
    if before['running']:
        raise RuntimeError('start after the startup scan has finished')
    refresh_utc = None
    if args.condition == 'active':
        refresh_utc = datetime.now(timezone.utc).isoformat()
        request('/timeline/refresh', 'POST')
        for _ in range(100):
            before = status()
            if before['running']:
                break
            time.sleep(.01)
        else:
            raise RuntimeError('background scan did not become active')
    offset = args.log_file.stat().st_size
    started_utc = datetime.now(timezone.utc).isoformat()
    started = time.perf_counter()
    query = urlencode({'path': args.path, 'sort': 'name', 'order': 'asc',
                       'foldersFirst': 'true', 'limit': 200})
    http_status, body = request('/folders/entries?' + query)
    elapsed = time.perf_counter() - started
    ended_utc = datetime.now(timezone.utc).isoformat()
    with args.log_file.open('rb') as log:
        log.seek(offset)
        raw = log.read()
    after = status()
    clean = re.sub(r'\x1b\[[0-9;]*m', '', raw.decode(errors='replace'))
    empty_eof = 0
    event_times = []
    window_start = datetime.fromisoformat(started_utc)
    window_end = datetime.fromisoformat(ended_utc)
    for line in clean.splitlines():
        if line.startswith('{'):
            try:
                event = json.loads(line)
                fields = event.get('fields', {})
                stamp = datetime.fromisoformat(event['timestamp'])
                if (fields.get('operation') == 'local_listing' and fields.get('entry_count') == 0
                        and window_start <= stamp <= window_end):
                    empty_eof += 1
                    event_times.append(event['timestamp'])
            except (json.JSONDecodeError, KeyError, ValueError):
                pass
    reasons = []
    if args.snapshot_state != 'empty':
        reasons.append('empty snapshot was not declared by the restarting suite')
    if http_status != 200 or body['totalCount'] != 22433:
        reasons.append('unexpected foreground response')
    if args.condition == 'active' and (not before['running'] or empty_eof < 1):
        reasons.append('background progress not observed inside the foreground request interval')
    if args.condition == 'quiet' and (before['running'] or after['running']):
        reasons.append('background was running in quiet condition')
    record = {'schema': 1, 'label': args.label, 'condition': args.condition,
              'refresh_utc': refresh_utc, 'started_utc': started_utc, 'ended_utc': ended_utc,
              'elapsed_sec': elapsed, 'http_status': http_status, 'entries': len(body['entries']),
              'total_count': body['totalCount'], 'snapshot': body['snapshot'],
              'declared_snapshot_state': args.snapshot_state, 'cache_layers': 'uncontrolled',
              'background_before': before, 'background_after': after,
              'empty_folder_eof_in_window': empty_eof, 'empty_eof_utc': event_times, 'window_log_sha256': hashlib.sha256(raw).hexdigest(),
              'valid_comparison': not reasons, 'invalid_reasons': reasons,
              'fixture_manifest_sha256': hashlib.sha256((fixture / 'manifest.json').read_bytes()).hexdigest()}
    with args.output.open('x') as output:
        json.dump(record, output, indent=2)
        output.write('\n')
    print(json.dumps({'label': args.label, 'elapsed_sec': elapsed,
                      'valid_comparison': not reasons, 'empty_folder_eof_in_window': empty_eof}))


if __name__ == '__main__':
    main()
