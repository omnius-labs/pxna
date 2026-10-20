#!/usr/bin/env python3
"""Time representative thumbnails on a dedicated local fixture; never save paths or tokens."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import time
from urllib.error import HTTPError
from urllib.parse import urlencode, urlsplit
from urllib.request import Request, urlopen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--origin', required=True)
    parser.add_argument('--path', default='/twitter')
    parser.add_argument('--log-file', type=Path, required=True)
    parser.add_argument('--label', required=True)
    parser.add_argument('--expected-status', type=int, choices=[200, 404], default=200)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    origin = urlsplit(args.origin)
    if (origin.scheme != 'http' or origin.hostname != '127.0.0.1' or not origin.port
            or not 16050 <= origin.port < 17000 or origin.path not in ('', '/')
            or origin.username or origin.password or origin.query or origin.fragment
            or args.path != '/twitter'):
        parser.error('dedicated loopback performance daemon and perf:/twitter required')
    base = Path(os.environ.get('AGENT_TEMP_DIR', '/tmp')).resolve()
    if not args.log_file.resolve().is_relative_to(base):
        parser.error('daemon log must be under AGENT_TEMP_DIR')
    token = os.environ.get('PXNA_PERF_TOKEN')
    if not token:
        parser.error('PXNA_PERF_TOKEN required')
    query = urlencode({'path': args.path, 'variant': 'thumbnail', 'width': 128, 'height': 128, 'resize': 'crop'})
    url = args.origin.rstrip('/') + '/api/v1/libraries/perf/thumbnails?' + query
    offset = args.log_file.stat().st_size
    start_utc = datetime.now(timezone.utc)
    started = time.perf_counter()
    try:
        response = urlopen(Request(url, headers={'Authorization': 'Bearer ' + token}), timeout=240)
    except HTTPError as error:
        response = error
    with response:
        body = response.read()
        status = response.status
        content_type = response.headers.get('Content-Type')
        etag = response.headers.get('ETag')
    elapsed = time.perf_counter() - started
    end_utc = datetime.now(timezone.utc)
    # Let a buffered logger publish events; this wait is outside the measured request.
    time.sleep(.03)
    with args.log_file.open('rb') as log:
        log.seek(offset)
        raw = log.read()
    selection = []
    image_cache = []
    production = []
    cache_stages = []
    for line in raw.decode(errors='replace').splitlines():
        try:
            event = json.loads(line)
            stamp = datetime.fromisoformat(event['timestamp'])
            fields = event['fields']
        except (json.JSONDecodeError, KeyError, ValueError):
            continue
        if not start_utc <= stamp <= end_utc:
            continue
        if fields.get('operation') == 'representative_selection':
            selection.append({key: fields[key] for key in ['cache_hit', 'has_candidate', 'entry_count', 'elapsed_sec']})
        elif fields.get('operation') in ['thumbnail_cache_lookup', 'thumbnail_cache_read', 'thumbnail_cache_touch', 'representative_metadata']:
            cache_stages.append({'operation': fields['operation'], 'elapsed_sec': fields['elapsed_sec']})
        elif fields.get('operation') == 'thumbnail_production':
            production.append({'elapsed_sec': fields['elapsed_sec']})
        elif fields.get('operation') == 'thumbnail_image_cache':
            image_cache.append({'hit': fields['hit'], 'eligible': fields.get('eligible')})
    reasons = []
    if status != args.expected_status:
        reasons.append('unexpected status')
    if len(selection) != 1:
        reasons.append('expected exactly one representative-selection event')
    record = {'schema': 1, 'label': args.label, 'started_utc': start_utc.isoformat(),
              'ended_utc': end_utc.isoformat(), 'elapsed_sec': elapsed, 'status': status,
              'content_type': content_type, 'body_bytes': len(body), 'body_sha256': hashlib.sha256(body).hexdigest(),
              'etag': etag, 'selection': selection, 'image_cache': image_cache, 'production': production, 'cache_stages': cache_stages,
              'cache_layers': 'uncontrolled', 'valid': not reasons, 'invalid_reasons': reasons,
              'window_log_sha256': hashlib.sha256(raw).hexdigest()}
    with args.output.open('x') as output:
        json.dump(record, output, indent=2)
        output.write('\n')
    print(json.dumps({'label': args.label, 'elapsed_sec': elapsed, 'status': status,
                      'selection': selection, 'image_cache': image_cache, 'production': production, 'cache_stages': cache_stages, 'valid': not reasons}))


if __name__ == '__main__':
    main()
