#!/usr/bin/env python3
"""Create a fixed, synthetic photo fixture for display-image latency; never touches real data."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

SIZE = '6000x4000'
LOCAL = 10
FLIP = 20
ARCHIVE_INITIAL = 10
ARCHIVE_FLIP = 20
MTIME = 1700000000


def names(prefix, count):
    return [f'{prefix}{i:02d}.jpg' for i in range(count)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path, help='new directory under AGENT_TEMP_DIR')
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    base = Path(os.environ.get('AGENT_TEMP_DIR', '/tmp')).resolve()
    target = args.directory.resolve()
    if target == base or not target.is_relative_to(base):
        parser.error('directory must be strictly inside AGENT_TEMP_DIR (or /tmp)')
    library = target / 'library'
    expected = {
        'local': names('L', LOCAL),
        'flip': names('F', FLIP),
        'arch.zip': names('A', ARCHIVE_INITIAL) + names('B', ARCHIVE_FLIP),
    }
    if args.verify:
        manifest = json.loads((target / 'manifest.json').read_text())
        image = (library / 'local' / 'L00.jpg').read_bytes()
        if hashlib.sha256(image).hexdigest() != manifest['image_sha256']:
            parser.error('image changed')
        if sorted(p.name for p in (library / 'local').iterdir()) != sorted(expected['local']):
            parser.error('local entries changed')
        if sorted(p.name for p in (library / 'flip').iterdir()) != sorted(expected['flip']):
            parser.error('flip entries changed')
        with zipfile.ZipFile(library / 'arch.zip') as archive:
            if sorted(archive.namelist()) != sorted(expected['arch.zip']):
                parser.error('archive entries changed')
        print(json.dumps(manifest))
        return
    target.mkdir(parents=True, exist_ok=False)
    (library / 'local').mkdir(parents=True)
    (library / 'flip').mkdir()
    source = target / 'base.jpg'
    # Noise keeps the file the size of a real photo; the content has no meaning.
    subprocess.run(['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i',
                    f'testsrc2=size={SIZE}:rate=1,noise=alls=30:allf=t', '-frames:v', '1', '-q:v', '3', str(source)],
                   check=True)
    data = source.read_bytes()
    for name in expected['local']:
        shutil.copyfile(source, library / 'local' / name)
    for name in expected['flip']:
        shutil.copyfile(source, library / 'flip' / name)
    with zipfile.ZipFile(library / 'arch.zip', 'w', zipfile.ZIP_STORED) as archive:
        for name in expected['arch.zip']:
            info = zipfile.ZipInfo(name, (2023, 11, 14, 22, 13, 20))
            archive.writestr(info, data)
    source.unlink()
    for path in [*library.rglob('*'), library]:
        os.utime(path, (MTIME, MTIME))
    ffmpeg = subprocess.run(['ffmpeg', '-version'], capture_output=True, text=True).stdout.splitlines()[0]
    manifest = {'schema': 1, 'size': SIZE, 'bytes': len(data), 'image_sha256': hashlib.sha256(data).hexdigest(),
                'local': LOCAL, 'flip': FLIP, 'archive': ARCHIVE_INITIAL + ARCHIVE_FLIP, 'ffmpeg': ffmpeg}
    (target / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest))


if __name__ == '__main__':
    main()
