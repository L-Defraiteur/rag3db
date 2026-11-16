"""Prepare/launch/finalize a new HARNESS ONLY allocation. Never build or replay F8."""
import argparse
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from strict_target_guard import digest, snapshot, scan, require_source, require_hashes, finish
from launch_full import publish_result

R = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/cx-tty-english-aliases'
T = R / 'target-cx-tty-english-aliases-2'
Q = R / 'cx-tty-harness-only-controller-v3'
P = R / 'cx-tty-harness-only-v3'
S = R / 'cx-tty-harness-only-common-v3'
PRODUCT = '6d886375ea4627c88d28aef918b948a23f628ab1'
BUDGET = 1200
PREVIOUS_SCOPE = R / 'cx-tty-aliases-continuation-common-v1/validation-flsb10fe'


def production_identity():
    changed = subprocess.check_output(['git', '-C', str(W), 'diff', '--name-only', PRODUCT, '--', 'extension/rag3weaver'], text=True).splitlines()
    allowed = {'extension/rag3weaver/tty/mesures/' + name for name in
               ['mesure.py', 'harness_observation.py', 'test_harness_observation.py']}
    assert set(changed) <= allowed
    return changed


def outer_watchdog_command(worker, source, allocation, started, budget=BUDGET, grace=5):
    remaining = started + budget - time.monotonic()
    margin = min(1.0, budget / 10)
    soft = remaining - grace - margin
    assert soft > 0, 'allocation expired before watchdog entry'
    return ['/usr/bin/timeout', '--signal=TERM', f'--kill-after={grace}s', f'{soft:.6f}s',
            sys.executable, '-B', str(worker), '--source', source, '--allocation', allocation,
            '--worker', '--started', f'{started:.9f}']


def allocation_worker(args):
    if not args.prepare_only:
        assert args.started is not None and args.allocation
        parent = Path('/proc') / str(os.getppid())
        assert (parent / 'exe').resolve() == Path('/usr/bin/timeout').resolve(), 'worker must be inside the outer OS watchdog'
        assert args.started <= time.monotonic() < args.started + BUDGET
    require_source(W, args.source)
    changed = production_identity()
    assert not any(path.exists() or path.is_symlink() for path in [P, S])
    if args.prepare_only:
        Q.mkdir(mode=0o700)
        old = PREVIOUS_SCOPE
        authority = json.loads((old / 'autorite.json').read_text())
        verdict = json.loads((old / 'verdict.json').read_text())
        assert authority['etat'] == 'termine' and verdict['restes'] == [] and verdict['nettoye']
        assert T.resolve() == T and (T / '.arbre-du-target').read_text().strip() == str(W / 'extension/rag3weaver/tty')
        initial = (R / 'cx-tty-english-aliases-go2-v1/target-initial.txt').read_text().split()
        st = T.stat()
        assert [str(st.st_uid), str(st.st_dev), str(st.st_ino), oct(st.st_mode & 0o777)[2:]] == initial
        quiet = scan('/proc', [W, T])
        (Q / 'quiet-prepare.json').write_text(json.dumps(quiet, indent=2))
        assert quiet['found'] == []
        ledger = snapshot(T)
        assert snapshot(T) == ledger
        (Q / 'target-ledger.json').write_text(json.dumps(ledger, indent=2))
        build = R / 'cx-tty-aliases-continuation-v1'
        assert (build / 'build.rc').read_text().strip() == '0'
        elf = T / 'debug/rag3weaver-tty'
        elf_sha = (build / 'tty-elf-sha.txt').read_text().split()[0]
        assert digest(elf) == elf_sha
        shutil.copy2(elf, Q / 'qualified-tty-elf')
        assert digest(Q / 'qualified-tty-elf') == elf_sha
        f8 = R / 'cx-tty-english-aliases-full-v5'
        logs = sorted((f8 / 'features').glob('*.log'))
        assert len(logs) == 8 and (f8 / 'f8.log').read_text().splitlines()[-1] == 'CHECK_FEATURES VERT 8/8'
        assert (f8 / 'source.txt').read_text().strip() == PRODUCT and (f8 / 'f8.rc').read_text().strip() == '0'
        carried = {str(path): digest(path) for path in [f8 / 'f8.log', f8 / 'f8.rc', f8 / 'source.txt', *logs, build / 'build.rc', build / 'tty-elf-sha.txt']}
        sdk = Path.home() / '.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
        sdk_hashes = {str(sdk / 'librag3db.so'): 'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',
                      str(sdk / 'libvector.rag3db_extension'): '1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
        require_hashes(sdk_hashes)
        folder = W / 'docs/tty/16-11-2026'
        sources = {name: folder / name for name in ['harness_only.py', 'harness_only_admit.py', 'harness_only_driver.py',
                   'strict_target_guard.py', 'launch_full.py', 'run_private_harness.py']}
        sources.update({name: W / 'extension/rag3weaver/tty/mesures' / name for name in
                        ['mesure.py', 'faux_hote.py', 'harness_observation.py', 'test_harness_observation.py']})
        sources['poste'] = W / 'extension/rag3weaver/scripts/poste'
        sources['confiner_validation.py'] = W / 'extension/rag3weaver/scripts/confiner_validation.py'
        hashes = {}
        for name, path in sources.items():
            shutil.copy2(path, Q / name)
            hashes[str(Q / name)] = digest(path)
            assert digest(Q / name) == hashes[str(Q / name)]
        shutil.copy2(old / 'autorite.json', Q / 'previous-authority.json')
        shutil.copy2(old / 'verdict.json', Q / 'previous-verdict.json')
        tmux = Path(shutil.which('tmux'))
        config = dict(source=args.source, product=PRODUCT, changed_harness_only=changed,
                      host=os.uname().nodename, worktree=str(W), target=str(T), proofs=str(P), scope=str(S),
                      budget=BUDGET, outer_watchdog={'path':'/usr/bin/timeout','sha256':digest('/usr/bin/timeout'),'grace':5,'entry_margin':1}, controllers=hashes, carried=carried, sdk=sdk_hashes,
                      elf=str(elf), elf_sha=elf_sha, ledger_sha=digest(Q / 'target-ledger.json'),
                      tmux={str(tmux):digest(tmux)}, old_expired_end=852082.791694931,
                      purpose='HARNESS ONLY; F8 and build carried, no lost run requalified')
        (Q / 'prepared.json').write_text(json.dumps(config, indent=2))
        print('PREPARED_ONLY', Q)
        return
    assert args.allocation, 'a fresh exact C7 allocation is required'
    config = json.loads((Q / 'prepared.json').read_text())
    assert config['source'] == args.source and config['host'] == os.uname().nodename
    require_hashes(config['controllers'])
    require_hashes(config['carried'])
    require_hashes(config['sdk'])
    assert digest(Q / 'target-ledger.json') == config['ledger_sha']
    require_hashes({config['outer_watchdog']['path']:config['outer_watchdog']['sha256']})
    start = args.started
    end = start + BUDGET  # new run only, not a continuation of the expired original run
    with (Q / 'allocation.json').open('x') as output:
        json.dump(dict(reference=args.allocation, start=start, end=end), output)
    env = dict(os.environ, POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0', TTY_CONTROLLER_DIR=str(Q))
    os.chdir(Q)
    record = dict(verdict='ROUGE', source=args.source, start=start, end=end)
    rc = 2
    try:
        command = [str(Q / 'poste'), 'lourd', sys.executable, str(Q / 'harness_only_admit.py')]
        (Q / 'outer-command.json').write_text(json.dumps(command))
        rc = subprocess.run(command, env=env).returncode
        receipt = json.loads((P / 'authority-path.json').read_text())
        authority = json.loads(Path(receipt['path']).read_text())
        scope = json.loads((Path(receipt['path']).parent / 'verdict.json').read_text())
        assert rc == 0 and not Path(scope['prive']).exists()
        assert snapshot(T) == json.loads((Q / 'target-ledger.json').read_text())
        require_hashes(config['controllers'])
        require_hashes(config['carried'])
        require_hashes(config['sdk'])
        require_hashes({config['elf']:config['elf_sha']})
        quiet = scan('/proc', [W,T])
        (P / 'quiet-after-common.json').write_text(json.dumps(quiet, indent=2))
        assert quiet['found'] == []
        require_source(W, args.source)
        production_identity()
        driver = json.loads((P / 'driver-result.json').read_text())
        record['verdict'] = finish(authority, scope, driver['verdict'], time.monotonic(), end, True)
    except Exception as error:
        record['error'] = repr(error)
    record['rc'] = rc
    record = publish_result(Q / 'composed-verdict.json', record, end)
    print(record['verdict'], Q / 'composed-verdict.json')
    if record['verdict'] != 'VERT':
        raise SystemExit(1)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', required=True)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--allocation')
    parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--started', type=float, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.prepare_only or args.worker:
        return allocation_worker(args)
    assert args.allocation, 'a fresh exact C7 allocation is required'
    started = time.monotonic()
    command = outer_watchdog_command(Q / 'harness_only.py', args.source, args.allocation, started)
    # No hashing, finalization or verdict write follows this exec in the ultimate entry.
    os.execv(command[0], command)


if __name__ == '__main__':
    main()
