"""Strict target provenance and process links; no PID or ancestor exemption."""
import hashlib
import json
import os
from pathlib import Path
import time


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def identity(proc):
    fields = (proc / 'stat').read_text().rsplit(') ', 1)[1].split()
    return dict(pid=int(proc.name), start=fields[19], uid=proc.stat().st_uid)


def scan(proc_root, roots):
    found, unknown, identities = [], [], []
    roots = [str(Path(root)) for root in roots]
    for proc in Path(proc_root).iterdir():
        if not proc.name.isdigit():
            continue
        before = None
        attributed = False
        try:
            before = identity(proc)
            if before['uid'] != os.getuid():
                continue
            links = [proc / 'exe', proc / 'cwd', *list((proc / 'fd').iterdir())]
            for link in links:
                try:
                    value = os.readlink(link).removesuffix(' (deleted)')
                except (FileNotFoundError, ProcessLookupError):
                    continue
                if any(value == root or value.startswith(root + '/') for root in roots):
                    attributed = True
                    found.append(dict(**before, field=str(link.relative_to(proc)), path=value))
            after = identity(proc)
            if before != after:
                found.append(dict(before=before, after=after, reason='PID identity changed during scan'))
            identities.append(before)
        except PermissionError:
            unknown.append(dict(identity=before, pid=int(proc.name), attributed=attributed))
            if attributed:
                found.append(unknown[-1])
        except (FileNotFoundError, ProcessLookupError):
            continue
    return dict(found=found, unknown=unknown, identities=identities, observed=time.monotonic())


def snapshot(root):
    root = Path(root)
    assert root.resolve() == root and root.stat().st_uid == os.getuid()
    rows = {}
    for path in [root, *sorted(root.rglob('*'))]:
        st = path.lstat()
        row = dict(uid=st.st_uid, gid=st.st_gid, dev=st.st_dev, ino=st.st_ino,
                   mode=st.st_mode, mtime=st.st_mtime_ns)
        if path.is_symlink():
            row['link'] = os.readlink(path)
        elif path.is_file():
            row['sha256'] = digest(path)
        rows[str(path.relative_to(root))] = row
    return rows


def require_provenance(root, expected):
    actual = snapshot(root)
    if actual != expected:
        raise ValueError('target differs from frozen preparation ledger')
    return actual


def finish(authority, scope, driver, now, end, hashes_match):
    assert authority['etat'] == 'termine'
    assert scope['verdict'] == 'VERT' and scope['code'] == 0
    assert scope['restes'] == [] and scope['nettoye']
    assert driver == 'VERT' and hashes_match
    assert now < end, 'original deadline exceeded'
    return 'VERT'
