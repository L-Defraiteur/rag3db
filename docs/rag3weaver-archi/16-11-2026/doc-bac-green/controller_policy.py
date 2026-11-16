"""Exact controller policy and fresh process capture; no blanket ancestor exemption."""
from pathlib import Path
import hashlib, json, os, shutil, subprocess, sys, time

# Filled from the actual launcher assignments, in both execution and tests.
R = W = C = T = OLD_W = P = Q = REL = None
OUTER_SECONDS = 780

def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

def save(name, value):
    with (P / name).open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')
        stream.flush(); os.fsync(stream.fileno())

def commands(control, pilot):
    # The launcher and the policy share these actual command assignments.
    admit = [sys.executable, str(Q / 'qualify_green.py'), '--admit', pilot]
    poste = [str(C / 'scripts/poste'), 'lourd', *admit]
    outer = ['timeout', '--signal=TERM', '--kill-after=15s', str(OUTER_SECONDS)+'s',
             sys.executable, str(Q / 'qualify_green.py'), '--arm', pilot]
    return dict(admit=admit, poste=poste, outer=outer,
                launch=[str(control / REL), '--launch', pilot])

def policy(control, pilot, launch_identity=None):
    actual = commands(control, pilot)
    return dict(roles={
        'launch': dict(argv=['python3', *actual['launch']], exe=str(Path(sys.executable).resolve())),
        'poste': dict(argv=['/bin/sh', *actual['poste']], exe=str(Path('/bin/sh').resolve())),
        'watchdog': dict(argv=actual['outer'], exe=str(Path(shutil.which('timeout')).resolve()))},
        outer_command=actual['outer'], launch_identity=launch_identity, cwd='/var/tmp', uid=os.getuid(),
        script=str(C / 'scripts/poste'), script_sha256=sha(C / 'scripts/poste'))

def rooted(value, roots):
    return any(value == str(root) or value.startswith(str(root) + '/') for root in roots)

def snapshot(proc):
    uid = proc.stat().st_uid
    fields = (proc / 'stat').read_text().rsplit(') ', 1)[1].split()
    raw = (proc / 'cmdline').read_bytes()
    exe = os.readlink(proc / 'exe')
    row = dict(pid=int(proc.name), start=fields[19], ppid=int(fields[1]), state=fields[0],
               uid=uid, cgroup=(proc / 'cgroup').read_text(), exe=exe,
               exe_sha256=sha(proc / 'exe'), cwd=os.readlink(proc / 'cwd'),
               argv=[os.fsdecode(v) for v in raw.split(b'\0') if v],
               cmdline_hex=raw.hex(), fds=[], fd_errors=[])
    for fd in (proc / 'fd').iterdir():
        try: row['fds'].append(dict(fd=fd.name, path=os.readlink(fd)))
        except (FileNotFoundError, ProcessLookupError): continue
        except PermissionError as error: row['fd_errors'].append(dict(fd=fd.name, error=repr(error)))
    end = (proc / 'stat').read_text().rsplit(') ', 1)[1].split()
    row['stable'] = (fields[19], fields[1]) == (end[19], end[1]) and all([
        proc.stat().st_uid==uid, os.readlink(proc/'exe')==exe,
        (proc/'cgroup').read_text()==row['cgroup'],(proc/'cmdline').read_bytes()==raw])
    return row

def match_controller(row, ancestry, expected):
    comparisons = {}
    for role, rule in expected['roles'].items():
        ancestor = ancestry.get(row['pid'])
        checks = dict(argv=row['argv'] == rule['argv'],
            exe=row['exe'] == rule['exe'], exe_sha256=row['exe_sha256'] == sha(rule['exe']),
            cwd=row['cwd'] == expected['cwd'], uid=row['uid'] == expected['uid'],
            ancestor=ancestor is not None and all(row[k] == ancestor[k] for k in
                ['pid','start','ppid','cgroup','uid','exe','exe_sha256']), stable=row['stable'])
        if role == 'watchdog':
            parent = ancestry.get(row['ppid'])
            launch = expected['roles']['launch']
            checks['launch_parent'] = parent is not None and parent['argv'] == launch['argv'] and all([
                parent['stable'], parent['uid'] == expected['uid'],
                parent['cwd'] == expected['cwd'], parent['exe'] == launch['exe'],
                parent['exe_sha256'] == sha(launch['exe']), parent['cgroup'] == row['cgroup']])
        comparisons[role] = checks
    matches = [role for role, checks in comparisons.items() if all(checks.values())]
    return (matches[0] if len(matches) == 1 else None), comparisons

def partial_snapshot(proc, roots, old_cgroup, cause):
    row = dict(pid=int(proc.name), cause=repr(cause), readable={}, errors=[], reasons=[])
    readers = dict(uid=lambda:proc.stat().st_uid,
        stat=lambda:(proc/'stat').read_text(), cgroup=lambda:(proc/'cgroup').read_text(),
        argv=lambda:[os.fsdecode(v) for v in (proc/'cmdline').read_bytes().split(b'\0') if v],
        exe=lambda:os.readlink(proc/'exe'), cwd=lambda:os.readlink(proc/'cwd'),
        exe_sha256=lambda:sha(proc/'exe'))
    for name, read in readers.items():
        try:row['readable'][name]=read()
        except (OSError,ValueError) as error:row['errors'].append(dict(field=name,error=repr(error)))
    data=row['readable']
    if 'cgroup' in data and data['cgroup'].split('::',1)[-1].strip()==old_cgroup:
        row['reasons'].append(dict(field='cgroup',actual=data['cgroup']))
    for field in ['exe','cwd']:
        if field in data and rooted(data[field],roots):row['reasons'].append(dict(field=field,path=data[field]))
    for index,arg in enumerate(data.get('argv',[])):
        if rooted(arg,roots):row['reasons'].append(dict(field='argv',index=index,path=arg))
    data['fds']=[]
    try:
        for fd in (proc/'fd').iterdir():
            try:
                dest=os.readlink(fd);data['fds'].append(dict(fd=fd.name,path=dest))
                if rooted(dest,roots):row['reasons'].append(dict(field='fd',fd=fd.name,path=dest))
            except (OSError,ValueError) as error:row['errors'].append(dict(field='fd',fd=fd.name,error=repr(error)))
    except (OSError,ValueError) as error:row['errors'].append(dict(field='fds',error=repr(error)))
    row['attributed']=True if row['reasons'] else None
    row['admission']='BLOCKED_ATTRIBUTED_UNKNOWN' if row['reasons'] else 'UNATTRIBUTED_UNKNOWN_NOT_CLEARED'
    return row

def scan(expected, roots, old_cgroup, proc_root=Path('/proc'), current_pid=None, parent_pid=None, owned_cgroup=None):
    current_pid = os.getpid() if current_pid is None else current_pid
    parent_pid = os.getppid() if parent_pid is None else parent_pid
    ancestors = []; ancestor_errors = []; cursor = parent_pid
    while cursor > 1 and cursor not in {r['pid'] for r in ancestors}:
        try:
            row = snapshot(proc_root / str(cursor)); ancestors.append(row)
            boundary=expected.get('launch_identity')
            if boundary is not None and row['pid']==boundary['pid']:
                if not all(row[k]==boundary[k] for k in ['pid','start','ppid','cgroup','uid','exe','exe_sha256']):
                    ancestor_errors.append(dict(pid=cursor,error='recorded launcher identity changed'))
                break
            cursor = row['ppid']
        except (OSError, ValueError) as error:
            ancestor_errors.append(dict(pid=cursor, error=repr(error))); break
    ancestry = {r['pid']: r for r in ancestors}
    found = []; unknown = []; controllers = []
    for proc in proc_root.iterdir():
        if not proc.name.isdigit() or int(proc.name) == current_pid: continue
        try:
            if proc.stat().st_uid != expected['uid']: continue
            row = snapshot(proc)
            if row['state'] == 'Z': continue
        except (FileNotFoundError, ProcessLookupError): continue
        except (PermissionError, OSError, ValueError) as error:
            partial=partial_snapshot(proc,roots,old_cgroup,error)
            cg=partial['readable'].get('cgroup','').split('::',1)[-1].strip()
            if owned_cgroup is not None and cg==owned_cgroup:
                partial['reasons'].append(dict(field='owned-cgroup',actual=cg));partial['attributed']=True
            unknown.append(partial); continue
        reasons = []
        if row['cgroup'].split('::', 1)[-1].strip() == old_cgroup:
            reasons.append(dict(field='cgroup', actual=row['cgroup'], expected=old_cgroup))
        for index, arg in enumerate(row['argv']):
            if rooted(arg, roots): reasons.append(dict(field='argv', index=index, path=arg))
        role, comparisons = match_controller(row, ancestry, expected)
        if role:
            controllers.append(dict(**row, role=role, comparisons=comparisons))
            reasons = []  # Same original strict role exemption for argv/cgroup only.
        for field in ['exe', 'cwd']:
            if rooted(row[field], [root for root in roots if root != W]): reasons.append(dict(field=field, path=row[field]))
        for fd in row['fds']:
            if rooted(fd['path'], roots):
                if role == 'poste' and fd['path'] == expected['script'] and not rooted(fd['path'], [T, OLD_W]):
                    if sha(fd['path']) == expected['script_sha256']: continue
                if rooted(fd['path'], [W]):
                    try:
                        info=(proc/'fdinfo'/fd['fd']).read_text()
                        flags=int(next(line.split()[1] for line in info.splitlines() if line.startswith('flags:')),8)
                        if flags & os.O_ACCMODE == os.O_RDONLY:continue
                    except (OSError,ValueError,StopIteration) as error:
                        reasons.append(dict(field='fdinfo',fd=fd['fd'],path=fd['path'],error=repr(error)));continue
                reasons.append(dict(field='fd', **fd))
        if not row['stable']: reasons.append(dict(field='identity', error='changed during capture'))
        if row['fd_errors']: reasons.append(dict(field='fd', error='incomplete snapshot', details=row['fd_errors']))
        if reasons:
            found.append(dict(**row, role=role, reasons=reasons, comparisons=comparisons,
                              parent=ancestry.get(row['ppid'])))
    return dict(found=found, unknown=unknown, controllers=controllers, ancestors=ancestors,
                ancestor_errors=ancestor_errors, expected=expected, roots=[str(r) for r in roots],
                observed=time.monotonic(), historical_pid_56584_attribution='UNRESOLVED')

