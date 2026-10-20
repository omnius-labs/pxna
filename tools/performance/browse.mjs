#!/usr/bin/env node
// Node 24+, a running Chromium with a local CDP port. No npm dependency.
import { parseArgs } from 'node:util';
import { writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

const { values } = parseArgs({ options: {
    help: { type: 'boolean' }, thumbnails: { type: 'boolean' }, url: { type: 'string' }, output: { type: 'string' },
    cdp: { type: 'string', default: 'http://127.0.0.1:9222' },
    label: { type: 'string' }, marker: { type: 'string', default: 'folder-00000' },
    timeout: { type: 'string', default: '240' },
} });
if (values.help) {
    console.log('node tools/performance/browse.mjs --url URL --label BUILD --output NEW.json [--cdp http://127.0.0.1:9222] [--marker folder-00000] [--timeout 240] [--thumbnails]\nAuthentication: PXNA_PERF_TOKEN environment variable; not persisted. Viewport: 1440 x 900.');
    process.exit(0);
}
if (!values.url || !values.output || !values.label) throw new Error('--url, --label, --output required');
const url = new URL(values.url);
if (url.username || url.password || !['http:', 'https:'].includes(url.protocol)) throw new Error('invalid URL');
const listingPath = '/' + decodeURIComponent(url.pathname.split('/').slice(3).join('/'));
const cdpUrl = new URL(values.cdp);
if (!['127.0.0.1', 'localhost', '[::1]'].includes(cdpUrl.hostname)) throw new Error('CDP must be local');
const timeoutMs = Number(values.timeout) * 1000;
if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) throw new Error('invalid timeout');
const version = await (await fetch(new URL('/json/version', cdpUrl))).json();
const target = await (await fetch(new URL('/json/new?about:blank', cdpUrl), { method: 'PUT' })).json();
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
let sequence = 0;
const pending = new Map();
ws.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    if (!message.id) return;
    const task = pending.get(message.id);
    if (!task) return;
    pending.delete(message.id);
    clearTimeout(task.timer);
    if (message.error) task.reject(new Error(message.error.message));
    else task.resolve(message.result);
};
function send(method, params = {}) {
    const id = ++sequence;
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, timeoutMs);
        pending.set(id, { resolve, reject, timer });
        ws.send(JSON.stringify({ id, method, params }));
    });
}
const instrumentation = `(() => {
    const state = window.__pxnaPerf = { longtasks: [], firstTreeRowMs: null, firstGridRowMs: null, maxRows: 0, observerMs: 0 };
    window.__pxnaPerfTileTimes = new Map();
    new PerformanceObserver(list => {
        for (const e of list.getEntries()) state.longtasks.push({ start: e.startTime, duration: e.duration });
    }).observe({ type: 'longtask', buffered: true });
    const observer = new MutationObserver(() => {
        const start = performance.now();
        const rows = [...document.querySelectorAll('[role="treeitem"]')];
        state.maxRows = Math.max(state.maxRows, rows.length);
        for (const tile of document.querySelectorAll('[role="option"][data-kind]')) {
            if (state.firstGridRowMs === null) state.firstGridRowMs = performance.now();
            const name = tile.getAttribute('title');
            if (name && !window.__pxnaPerfTileTimes.has(name)) window.__pxnaPerfTileTimes.set(name, { kind: tile.dataset.kind, firstMs: performance.now() });
        }
        if (state.firstTreeRowMs === null && rows.some(row => row.textContent.includes(${JSON.stringify(values.marker)})))
            state.firstTreeRowMs = performance.now();
        state.observerMs += performance.now() - start;
    });
    observer.observe(document, { subtree: true, childList: true });
})();`;
try {
    await send('Page.enable');
    await send('Network.enable');
    await send('Network.setCacheDisabled', { cacheDisabled: true });
    await send('Performance.enable');
    await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
    if (process.env.PXNA_PERF_TOKEN) await send('Network.setExtraHTTPHeaders', { headers: { Authorization: `Bearer ${process.env.PXNA_PERF_TOKEN}` } });
    await send('Page.addScriptToEvaluateOnNewDocument', { source: instrumentation });
    const navigation = await send('Page.navigate', { url: values.url });
    if (navigation.errorText) throw new Error('navigation failed');
    const deadline = Date.now() + timeoutMs;
    let ready = false;
    while (Date.now() < deadline) {
        await delay(100);
        const result = await send('Runtime.evaluate', { expression: 'window.__pxnaPerf?.firstTreeRowMs !== null && window.__pxnaPerf?.firstTreeRowMs !== undefined', returnByValue: true });
        if (result.result.value) { ready = true; break; }
    }
    if (!ready) throw new Error('tree marker not observed; check login and route');
    // A separate event-loop turn after observation; not a claim of completed paint.
    const probeStart = performance.now();
    await send('Runtime.evaluate', { expression: 'new Promise(resolve => setTimeout(resolve, 0))', awaitPromise: true });
    const eventLoopProbeMs = performance.now() - probeStart;
    await delay(values.thumbnails ? 2000 : 250);
    const { result, exceptionDetails } = await send('Runtime.evaluate', { expression: `JSON.stringify({
        ...window.__pxnaPerf,
        rowsNow: document.querySelectorAll('[role="treeitem"]').length,
        thumbnailRequests: ${values.thumbnails ? `performance.getEntriesByType('resource').filter(e => new URL(e.name).pathname.endsWith('/thumbnails')).map(e => {
            const requested = new URL(e.name).searchParams.get('path') || '';
            const tile = window.__pxnaPerfTileTimes.get(requested.split('/').at(-1));
            return { kind: tile?.kind || 'unobserved', domObserved: !!tile, startMs: e.startTime, responseEndMs: e.responseEnd,
                delayFromFirstGridRowMs: window.__pxnaPerf.firstGridRowMs === null ? null : e.startTime - window.__pxnaPerf.firstGridRowMs,
                delayFromOwnTileMs: tile ? e.startTime - tile.firstMs : null };
        })` : 'undefined'},
        resources: performance.getEntriesByType('resource').filter(e => ['/folders/entries', '/folders/summaries'].some(suffix => new URL(e.name).pathname.endsWith(suffix))).map(e => ({
            summary: new URL(e.name).pathname.endsWith('/summaries'),
            foldersOnly: new URL(e.name).searchParams.get('foldersOnly') === 'true',
            targetFolder: new URL(e.name).searchParams.get('path') === ${JSON.stringify(listingPath)},
            offset: Number(new URL(e.name).searchParams.get('offset') || 0), startMs: e.startTime, responseEndMs: e.responseEnd
        }))
    })`, returnByValue: true });
    if (exceptionDetails) throw new Error('metrics collection failed');
    const record = { schema: 1, utc: new Date().toISOString(), label: values.label,
        browser: version.Browser, viewport: { width: 1440, height: 900 },
        filesystemCacheState: 'uncontrolled', browserHttpCacheDisabled: true, eventLoopProbeMs,
        ...JSON.parse(result.value), cdpMetrics: (await send('Performance.getMetrics')).metrics };
    const targetRequests = record.resources.filter(r => r.foldersOnly && r.targetFolder);
    record.targetTreeRequests = targetRequests.length;
    record.afterFirstTargetResponseMs = targetRequests.length ? record.firstTreeRowMs - Math.min(...targetRequests.map(r => r.responseEndMs)) : null;
    await writeFile(values.output, JSON.stringify(record, null, 2) + '\n', { flag: 'wx' });
    console.log(JSON.stringify({ firstTreeRowMs: record.firstTreeRowMs, maxRows: record.maxRows,
        treeRequests: record.targetTreeRequests, afterFirstTargetResponseMs: record.afterFirstTargetResponseMs }));
} finally {
    ws.close();
    for (const task of pending.values()) { clearTimeout(task.timer); task.reject(new Error('CDP closed')); }
    await fetch(new URL(`/json/close/${encodeURIComponent(target.id)}`, cdpUrl));
}
