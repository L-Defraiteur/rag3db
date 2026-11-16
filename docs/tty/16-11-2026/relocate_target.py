"""Relogement privé réversible ; aucune suppression ni réutilisation d'artefact."""
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess

ARCHIVE_SHA = '56aca2b2488b99a302f0e806724bf58a2677805a9413f8ca9152ac2caf9d09f3'
BASELINE_UNKNOWN = {5435, 5438, 6208, 25298}


def rename_noreplace(source, destination):
    libc = ctypes.CDLL(None, use_errno=True)
    function = libc.renameat2
    function.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    function.restype = ctypes.c_int
    if function(-100, os.fsencode(source), -100, os.fsencode(destination), 1):
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error), str(destination))


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def snapshot(root):
    rows = {}
    for path in [root, *sorted(root.rglob('*'))]:
        info = path.lstat()
        row = dict(mode=info.st_mode, uid=info.st_uid, gid=info.st_gid,
                   dev=info.st_dev, ino=info.st_ino, mtime_ns=info.st_mtime_ns)
        if stat.S_ISREG(info.st_mode):
            row['sha256'] = digest(path)
        elif stat.S_ISLNK(info.st_mode):
            row['link'] = os.readlink(path)
        elif not stat.S_ISDIR(info.st_mode):
            raise RuntimeError('unexpected filesystem object')
        rows[str(path.relative_to(root))] = row
    return rows


def scan(source, worktree):
    # Les ancêtres du contrôleur sont nécessaires à cette inspection, pas des
    # descendants survivants de l'ancien Common.
    ancestors = set()
    pid = os.getpid()
    while pid > 1 and pid not in ancestors:
        ancestors.add(pid)
        try:
            pid = int((Path('/proc') / str(pid) / 'stat').read_text().rsplit(') ', 1)[1].split()[1])
        except FileNotFoundError:
            break
    unknown, attributed = [], []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit() or int(proc.name) in ancestors:
            continue
        try:
            if proc.stat().st_uid != os.getuid():
                continue
            links = [proc / 'exe', proc / 'cwd', *list((proc / 'fd').iterdir())]
            for link in links:
                try:
                    value = os.readlink(link).removesuffix(' (deleted)')
                except FileNotFoundError:
                    continue
                if any(value == str(root) or value.startswith(str(root) + '/') for root in [source, worktree]):
                    attributed.append(dict(pid=int(proc.name), link=str(link), path=value))
        except FileNotFoundError:
            continue
        except PermissionError:
            unknown.append(int(proc.name))
    return dict(unknown=sorted(unknown), attributed=attributed,
                controller_ancestors=sorted(ancestors))


def main():
    base = Path.home() / '.cache/rag3weaver-build'
    source = base / 'target-cx-tty-english-aliases-2'
    archive = base / 'archives/cx-tty-aliases-red-20261116'
    destination = archive / 'retired-target'
    worktree = Path.home() / 'git_workspaces/cx-tty-english-aliases'
    report = archive / 'relocation-atomic.json'
    assert not report.exists() and not report.is_symlink()
    with report.open('x') as output:
        evidence = dict(status='STOP', source=str(source), destination=str(destination))
        try:
            for path in [source, archive, worktree]:
                assert path.resolve() == path and not path.is_symlink()
            info, parent = source.lstat(), archive.lstat()
            assert stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid()
            assert stat.S_ISDIR(parent.st_mode) and parent.st_uid == os.getuid()
            assert info.st_dev == parent.st_dev
            assert not destination.exists() and not destination.is_symlink()
            evidence['source_identity'] = [info.st_uid, info.st_dev, info.st_ino]
            evidence['parent_identity'] = [parent.st_uid, parent.st_dev, parent.st_ino]
            assert digest(archive / 'target-and-proof.tar') == ARCHIVE_SHA
            remote = subprocess.run(['ssh', 'luciepc', 'sha256sum', '.cache/rag3weaver-build/archives/cx-tty-aliases-red-20261116/target-and-proof.tar'], capture_output=True, text=True, timeout=30)
            evidence['remote_archive'] = dict(rc=remote.returncode, stdout=remote.stdout, stderr=remote.stderr)
            assert remote.returncode == 0 and remote.stdout.split()[0] == ARCHIVE_SHA
            authority = json.loads((base / 'validations/validation-g4dqpr6q/autorite.json').read_text())
            verdict = json.loads((base / 'validations/validation-g4dqpr6q/verdict.json').read_text())
            assert authority['etat'] == 'termine' and verdict['restes'] == [] and verdict['nettoye']
            assert not (Path('/proc') / str(authority['pid'])).exists()
            evidence['scan'] = scan(source, worktree)
            assert not evidence['scan']['attributed']
            assert set(evidence['scan']['unknown']) <= BASELINE_UNKNOWN
            before = snapshot(source)
            evidence['before'] = before
            assert snapshot(source) == before
            fresh_parent = archive.lstat()
            assert (fresh_parent.st_uid, fresh_parent.st_dev, fresh_parent.st_ino) == (parent.st_uid, parent.st_dev, parent.st_ino)
            rename_noreplace(source, destination)
            evidence['renamed'] = True
            after = snapshot(destination)
            evidence['after'] = after
            assert before == after and not source.exists()
            evidence['status'] = 'RELOCATED_IDENTICAL'
        except Exception as error:
            evidence['error'] = repr(error)
            raise
        finally:
            output.write(json.dumps(evidence, indent=2))
    print('RELOCATED_IDENTICAL', report)


if __name__ == '__main__':
    main()
