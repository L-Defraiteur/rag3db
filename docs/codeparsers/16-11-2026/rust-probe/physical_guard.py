"""Contrôles physiques, adaptés des primitives TTY256 et recherche af11 ; aucune exemption de chemin."""
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
    return dict(pid=int(proc.name), start=fields[19], uid=proc.stat().st_uid,
                state=fields[0], parent=int(fields[1]))


def below(value, roots):
    value = value.removesuffix(' (deleted)')
    return any(value == root or value.startswith(root + '/') for root in roots)


def scan(proc_root, roots, groups=(), allowed_controls=()):
    """Conserver les indices déjà vus malgré une permission ultérieure ; comparer PID/start deux fois."""
    roots = [str(Path(root)) for root in roots]
    known = {(row['pid'], row['start']): row for row in allowed_controls}
    found, unknown, identities = [], [], []
    for proc in Path(proc_root).iterdir():
        if not proc.name.isdigit():
            continue
        record = {'pid': int(proc.name), 'attributed': False, 'evidence': [], 'unknown_fields': []}
        before = None
        argv = None
        def observe(field, value):
            if below(value, roots):
                record['attributed'] = True
                record['evidence'].append({'field': field, 'path': value})
        try:
            try:
                before = identity(proc)
                record.update(before)
            except PermissionError:
                record['unknown_fields'].append('identity')
            if before and before['state'] == 'Z':
                continue
            for field in ['exe', 'cwd']:
                try:
                    value = os.readlink(proc / field)
                    record[field] = value
                    observe(field, value)
                except PermissionError:
                    record['unknown_fields'].append(field)
                except (FileNotFoundError, ProcessLookupError):
                    pass
            try:
                raw = (proc / 'cmdline').read_bytes()
                argv = [os.fsdecode(value) for value in raw.split(b'\0') if value]
                record['argv_sha256'] = hashlib.sha256(raw).hexdigest()
                for argument in argv:
                    # Un indice argv ne prouve pas un accès : il suffit à refuser prudemment le réemploi.
                    for root in roots:
                        if root in argument:
                            record['attributed'] = True
                            record['evidence'].append({'field': 'argv', 'root': root})
                            break
            except PermissionError:
                record['unknown_fields'].append('argv')
            except (FileNotFoundError, ProcessLookupError):
                argv = None
            try:
                cgroup = (proc / 'cgroup').read_text().strip()
                record['cgroup'] = cgroup
                paths = [line.split(':', 2)[-1] for line in cgroup.splitlines()]
                if any(path == group or path.startswith(group.rstrip('/') + '/') for path in paths for group in groups):
                    control = known.get((before['pid'], before['start'])) if before else None
                    same = control and all(record.get(key) == control[key] for key in ['uid', 'exe', 'cgroup']) and argv == control['argv']
                    if not same:
                        record['attributed'] = True
                        record['evidence'].append({'field': 'cgroup', 'paths': paths})
            except PermissionError:
                record['unknown_fields'].append('cgroup')
            except (FileNotFoundError, ProcessLookupError):
                pass
            try:
                for fd in (proc / 'fd').iterdir():
                    try:
                        observe('fd/' + fd.name, os.readlink(fd))
                    except PermissionError:
                        record['unknown_fields'].append('fd/' + fd.name)
                    except (FileNotFoundError, ProcessLookupError):
                        pass
            except PermissionError:
                record['unknown_fields'].append('fd')
            except (FileNotFoundError, ProcessLookupError):
                pass
            try:
                after = identity(proc)
                if before and any(before[key] != after[key] for key in ['pid', 'start', 'uid']):
                    record['attributed'] = True
                    record['evidence'].append({'field': 'identity_changed', 'after': after})
            except PermissionError:
                record['unknown_fields'].append('identity_after')
            if before:
                identities.append(before)
        except PermissionError:
            record['unknown_fields'].append('identity')
        except (FileNotFoundError, ProcessLookupError):
            # Une disparition ne retire pas un indice protégé déjà collecté.
            if record['attributed']:
                record['unknown_fields'].append('disappeared_after_evidence')
        if record['unknown_fields']:
            unknown.append(record)
        if record['attributed']:
            found.append(record)
    return dict(found=found, unknown=unknown, identities=identities, observed=time.monotonic())


def process_record(pid, proc_root='/proc'):
    proc = Path(proc_root) / str(pid)
    record = identity(proc)
    record.update(exe=os.readlink(proc / 'exe'), cwd=os.readlink(proc / 'cwd'),
        argv=[os.fsdecode(v) for v in (proc / 'cmdline').read_bytes().split(b'\0') if v],
        cgroup=(proc / 'cgroup').read_text().strip())
    return record


def owned_quiet(scope, proc_root='/proc'):
    """Adapter af11 : les identités réellement observées par Common, jamais un PID nu."""
    scope = Path(scope)
    rows = json.loads((scope / 'enfants.json').read_text())
    authority = json.loads((scope / 'autorite.json').read_text())
    rows.append({'pid': authority['pid'], 'debut': authority['debut']})
    alive, unknown, ended = [], [], []
    for row in rows:
        try:
            actual = identity(Path(proc_root) / str(row['pid']))
            if actual['start'] == row['debut'] and actual['state'] != 'Z':
                alive.append(actual)
            else:
                ended.append({'expected': row, 'actual': actual})
        except (FileNotFoundError, ProcessLookupError):
            ended.append({'expected': row, 'actual': None})
        except OSError as error:
            unknown.append({'expected': row, 'error': repr(error)})
    return dict(alive=alive, unknown=unknown, ended=ended, observed=time.monotonic())


def publish_result(path, result, deadline):
    # Primitive TTY256 : horloge fraîche APRÈS open, puis après fsync ; aucun vert tardif.
    with Path(path).open('x') as stream:
        now = time.monotonic()
        result['finished'] = now
        if now >= deadline:
            result.update(verdict='ROUGE', phase_completed=False, error='deadline exceeded before publication')
        json.dump(result, stream, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
        after = time.monotonic()
        if after >= deadline:
            result.update(verdict='ROUGE', phase_completed=False, error='deadline exceeded during publication', finished=after)
            stream.seek(0)
            stream.truncate()
            json.dump(result, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
    return result
