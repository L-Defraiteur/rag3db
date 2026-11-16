"""Acquire only the explicitly authorized locked crate, never compile or overwrite."""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import tarfile
import tomllib
import urllib.request
from relocate_target import rename_noreplace

NAME = 'ld-lucivy'
VERSION = '4.3.9'
CHECKSUM = '24424a98439ff03e1fb75fc749a4d88d720ca5b9084b6c1a885dcd6cfe99afb7'
LOCK_SHA = '090cc4e1330dc694127c0f08330d25acc656699763079400b502ba6607093fa7'
REGISTRY = 'index.crates.io-1949cf8c6b5b557f'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    w = Path(__file__).resolve().parents[3]
    lock = w / 'extension/rag3weaver/Cargo.lock'
    assert digest(lock) == LOCK_SHA
    package = next(p for p in tomllib.loads(lock.read_text())['package'] if p['name'] == NAME and p['version'] == VERSION)
    assert package['source'] == 'registry+https://github.com/rust-lang/crates.io-index' and package['checksum'] == CHECKSUM
    cargo = Path(os.environ.get('CARGO_HOME', str(Path.home() / '.cargo')))
    cache = cargo / 'registry/cache' / REGISTRY
    source = cargo / 'registry/src' / REGISTRY / f'{NAME}-{VERSION}'
    destination = cache / f'{NAME}-{VERSION}.crate'
    assert cache.resolve() == cache and cache.stat().st_uid == os.getuid()
    assert not destination.exists() and not destination.is_symlink()
    assert not source.exists() and not source.is_symlink()
    staging = Path.home() / '.cache/rag3weaver-build/cx-tty-crate-ld-lucivy-439'
    staging.mkdir(mode=0o700)
    proof = dict(package=package, lock_sha=LOCK_SHA, destination=str(destination),
                 before=dict(cache_absent=True, source_absent=True))
    try:
        url = f'https://static.crates.io/crates/{NAME}/{NAME}-{VERSION}.crate'
        with urllib.request.urlopen(url, timeout=45) as response:
            assert response.url == url
            data = response.read(32 * 1024**2 + 1)
        assert len(data) <= 32 * 1024**2
        payload = staging / f'{NAME}-{VERSION}.crate'
        with payload.open('xb') as output:
            output.write(data)
        assert digest(payload) == CHECKSUM
        unpacked = staging / 'unpacked'
        unpacked.mkdir(mode=0o700)
        seen, expanded = set(), 0
        files = {}
        with tarfile.open(payload, mode='r:gz') as archive:
            members = archive.getmembers()
            assert len(members) <= 20000
            for member in members:
                path = PurePosixPath(member.name)
                assert not path.is_absolute() and '..' not in path.parts
                assert path.parts and path.parts[0] == f'{NAME}-{VERSION}'
                assert member.name not in seen
                seen.add(member.name)
                assert member.isdir() or member.isfile(), 'links/devices refused'
                assert not member.mode & 0o7000
                expanded += member.size
                assert expanded <= 256 * 1024**2
                target = unpacked.joinpath(*path.parts)
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with archive.extractfile(member) as input_file, target.open('xb') as output:
                        shutil.copyfileobj(input_file, output)
                    target.chmod(0o755 if member.mode & 0o111 else 0o644)
                    files[member.name] = dict(size=target.stat().st_size, mode=stat.S_IMODE(target.stat().st_mode), sha256=digest(target))
        manifest = tomllib.loads((unpacked / f'{NAME}-{VERSION}/Cargo.toml').read_text())
        assert manifest['package']['name'] == NAME and manifest['package']['version'] == VERSION
        candidate = staging / 'verified-cache-candidate.crate'
        shutil.copyfile(payload, candidate)
        candidate.chmod(0o644)
        assert candidate.stat().st_dev == cache.stat().st_dev and digest(candidate) == CHECKSUM
        assert not destination.exists() and not destination.is_symlink()
        assert not source.exists() and not source.is_symlink()
        rename_noreplace(candidate, destination)
        info = destination.lstat()
        assert stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and digest(destination) == CHECKSUM
        proof.update(status='CACHE_ENTRY_ADDED', url=url, payload_sha=CHECKSUM, bytes=len(data), files=files,
                     after=dict(uid=info.st_uid, dev=info.st_dev, ino=info.st_ino, mode=stat.S_IMODE(info.st_mode), source_absent=not source.exists()))
    except Exception as error:
        proof.update(status='STOP', error=repr(error))
        raise
    finally:
        (staging / 'proof.json').write_text(json.dumps(proof, indent=2))
    print('CACHE_ENTRY_ADDED', destination, CHECKSUM)


if __name__ == '__main__':
    main()
