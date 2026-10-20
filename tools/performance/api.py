#!/usr/bin/env python3
"""Time read-only listing requests. Never save credentials or entry names."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import statistics
import time
from urllib.parse import urlsplit
from urllib.request import Request, urlopen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--url', required=True, help='listing API URL (no credentials)')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--label', required=True, help='build/condition identifier')
    parser.add_argument('--snapshot-state', choices=['empty', 'reuse', 'unknown'], default='unknown')
    parser.add_argument('--timeout', type=float, default=240)
    parser.add_argument('--runs', type=int, default=1)
    args = parser.parse_args()
    parsed = urlsplit(args.url)
    if parsed.scheme not in ('http', 'https') or parsed.username or parsed.password:
        parser.error('URL must be HTTP(S), without embedded credentials')
    if not 1 <= args.runs <= 100 or args.timeout <= 0:
        parser.error('invalid runs or timeout')
    if args.snapshot_state == 'empty' and args.runs != 1:
        parser.error('an empty snapshot must be measured once after each independent restart')
    headers = {}
    if token := os.environ.get('PXNA_PERF_TOKEN'):
        headers['Authorization'] = 'Bearer ' + token
    results = []
    for _ in range(args.runs):
        start = time.perf_counter()
        with urlopen(Request(args.url, headers=headers), timeout=args.timeout) as response:
            body = json.load(response)
            status = response.status
        results.append({'elapsed_sec': time.perf_counter() - start, 'status': status,
                        'entries': len(body['entries']), 'total_count': body['totalCount'],
                        'snapshot': body['snapshot'], 'truncated': body['truncated']})
    record = {'schema': 1, 'utc': datetime.now(timezone.utc).isoformat(), 'label': args.label,
              'snapshot_state': args.snapshot_state, 'filesystem_cache_state': 'uncontrolled',
              'runs': results, 'median_sec': statistics.median(r['elapsed_sec'] for r in results)}
    with args.output.open('x') as output:
        json.dump(record, output, indent=2)
        output.write('\n')
    print(json.dumps({'runs': len(results), 'median_sec': record['median_sec']}))


if __name__ == '__main__':
    main()
