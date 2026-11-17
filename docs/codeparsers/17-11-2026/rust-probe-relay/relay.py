#!/usr/bin/env python3
"""Relayer les publications de phase sans intervention de la session ; allocation explicite."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import signal
import socket
import subprocess
import sys
import tempfile
import time


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def atomic_json(path, value, before_link=lambda: None):
    path = Path(path)
    fd, pending = tempfile.mkstemp(prefix='.' + path.name + '-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        before_link()
        os.link(pending, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        os.unlink(pending)


class Relay:
    """Le même moteur pour SSH réel et témoins : pas de branche fake/skip."""
    def __init__(self, transport, root, clock=time.monotonic, sleep=time.sleep,
                 queue_budget=300, transfer_cap=30, control_cap=5):
        self.transport, self.root, self.clock, self.sleep = transport, Path(root), clock, sleep
        self.queue_budget, self.transfer_cap, self.control_cap = queue_budget, transfer_cap, control_cap
        self.deadline = None
        self.phase_deadline = None
        self.queue_end = clock() + queue_budget
        self.stage = None

    def cap(self, cap):
        end = self.deadline if self.deadline is not None else self.queue_end
        remaining = end - self.clock()
        if remaining <= 0:
            raise TimeoutError('relay deadline')
        return min(cap, remaining)

    def refresh(self, expected=None):
        before = self.clock()
        state = self.transport.snapshot(self.cap(self.control_cap))
        if state['state'] == 'waiting':
            assert expected is None, 'active phase disappeared'
            self.cap(self.control_cap)
            return state
        assert state['state'] == 'active', 'phase is no longer active'
        # Durée PC ancrée AVANT le RPC : sa latence ne rallonge jamais le délai.
        end = before + state['remaining']
        self.deadline = min(self.deadline, end) if self.deadline is not None else end
        phase_end = before + state.get('phase_remaining', state['remaining'])
        self.phase_deadline = min(self.phase_deadline, phase_end) if self.phase_deadline is not None else phase_end
        self.cap(self.control_cap)
        if expected is not None:
            assert state['stage'] == expected, 'publication stage changed'
        return state

    def publish(self, kind, state):
        files = ['ledger', 'elf'] if kind == 'build' else ['ledger', 'all', 'ignored']
        folder = self.root / kind
        folder.mkdir(mode=0o700)
        for name in files:
            current = self.refresh(kind)
            assert current['files'] == state['files'] and current['ledger'] == state['ledger']
            pending = folder / (name + '.pending')
            actual = self.transport.fetch(kind, name, pending, self.cap(self.transfer_cap))
            with pending.open('rb') as stream:
                os.fsync(stream.fileno())
            assert actual == digest(pending) == current['files'][name], 'copied file hash differs'
            if name == 'ledger':
                assert json.loads(pending.read_text()) == current['ledger']
            elif name == 'elf':
                assert actual == current['ledger']['elf_sha256']
            fresh = self.refresh(kind)
            assert fresh['files'] == current['files'], 'payload changed before local publication'
            self.cap(self.control_cap)
            if name == 'elf':
                pending.chmod(0o500)
            os.link(pending, folder / name)
            directory = os.open(folder, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
            pending.unlink()
        if kind == 'runtime':
            assert digest(folder / 'all') == state['ledger']['inventory_sha256']
            assert digest(folder / 'ignored') == state['ledger']['ignored_inventory_sha256']
            assert digest(self.root / 'build/elf') == state['ledger']['elf_sha256']
        fresh = self.refresh(kind)
        assert fresh['files'] == state['files'] and fresh['ledger'] == state['ledger']
        ack = {'kind': kind, 'ledger_sha256': state['files']['ledger'],
               'elf_sha256': state['ledger']['elf_sha256'], 'copied_and_rehashed': True}
        self.transport.publish_ack(kind, ack, self.cap(self.control_cap))
        atomic_json(folder / 'ack-accepted.json', ack, lambda: self.cap(self.control_cap))

    def run(self):
        completed = set()
        try:
            self.transport.ready()
            self.transport.start_phase()
            while len(completed) != 2:
                if not self.transport.running():
                    raise RuntimeError('phase exited before both publications')
                state = self.refresh()
                stage = state.get('stage')
                if state['state'] == 'active' and stage in ['build', 'runtime']:
                    assert stage not in completed
                    assert stage == 'build' or completed == {'build'}, 'runtime before build'
                    self.publish(stage, state)
                    completed.add(stage)
                else:
                    self.sleep(min(.1, self.cap(.1)))
            remaining = self.phase_deadline - self.clock()
            if remaining <= 0:
                raise TimeoutError('phase completion deadline')
            rc = self.transport.finish(remaining)
            if rc:
                raise RuntimeError('phase returned ' + str(rc))
            return rc
        except BaseException as error:
            try:
                atomic_json(self.root / 'first-red.json', {'reason': repr(error), 'observed': self.clock()})
            except FileExistsError:
                pass
            # Le refus distant est borné par le temps restant déjà reçu, pas un nouveau budget.
            try:
                self.transport.abort(repr(error), self.cap(self.control_cap))
            except BaseException:
                pass
            raise
        finally:
            self.transport.stop()


class SSHTransport:
    def __init__(self, pair, pin, phase, allocation, host, root):
        self.pair, self.pin, self.phase, self.allocation = pair, pin, phase, allocation
        self.root, self.host, self.child, self.counter = Path(root), host, None, 0
        self.active_call = None
        self.handles = []
        self.ready_record = None
        self.control = Path(pair['control'])
        spec = importlib.util.spec_from_file_location('relay_physical_guard', self.control / 'physical_guard.py')
        self.guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.guard)

    def command(self, action, *extra, cap=None):
        remote = [sys.executable, '-B', str(self.control / 'gate.py'), action,
                  '--proofs', self.pair['proofs'], '--phase', self.phase, '--pin', self.pin, *extra]
        if cap is not None:
            assert 0 < cap <= 30
            remote = [self.pair['timeout'], '--signal=TERM', '--kill-after=1s', str(cap) + 's', *remote]
        # Un seul argv distant quoté ; pas d'interpolation de shell ni de parent argv W/T.
        return ['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', '-o', 'ControlMaster=no',
                '-o', 'ControlPath=none', self.host, 'exec ' + shlex.join(remote)]

    def paths(self, label, argv):
        self.counter += 1
        stem = self.root / (str(self.counter).zfill(3) + '-' + label)
        atomic_json(stem.with_suffix('.command.json'), {'argv': argv, 'started': time.monotonic()})
        return stem

    def call(self, action, timeout, extra=(), payload=None, destination=None):
        self.check_coordinator()
        argv = self.command(action, *extra, cap=max(.001, timeout - .1))
        stem = self.paths(action, argv)
        output = Path(destination) if destination else stem.with_suffix('.stdout')
        with output.open('xb') as out, stem.with_suffix('.stderr').open('xb') as err:
            child = subprocess.Popen(argv, stdin=subprocess.PIPE if payload else subprocess.DEVNULL,
                stdout=out, stderr=err, start_new_session=True)
            self.active_call = child
            try:
                child.communicate(json.dumps(payload).encode() if payload else None, timeout=timeout)
                atomic_json(stem.with_suffix('.result.json'), {'rc': child.returncode, 'finished': time.monotonic()})
                if child.returncode:
                    raise RuntimeError('SSH gate ' + action + ' returned ' + str(child.returncode))
            finally:
                self.reap(child)
                if not stem.with_suffix('.result.json').exists():
                    atomic_json(stem.with_suffix('.result.json'), {'rc': child.returncode, 'finished': time.monotonic(),
                        'interrupted': True})
                self.active_call = None
        return None if destination else json.loads(output.read_text())

    @staticmethod
    def reap(child):
        # L'objet Popen non récolté identifie cet enfant ; aucun PID découvert par nom.
        if child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=.5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=.5)

    def ready(self):
        identity = self.guard.process_record(os.getpid())
        self.ready_record = {'host': socket.gethostname(), 'identity': identity,
            'script_sha256': digest(__file__), 'prepared_sha256': self.pin, 'phase': self.phase}
        atomic_json(self.root / 'ready.json', self.ready_record)

    def check_coordinator(self):
        assert digest(__file__) == self.pair['controllers']['relay.py']
        assert digest(Path(self.pair['proofs']) / 'prepared.json') == self.pin
        actual = self.guard.process_record(os.getpid())
        expected = self.ready_record['identity']
        assert actual['state'] != 'Z' and all(actual[k] == expected[k] for k in
            ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])

    def start_phase(self):
        self.check_coordinator()
        argv = self.command('launch', '--allocation', self.allocation)
        stem = self.paths('launch', argv)
        out, err = stem.with_suffix('.stdout').open('xb'), stem.with_suffix('.stderr').open('xb')
        self.handles.extend([out, err])
        self.child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=out, stderr=err, start_new_session=True)
        self.child.stdin.write(json.dumps(self.ready_record).encode())
        self.child.stdin.close()

    def snapshot(self, timeout):
        return self.call('snapshot', timeout)

    def fetch(self, kind, name, destination, timeout):
        self.call('export', timeout, ('--kind', kind, '--file', name), destination=destination)
        return digest(destination)

    def publish_ack(self, kind, ack, timeout):
        names = ['ledger', 'elf'] if kind == 'build' else ['ledger', 'all', 'ignored']
        ack['peer_files'] = {name: str(self.root / kind / name) for name in names}
        ack.update(host=socket.gethostname(), phase=self.phase, prepared_sha256=self.pin,
            coordinator=self.ready_record)
        result = self.call('ack', timeout, ('--kind', kind), ack)
        assert result['accepted'] is True

    def running(self):
        return self.child is not None and self.child.poll() is None

    def finish(self, timeout):
        rc = self.child.wait(timeout=timeout)
        atomic_json(self.root / 'phase-ssh-result.json', {'rc': rc, 'finished': time.monotonic()})
        return rc

    def abort(self, reason, timeout):
        if self.child is not None:
            self.call('abort', timeout, payload={'reason': reason})

    def stop(self):
        if self.active_call is not None:
            self.reap(self.active_call)
        if self.child is not None:
            self.reap(self.child)
        for handle in self.handles:
            handle.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--phase', choices=['baseline', 'candidate'], required=True)
    parser.add_argument('--allocation', required=True)
    parser.add_argument('--host', default='luciepc')
    args = parser.parse_args()
    path = args.proofs / 'prepared.json'
    pair, pin = json.loads(path.read_text()), digest(path)
    assert args.host == 'luciepc'
    assert socket.gethostname() == pair['peer_host']
    assert Path(__file__).resolve() == Path(pair['control']) / 'relay.py'
    assert digest(__file__) == pair['controllers']['relay.py']
    assert pair['relay']['queue_budget'] == 300 and pair['relay']['transfer_cap'] == 30 and pair['relay']['control_cap'] == 5
    os.chdir(args.proofs)
    root = args.proofs / args.phase / 'relay-local'
    root.mkdir(mode=0o700)
    transport = SSHTransport(pair, pin, args.phase, args.allocation, args.host, root)
    def stop(sig, frame):
        raise TimeoutError('relay received signal ' + str(sig))
    for sig in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM]:
        signal.signal(sig, stop)
    signal.setitimer(signal.ITIMER_REAL, 2100)
    return Relay(transport, root, **pair['relay']).run()


if __name__ == '__main__':
    sys.exit(main())
