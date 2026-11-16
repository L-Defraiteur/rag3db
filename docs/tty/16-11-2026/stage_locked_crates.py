"""The five named lockfile entries only; no Cargo invocation or broad fetch."""
import json
import os
from pathlib import Path
import stat
import tomllib
import stage_locked_crate as stage

NAMES = ['ld-lucivy', 'luciole', 'lucistore', 'lucivy-core', 'sparse-vector']


def existing(path, checksum):
    info = path.lstat()
    assert stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
    assert stage.digest(path) == checksum
    return dict(uid=info.st_uid, dev=info.st_dev, ino=info.st_ino,
                mode=stat.S_IMODE(info.st_mode), sha256=checksum)


def main():
    w = Path(__file__).resolve().parents[3]
    lock = w / 'extension/rag3weaver/Cargo.lock'
    assert stage.digest(lock) == stage.LOCK_SHA
    packages = tomllib.loads(lock.read_text())['package']
    cargo = Path(os.environ.get('CARGO_HOME', str(Path.home() / '.cargo')))
    registry = cargo / 'registry'
    report_dir = Path.home() / '.cache/rag3weaver-build/cx-tty-five-crates-preparation'
    report_dir.mkdir(mode=0o700)
    report = dict(lock_sha=stage.LOCK_SHA, packages=[], cargo_executed=False)
    try:
        for name in NAMES:
            assert stage.digest(lock) == stage.LOCK_SHA
            package = next(p for p in packages if p['name'] == name and p['version'] == '4.3.9')
            assert package['source'] == 'registry+https://github.com/rust-lang/crates.io-index'
            destination = registry / 'cache' / stage.REGISTRY / f'{name}-4.3.9.crate'
            if destination.exists() or destination.is_symlink():
                info = existing(destination, package['checksum'])
                report['packages'].append(dict(name=name, status='VERIFIED_EXISTING', **info))
                continue
            stage.NAME, stage.CHECKSUM = name, package['checksum']
            try:
                stage.main()
                status = 'ADDED_ABSENT'
            except Exception:
                # A legitimate concurrent cache publisher must never be overwritten.
                if not destination.exists():
                    raise
                existing(destination, package['checksum'])
                status = 'VERIFIED_CONCURRENT'
            info = existing(destination, package['checksum'])
            report['packages'].append(dict(name=name, status=status, **info))
        missing = []
        for package in packages:
            if package.get('source') != 'registry+https://github.com/rust-lang/crates.io-index':
                continue
            name = package['name'] + '-' + package['version']
            cache = registry / 'cache' / stage.REGISTRY / (name + '.crate')
            src = registry / 'src' / stage.REGISTRY / name
            if not cache.exists() and not src.exists():
                missing.append(name)
        report['missing_locked_payloads'] = missing
        report['status'] = 'FIVE_VERIFIED'
    except Exception as error:
        report.update(status='STOP', error=repr(error))
        raise
    finally:
        (report_dir / 'report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
