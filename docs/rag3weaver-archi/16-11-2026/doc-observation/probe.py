#!/usr/bin/env python3
"""Read-only observation; never a native qualification or target admission."""
from pathlib import Path
import hashlib, json, os, shutil, subprocess, sys, time

R = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/rag3db-cx-archi-doc-supervisor-r1'
C = W / 'extension/rag3weaver'
T = R / 'target-cx-archi-pas-va-au-bout-r2'
OLD_W = Path.home() / 'git_workspaces/rag3db-cx-archi-pas-va-au-bout-r2'
OLD = R / 'cx-archi-pas-va-au-bout-r2-59ac-pass1'
P = R / 'cx-archi-doc-observation-r1'
Q = R / 'cx-archi-doc-observation-controller-r1'
APP = 'ea67054a11d73e06cf44c38d9ce941a754c0190d'
OLD_APP = '713433bd602a6d35266fe5cd528925ba664924c9'
REL = 'docs/rag3weaver-archi/16-11-2026/doc-observation/probe.py'

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
    admit = [sys.executable, str(Q / 'probe.py'), '--admit', pilot]
    poste = [str(C / 'scripts/poste'), 'lourd', *admit]
    outer = ['timeout', '--signal=TERM', '--kill-after=15s', '1320s', *poste]
    return dict(admit=admit, poste=poste, outer=outer,
                launch=[str(control / REL), pilot])

def policy(control, pilot):
    actual = commands(control, pilot)
    return dict(roles={
        'launch': dict(tail=actual['launch'], exe=str(Path(sys.executable).resolve())),
        'poste': dict(tail=actual['poste'], exe=str(Path('/bin/sh').resolve()))},
        outer_command=actual['outer'], cwd='/var/tmp', uid=os.getuid(),
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
    row['stable'] = (fields[19], fields[1]) == (end[19], end[1])
    return row

def scan(expected, roots, old_cgroup, proc_root=Path('/proc'), current_pid=None, parent_pid=None):
    current_pid = os.getpid() if current_pid is None else current_pid
    parent_pid = os.getppid() if parent_pid is None else parent_pid
    ancestors = []; ancestor_errors = []; cursor = parent_pid
    while cursor > 1 and cursor not in {r['pid'] for r in ancestors}:
        try:
            row = snapshot(proc_root / str(cursor)); ancestors.append(row); cursor = row['ppid']
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
            unknown.append(dict(pid=int(proc.name), error=repr(error), attributed=None)); continue
        reasons = []
        if row['cgroup'].split('::', 1)[-1].strip() == old_cgroup:
            reasons.append(dict(field='cgroup', actual=row['cgroup'], expected=old_cgroup))
        for index, arg in enumerate(row['argv']):
            if rooted(arg, roots): reasons.append(dict(field='argv', index=index, path=arg))
        comparisons = {}
        for role, rule in expected['roles'].items():
            a = ancestry.get(row['pid'])
            comparisons[role] = dict(argv=row['argv'][1:] == rule['tail'],
                exe=row['exe'] == rule['exe'], exe_sha256=row['exe_sha256'] == sha(rule['exe']),
                cwd=row['cwd'] == expected['cwd'], uid=row['uid'] == expected['uid'],
                ancestor=a is not None and all(row[k] == a[k] for k in ['start','ppid','cgroup']),
                stable=row['stable'])
        matches = [role for role, checks in comparisons.items() if all(checks.values())]
        role = matches[0] if len(matches) == 1 else None
        if role:
            controllers.append(dict(**row, role=role, comparisons=comparisons))
            reasons = []  # Same original strict role exemption for argv/cgroup only.
        for field in ['exe', 'cwd']:
            if rooted(row[field], roots): reasons.append(dict(field=field, path=row[field]))
        for fd in row['fds']:
            if rooted(fd['path'], roots):
                if role == 'poste' and fd['path'] == expected['script'] and not rooted(fd['path'], [T, OLD_W]):
                    if sha(fd['path']) == expected['script_sha256']: continue
                reasons.append(dict(field='fd', **fd))
        if not row['stable']: reasons.append(dict(field='identity', error='changed during capture'))
        if row['fd_errors']: reasons.append(dict(field='fd', error='incomplete snapshot', details=row['fd_errors']))
        if reasons:
            found.append(dict(**row, role=role, reasons=reasons, comparisons=comparisons,
                              parent=ancestry.get(row['ppid'])))
    return dict(found=found, unknown=unknown, controllers=controllers, ancestors=ancestors,
                ancestor_errors=ancestor_errors, expected=expected, roots=[str(r) for r in roots],
                observed=time.monotonic(), historical_pid_56584_attribution='UNRESOLVED')

def guard(cfg):
    for root, head in [(W, APP), (OLD_W, OLD_APP), (Path(cfg['control']), cfg['pilot'])]:
        assert git(root, 'rev-parse', 'HEAD') == head and not git(root, 'status', '--porcelain')
    assert sha(Q / 'probe.py') == cfg['script_sha256']
    assert sha(Q / 'confiner_validation.py') == cfg['supervisor_sha256']
    st = T.lstat(); assert T.resolve() == T and st.st_uid == os.getuid()
    assert [st.st_dev, st.st_ino] == cfg['target_identity']
    assert (T / '.arbre-du-target').read_text().strip() == str(OLD_W / 'extension/rag3weaver')

def driver():
    cfg = json.loads((Q / 'configuration.json').read_text()); guard(cfg)
    receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    auth = json.loads(receipt.read_text())
    assert json.loads((receipt.parent / 'recette.json').read_text())['delai'] == 60
    subprocess.run(['python3', str(Q / 'confiner_validation.py'), 'verifier'], check=True)
    old_authority = json.loads(Path(cfg['old_receipt']).read_text())
    assert old_authority['etat'] == 'termine'
    expected = policy(Path(cfg['control']), cfg['pilot'])
    result = scan(expected, [T, W, OLD_W, Path(cfg['control'])], old_authority['cgroup'])
    save('snapshot.json', result)  # Snapshot persists BEFORE assertions or result publication.
    assert time.monotonic() < auth['echeance']
    guard(cfg)
    save('observation.json', dict(status='OBSERVATION_ONLY', native_qualification=False,
        found_count=len(result['found']), unknown_count=len(result['unknown']),
        controller_count=len(result['controllers']), deadline=auth['echeance'],
        historical_pid_56584_attribution='UNRESOLVED'))

def prepare(pilot):
    assert os.uname().nodename.lower() == 'luciepc'
    control = Path(__file__).resolve().parents[4]
    assert sys.orig_argv[1:] == [str(control / REL), pilot]
    assert len(pilot) == 40 and git(control, 'rev-parse', 'HEAD') == pilot
    assert not P.exists() and not Q.exists()
    for root, head in [(W, APP), (OLD_W, OLD_APP), (control, pilot)]:
        assert git(root, 'rev-parse', 'HEAD') == head and not git(root, 'status', '--porcelain')
    st = T.lstat()
    old_receipts = list((OLD / 'supervision').glob('validation-*/autorite.json')); assert len(old_receipts) == 1
    P.mkdir(mode=0o700); Q.mkdir(mode=0o700)
    shutil.copy2(__file__, Q / 'probe.py'); shutil.copy2(C / 'scripts/confiner_validation.py', Q / 'confiner_validation.py')
    cfg = dict(mode='OBSERVATION_ONLY', control=str(control), pilot=pilot, app=APP,
        script_sha256=sha(__file__), supervisor_sha256=sha(Q / 'confiner_validation.py'),
        target_identity=[st.st_dev,st.st_ino], old_receipt=str(old_receipts[0]))
    (Q / 'configuration.json').write_text(json.dumps(cfg, indent=2)+'\n'); guard(cfg)
    runtime = Path('/run/user') / str(os.getuid()); bus = runtime / 'bus'
    assert runtime.resolve() == runtime and bus.is_socket() and bus.stat().st_uid == os.getuid()
    env = {k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
    env.update(POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0', XDG_RUNTIME_DIR=str(runtime), DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus))
    with (P / 'supervision.log').open('w') as stream:
        outcome = subprocess.run(commands(control, pilot)['outer'], cwd='/var/tmp', env=env, stdout=stream, stderr=subprocess.STDOUT)
    scopes = list((P / 'supervision').glob('validation-*/verdict.json'))
    result = dict(status='OBSERVATION_ONLY', native_qualification=False, outer_rc=outcome.returncode)
    if len(scopes) == 1: result['scope'] = json.loads(scopes[0].read_text())
    save('result.json', result); print(json.dumps(result)); return outcome.returncode

if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == '--admit':
        os.execv(sys.executable, [sys.executable, str(Q / 'confiner_validation.py'), 'lancer', '--delai', '60', '--preuves', str(P / 'supervision'), '--', 'python3', str(Q / 'probe.py'), '--driver', sys.argv[-1]])
    elif mode == '--driver': driver()
    else: sys.exit(prepare(mode))
