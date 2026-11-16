"""Pure contracts for the proposed full pass; no Cargo/process/network entry point."""
from dataclasses import dataclass
from pathlib import Path
import hashlib
import json
import math
import os
import stat

APP = '274803c9241b59523ac4026180d9c20f3aa03091'
BASE = 'bef0b27cb33b9143dabee019fccdf7453da553a3'
LIB = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SDK_KEYS = ('RAG3DB_LIB', 'RAG3DB_SHARED', 'RAG3DB_LIBRARY_DIR',
            'RAG3DB_INCLUDE_DIR', 'LD_LIBRARY_PATH', 'RAG3DB_BUFFER_POOL_SIZE',
            'CARGO_MANIFEST_DIR')
FEATURES = 'rag3db-native,code,daemon'
D6_NAME = 'backend::tests::agent_name_rejects_an_inner_space'
REVIEW_NAME = 'backend::cx_d6_review::interior_whitespace_contract'
CANONICAL_SHA = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(path):
    path = Path(path)
    require(path.is_absolute() and path.resolve() == path, 'path is not absolute/canonical')
    return path


def jobs(poste_fils):
    value = int(poste_fils)
    require(value > 0, 'invalid poste allocation')
    return min(8, value)


@dataclass(frozen=True)
class Window:
    start: float
    end: float

    @classmethod
    def fresh(cls, receipt, recipe, now):
        # receipt.debut is /proc starttime (ticks), not monotonic seconds.
        end = float(receipt['echeance'])
        start = end-float(recipe['delai'])
        require(all(math.isfinite(v) for v in (start, end, now)), 'nonfinite authority')
        require(recipe['delai'] == 7200 and receipt['etat'] == 'actif', 'wrong authority')
        require(start <= now < end and 0 < end-start <= 7200, 'wrong fresh window')
        return cls(start, end)

    def phase_end(self, now, cap):
        require(math.isfinite(now) and 0 < cap <= 7200, 'invalid phase cap')
        end = min(self.end-15, now+cap)
        require(self.start <= now < end, 'phase deadline expired')
        return end


def sdk_profile(store, manifest, ld):
    """Declared before observation. SDK env is separate from Cargo features."""
    root = canonical(store)/LIB
    return dict(RAG3DB_LIB=LIB, RAG3DB_SHARED='1', RAG3DB_LIBRARY_DIR=str(root),
                RAG3DB_INCLUDE_DIR=str(root), LD_LIBRARY_PATH=ld,
                RAG3DB_BUFFER_POOL_SIZE='1073741824', CARGO_MANIFEST_DIR=str(canonical(manifest)))


def cpu_profile(manifest, rust_ld):
    profile = dict.fromkeys(SDK_KEYS)
    profile.update(CARGO_MANIFEST_DIR=str(canonical(manifest)), LD_LIBRARY_PATH=rust_ld)
    return profile


def selected_profile(raw, expected):
    """Take bytes only in memory; return seven selected fields, never raw env."""
    require(set(expected) == set(SDK_KEYS), 'profile keys differ')
    require(raw and raw.endswith(b'\0'), 'empty/truncated Cargo environment')
    fields = {}
    for item in raw.split(b'\0'):
        if b'=' not in item:
            continue
        key, value = item.split(b'=', 1)
        if key in {os.fsencode(k) for k in SDK_KEYS} or key == b'RAG3WEAVER_MOTEUR_ANCIEN':
            require(key not in fields, 'duplicate selected field')
            fields[key] = value
    actual = {k: os.fsdecode(fields[os.fsencode(k)]) if os.fsencode(k) in fields else None
              for k in SDK_KEYS}
    presence = {k: os.fsencode(k) in fields for k in SDK_KEYS}
    diff = {k: dict(expected=v, actual=actual[k]) for k, v in expected.items() if actual[k] != v}
    return dict(rawlen=len(raw), actual=actual, present=presence,
                expected=expected, expected_presence={k: v is not None for k, v in expected.items()},
                diff=diff, old_engine_present=b'RAG3WEAVER_MOTEUR_ANCIEN' in fields)


def qualify_profile(snapshot, declared):
    require(snapshot['expected'] == declared, 'profile was not declared')
    require(snapshot['rawlen'] > 0 and not snapshot['old_engine_present'], 'empty env or old engine')
    require(snapshot['present'] == {k: v is not None for k, v in declared.items()}, 'wrong field presence')
    require(snapshot['actual'] == declared and not snapshot['diff'], 'profile differs')


def task_target(root, repo_name):
    """Matches commande::cible_du_depot, including ASCII-only sanitization."""
    name = ''.join(c if c.isascii() and (c.isalnum() or c in '-_.') else '_' for c in repo_name)
    require(name not in ('', '.', '..'), 'invalid repository name')
    return canonical(root)/('taches-'+name)


def d6_argv(test_name=D6_NAME):
    require(test_name in (D6_NAME, REVIEW_NAME), 'undeclared D6 witness')
    return ['test', '--lib', '--no-default-features', '--features', FEATURES,
            '--locked', '--offline', test_name, '--', '--exact', '--nocapture', '--test-threads=1']


@dataclass(frozen=True)
class CargoAdmission:
    cwd: Path
    target: Path
    argv: tuple
    features: tuple
    profile: dict
    role: str

    def validate(self, cwd, target, argv, profile):
        require(canonical(cwd) == canonical(self.cwd), 'undeclared Cargo cwd')
        require(canonical(target) == canonical(self.target), 'undeclared Cargo target')
        require(tuple(argv) == self.argv, 'undeclared Cargo command')
        qualify_profile(profile, self.profile)


def fixture_sandbox(manifest, q, sdk, task_targets, allocation):
    """Copy configuration, never mutate the source template or disable Landlock."""
    result = json.loads(json.dumps(manifest))
    workspace = result['workspace']
    require(workspace['commands'] == 'approval', 'fixture approval policy differs')
    sandbox = workspace['sandbox']
    reads = sandbox['extra_read']
    for path in (canonical(q), canonical(sdk)):
        if str(path) not in reads:
            reads.append(str(path))
    sandbox['env'].update(CARGO_TARGET_DIR='{repo_target}',
                          CARGO_BUILD_JOBS=str(jobs(allocation)),
                          CARGO_NET_OFFLINE='true')
    require(canonical(task_targets).name == 'task-targets', 'wrong task target root')
    return result


def frozen_inventory(root, exclusions):
    """Only explicitly declared direct children may be omitted; no broad glob."""
    root = canonical(root)
    excluded = {Path(p) for p in exclusions}
    require(all(p.parent == root for p in excluded), 'exclusion outside cache root')
    rows = []
    def fail(error):
        raise error
    for current, dirs, files in os.walk(root, followlinks=False, onerror=fail):
        current = Path(current)
        if current == root:
            dirs[:] = [n for n in dirs if root/n not in excluded]
        for name in sorted(dirs+files):
            path = current/name
            if path in excluded:
                continue
            st = path.lstat()
            row = dict(path=str(path.relative_to(root)), mode=st.st_mode,
                       device=st.st_dev, inode=st.st_ino, uid=st.st_uid)
            if stat.S_ISLNK(st.st_mode):
                row['link'] = os.readlink(path)
            elif stat.S_ISREG(st.st_mode):
                with path.open('rb') as stream:
                    row['sha256'] = hashlib.file_digest(stream, 'sha256').hexdigest()
                row['size'] = st.st_size
            else:
                require(stat.S_ISDIR(st.st_mode), 'special object in frozen cache')
            rows.append(row)
    return sorted(rows, key=lambda row: row['path'])


def task_root_precondition(session_root):
    root = canonical(session_root)
    target = root/'task-targets'
    require(not target.exists() and not target.is_symlink(), 'task-targets already exists')
    st = root.stat()
    require(st.st_uid == os.getuid() and stat.S_ISDIR(st.st_mode), 'cache owner/type differs')
    return dict(path=str(target), parent_identity=[st.st_dev, st.st_ino, st.st_uid])


def publish_result(path, result, now):
    """Verdict returned to the caller is the verdict actually written after open."""
    with Path(path).open('x') as stream:
        if result['verdict'] == 'VERT' and now() >= result['deadline']:
            result = dict(result, verdict='ROUGE', error='deadline after final open')
        json.dump(result, stream, indent=2)
        stream.write('\n')
    return result
