#!/usr/bin/env python3
"""Create or verify a fixed, synthetic listing fixture without touching real data."""
import argparse
import hashlib
import json
import os
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path, help='new directory under AGENT_TEMP_DIR')
    parser.add_argument('--verify', action='store_true')
    parser.add_argument('--folders', type=int, default=22432)
    args = parser.parse_args()
    base = Path(os.environ.get('AGENT_TEMP_DIR', '/tmp')).resolve()
    target = args.directory.resolve()
    if target == base or not target.is_relative_to(base):
        parser.error('directory must be strictly inside AGENT_TEMP_DIR (or /tmp)')
    if not 1 <= args.folders <= 100000:
        parser.error('--folders must be in 1..100000')
    names = [f'folder-{i:05d}' for i in range(args.folders)]
    signature = hashlib.sha256(('\n'.join(names) + '\n_meta.db\n').encode()).hexdigest()
    manifest = {'schema': 1, 'folders': args.folders, 'files': 1,
                'entry_names_sha256': signature, 'mtime_unix': 1700000000}
    library = target / 'library' / 'twitter'
    if args.verify:
        if json.loads((target / 'manifest.json').read_text()) != manifest:
            parser.error('manifest mismatch')
        actual = sorted(p.name for p in library.iterdir())
        if actual != sorted(names + ['_meta.db']):
            parser.error('entry names changed')
        for name in names:
            child = library / name
            if child.is_symlink() or not child.is_dir() or any(child.iterdir()):
                parser.error('fixture contents changed')
            if child.stat().st_mtime != manifest['mtime_unix']:
                parser.error('fixture timestamps changed')
        if (library / '_meta.db').is_symlink() or (library / '_meta.db').read_bytes() != b'':
            parser.error('fixture file changed')
    else:
        target.mkdir(parents=True, exist_ok=False)
        library.mkdir(parents=True)
        for name in names:
            child = library / name
            child.mkdir()
            os.utime(child, (manifest['mtime_unix'], manifest['mtime_unix']))
        (library / '_meta.db').touch()
        os.utime(library / '_meta.db', (manifest['mtime_unix'], manifest['mtime_unix']))
        os.utime(library, (manifest['mtime_unix'], manifest['mtime_unix']))
        (target / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest))


if __name__ == '__main__':
    main()
