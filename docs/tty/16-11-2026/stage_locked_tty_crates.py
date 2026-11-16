"""Inventory the whole tty lock, stage all missing exact payloads; never invoke Cargo."""
import json
import os
from pathlib import Path
import stat
import tomllib
import stage_locked_crate as stage

LOCK_SHA = '9127845d654a37f4a9e0fbce59b69ef26da51c6f6d7af6a6833782fd4fecabbb'


def verify(path, sha):
    info = path.lstat()
    assert stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
    assert stage.digest(path) == sha
    return dict(uid=info.st_uid, dev=info.st_dev, ino=info.st_ino, mode=stat.S_IMODE(info.st_mode), sha256=sha)


def main():
    w = Path(__file__).resolve().parents[3]
    lock = w / 'extension/rag3weaver/tty/Cargo.lock'
    assert stage.digest(lock) == LOCK_SHA
    registry = Path(os.environ.get('CARGO_HOME', str(Path.home() / '.cargo'))) / 'registry'
    packages = [p for p in tomllib.loads(lock.read_text())['package'] if p.get('source') == 'registry+https://github.com/rust-lang/crates.io-index']
    q = Path.home() / '.cache/rag3weaver-build/cx-tty-lock-payload-preparation'
    q.mkdir(mode=0o700)
    inventory, missing = [], []
    for package in packages:
        stem = package['name'] + '-' + package['version']
        cache = registry / 'cache' / stage.REGISTRY / (stem + '.crate')
        src = registry / 'src' / stage.REGISTRY / stem
        entry = dict(package, cache=str(cache), src=str(src), cache_present=cache.exists(), source_present=src.exists())
        inventory.append(entry)
        if not cache.exists() and not src.exists():
            assert not cache.is_symlink() and not src.is_symlink()
            missing.append(entry)
    (q / 'inventory-before.json').write_text(json.dumps(dict(lock_sha=LOCK_SHA, registry_packages=inventory, missing=missing), indent=2))
    stage.LOCK_SHA, stage.LOCK_PATH = LOCK_SHA, 'extension/rag3weaver/tty/Cargo.lock'
    stage.STAGING_PREFIX = 'cx-tty-lock-crate'
    results = []
    try:
        for package in missing:
            assert stage.digest(lock) == LOCK_SHA
            stage.NAME, stage.VERSION, stage.CHECKSUM = package['name'], package['version'], package['checksum']
            destination = Path(package['cache'])
            if destination.exists() or destination.is_symlink():
                status = 'VERIFIED_CONCURRENT'
            else:
                try:
                    stage.main()
                    status = 'ADDED_ABSENT'
                except Exception:
                    if not destination.exists():
                        raise
                    verify(destination, stage.CHECKSUM)
                    status = 'VERIFIED_CONCURRENT'
            results.append(dict(name=stage.NAME, version=stage.VERSION, status=status, **verify(destination, stage.CHECKSUM)))
        remaining = [p['name'] + '-' + p['version'] for p in inventory if not Path(p['cache']).exists() and not Path(p['src']).exists()]
        assert remaining == []
        report = dict(status='TTY_LOCK_PAYLOADS_READY', lock_sha=LOCK_SHA, added_or_verified=results, missing_after=remaining, cargo_executed=False)
    except Exception as error:
        report = dict(status='STOP', results=results, error=repr(error), cargo_executed=False)
        raise
    finally:
        (q / 'report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
