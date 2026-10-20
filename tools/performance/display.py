#!/usr/bin/env python3
"""Time viewer display images on the display fixture; never save paths of real data or tokens."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import http.client
import json
import os
from pathlib import Path
import statistics
import threading
import time
from urllib.parse import quote, urlencode, urlsplit

LIBRARY = 'perf'
OPERATIONS = ['thumbnail_generation', 'thumbnail_production', 'thumbnail_materialize', 'thumbnail_convert',
              'thumbnail_store', 'thumbnail_cache_lookup', 'thumbnail_cache_read', 'thumbnail_image_cache']
# initial and cached use the first five files of each group; flip uses ten consecutive files per interval.
INITIAL = 5
FLIP_STEPS = 10
GROUPS = {
    'local': {'initial': ['/local/L%02d.jpg' % i for i in range(INITIAL)],
              'flip': ['/flip/F%02d.jpg' % i for i in range(20)]},
    'archive': {'initial': ['/arch.zip/A%02d.jpg' % i for i in range(INITIAL)],
                'flip': ['/arch.zip/B%02d.jpg' % i for i in range(20)]},
}


def fetch(connection_factory, token, route, query):
    """One request; reads the whole body. Times are seconds from the request start."""
    connection = connection_factory()
    started = time.perf_counter()
    connection.request('GET', route + '?' + urlencode(query, quote_via=quote),
                       headers={'Authorization': 'Bearer ' + token})
    response = connection.getresponse()
    first = time.perf_counter() - started
    size = 0
    while chunk := response.read(1 << 20):
        size += len(chunk)
    elapsed = time.perf_counter() - started
    status = response.status
    connection.close()
    return {'status': status, 'bytes': size, 'first_byte_sec': first, 'elapsed_sec': elapsed,
            'started_perf': started, 'finished_perf': started + elapsed}


def read_events(log_file, offset, start_utc, end_utc):
    # Let a buffered logger publish events; this wait is outside the measured request.
    time.sleep(.05)
    with log_file.open('rb') as log:
        log.seek(offset)
        raw = log.read()
    events = []
    for line in raw.decode(errors='replace').splitlines():
        try:
            event = json.loads(line)
            stamp = datetime.fromisoformat(event['timestamp'])
            fields = event['fields']
        except (json.JSONDecodeError, KeyError, ValueError):
            continue
        if not start_utc <= stamp <= end_utc or fields.get('operation') not in OPERATIONS:
            continue
        item = {'operation': fields['operation']}
        for key in ('elapsed_sec', 'hit', 'eligible'):
            if key in fields:
                item[key] = fields[key]
        events.append(item)
    return events


def summarize(values):
    if not values:
        return None
    return {'count': len(values), 'median': statistics.median(values), 'min': min(values), 'max': max(values)}


def operation_summary(events):
    result = {}
    for operation in OPERATIONS:
        values = [e['elapsed_sec'] for e in events if e['operation'] == operation and 'elapsed_sec' in e]
        if values:
            result[operation] = summarize(values)
    hits = [e['hit'] for e in events if e['operation'] == 'thumbnail_image_cache']
    if hits:
        result['image_cache'] = {'hit': sum(hits), 'miss': len(hits) - sum(hits)}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--origin', required=True)
    parser.add_argument('--case', choices=['initial', 'cached', 'flip'], required=True)
    parser.add_argument('--log-file', type=Path, required=True)
    parser.add_argument('--label', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--viewport', default='1440x900', help='CSS size of the viewer, WxH')
    parser.add_argument('--pixel-ratio', type=float, default=2.0)
    parser.add_argument('--repeats', type=int, default=5, help='cached: repeats per file')
    parser.add_argument('--intervals-ms', default='1000,300', help='flip: step intervals, one sequence each')
    args = parser.parse_args()
    origin = urlsplit(args.origin)
    if (origin.scheme != 'http' or origin.hostname != '127.0.0.1' or not origin.port
            or not 16050 <= origin.port < 17000 or origin.path not in ('', '/')
            or origin.username or origin.password or origin.query or origin.fragment):
        parser.error('dedicated loopback performance daemon required')
    base = Path(os.environ.get('AGENT_TEMP_DIR', '/tmp')).resolve()
    if not args.log_file.resolve().is_relative_to(base):
        parser.error('daemon log must be under AGENT_TEMP_DIR')
    if args.output.exists():
        parser.error('output exists; never overwrite a record')
    token = os.environ.get('PXNA_PERF_TOKEN')
    if not token:
        parser.error('PXNA_PERF_TOKEN required')
    width, height = (int(v) for v in args.viewport.split('x'))
    side = lambda css: min(4096, max(1, round(css * max(1.0, args.pixel_ratio))))
    size = {'width': side(width), 'height': side(height)}
    prefix = f'/api/v1/libraries/{LIBRARY}/'
    port = origin.port

    def connect():
        return http.client.HTTPConnection('127.0.0.1', port, timeout=240)

    def display(path):
        query = {'path': path, 'variant': 'display', 'width': size['width'], 'height': size['height'], 'resize': 'fit'}
        return fetch(connect, token, prefix + 'thumbnails', query)

    def original(path):
        return fetch(connect, token, prefix + 'files/content', {'path': path})

    offset = args.log_file.stat().st_size
    case_start = datetime.now(timezone.utc)
    result = {'schema': 1, 'label': args.label, 'case': args.case, 'request_size': size,
              'pixel_ratio': args.pixel_ratio, 'groups': {}}

    def check(response):
        if response['status'] != 200:
            raise SystemExit(f'unexpected status {response["status"]}')
        return response

    if args.case == 'initial':
        for group, files in GROUPS.items():
            runs = []
            for path in files['initial']:
                start_utc = datetime.now(timezone.utc)
                response = check(display(path))
                end_utc = datetime.now(timezone.utc)
                events = read_events(args.log_file, offset, start_utc, end_utc)
                runs.append({'order': len(runs) + 1, **{k: response[k] for k in ('bytes', 'first_byte_sec', 'elapsed_sec')},
                             'events': events})
            result['groups'][group] = {'runs': runs, 'elapsed_sec': summarize([r['elapsed_sec'] for r in runs])}
    elif args.case == 'cached':
        for group, files in GROUPS.items():
            for path in files['initial']:
                check(display(path))  # warm-up: not recorded as a measurement
            hits, swaps = [], []
            for repeat in range(args.repeats):
                for path in files['initial']:
                    start_utc = datetime.now(timezone.utc)
                    response = check(display(path))
                    end_utc = datetime.now(timezone.utc)
                    events = read_events(args.log_file, offset, start_utc, end_utc)
                    hits.append({'repeat': repeat + 1, **{k: response[k] for k in ('bytes', 'first_byte_sec', 'elapsed_sec')},
                                 'image_cache_hit': [e['hit'] for e in events if e['operation'] == 'thumbnail_image_cache']})
            for repeat in range(args.repeats):
                for path in files['initial']:
                    response = check(original(path))
                    swaps.append({'repeat': repeat + 1, **{k: response[k] for k in ('bytes', 'first_byte_sec', 'elapsed_sec')}})
            result['groups'][group] = {
                'display_hit': {'runs': hits, 'elapsed_sec': summarize([r['elapsed_sec'] for r in hits]),
                                'all_cache_hits': all(r['image_cache_hit'] == [True] for r in hits)},
                'original': {'runs': swaps, 'elapsed_sec': summarize([r['elapsed_sec'] for r in swaps])}}
    else:
        # Mirror the viewer: opening an image requests it, and the previous and next images are requested ahead.
        intervals = [int(v) for v in args.intervals_ms.split(',')]
        for group, files in GROUPS.items():
            result['groups'][group] = {}
            for sequence, interval in enumerate(intervals):
                paths = files['flip'][sequence * FLIP_STEPS:(sequence + 1) * FLIP_STEPS]
                lock = threading.Lock()
                futures = {}
                requested_at = {}
                pool = ThreadPoolExecutor(max_workers=8)

                def request(path):
                    with lock:
                        if path not in futures:
                            requested_at[path] = time.perf_counter()
                            futures[path] = pool.submit(display, path)
                        return futures[path]

                origin_time = time.perf_counter()
                steps = []
                for index, path in enumerate(paths):
                    due = origin_time + index * interval / 1000
                    while (remaining := due - time.perf_counter()) > 0:
                        time.sleep(min(remaining, .01))
                    step_time = time.perf_counter()
                    already = path in futures
                    future = request(path)
                    for neighbor in (index + 1, index - 1):
                        if 0 <= neighbor < len(paths):
                            request(paths[neighbor])
                    response = check(future.result())
                    wait = max(0.0, response['finished_perf'] - step_time)
                    steps.append({'step': index + 1, 'prefetched': already,
                                  'wait_sec': wait, 'request_sec': response['elapsed_sec'],
                                  'request_started_before_step_sec': max(0.0, step_time - requested_at[path])})
                for future in list(futures.values()):
                    check(future.result())
                pool.shutdown()
                result['groups'][group][f'interval_{interval}ms'] = {
                    'steps': steps,
                    'first_wait_sec': steps[0]['wait_sec'],
                    'later_wait_sec': summarize([s['wait_sec'] for s in steps[1:]]),
                    'later_request_sec': summarize([s['request_sec'] for s in steps[1:]])}
    end_utc = datetime.now(timezone.utc)
    events = read_events(args.log_file, offset, case_start, end_utc)
    result['daemon_operations'] = operation_summary(events)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: result[k] for k in ('label', 'case')}))


if __name__ == '__main__':
    main()
