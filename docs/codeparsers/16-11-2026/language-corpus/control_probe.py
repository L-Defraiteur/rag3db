#!/usr/bin/env python3
"""Contrôler des workers de production, un fichier en vol, sans relancer un cas."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import selectors
import signal
import subprocess
import time


COMMON = Path(__file__).resolve().parents[4] / "extension/rag3weaver/scripts/confiner_validation.py"

def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def worker_limits():
    # Limite de l'espace d'adressage du worker, distincte du RSS et du cgroup global.
    resource.setrlimit(resource.RLIMIT_AS, (2 * 1024**3, 2 * 1024**3))


def stop(worker):
    if worker is None:
        return
    try:
        os.killpg(worker.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    worker.wait()
    worker.stdin.close()
    worker.stdout.close()


def response(worker, deadline):
    data = bytearray()
    with selectors.DefaultSelector() as selector:
        selector.register(worker.stdout, selectors.EVENT_READ)
        while time.monotonic() < deadline:
            ready = selector.select(max(0, deadline - time.monotonic()))
            if not ready:
                return None, 'timeout'
            block = os.read(worker.stdout.fileno(), 65536)
            if not block:
                return None, 'worker_exit'
            data.extend(block)
            if len(data) > 8 * 1024**2:
                raise ValueError('worker response exceeds 8MiB protocol bound')
            if b'\n' in data:
                line, extra = data.split(b'\n', 1)
                if extra:
                    raise ValueError('unsolicited worker output')
                return json.loads(line), None
    return None, 'timeout'


def verify_roots(roots):
    for identity, root in roots['repositories'].items():
        path = Path(root)
        if not path.is_absolute() or path.resolve(strict=True) != path:
            raise ValueError('noncanonical root')
        if (path / '.git').exists():
            head = subprocess.check_output(['git', '--no-optional-locks', '-C', root,
                'rev-parse', 'HEAD'], text=True).strip()
            if identity.rsplit('@', 1)[-1] != head:
                raise ValueError('checkout commit differs from identity')
        else:
            freeze_path = path.parent / 'freeze.json'
            if freeze_path.is_symlink():
                raise ValueError('linked freeze')
            freeze = json.loads(freeze_path.read_bytes())
            if (freeze['repository_id'] != identity or freeze['root'] != root or
                    freeze['repository_commit'] != identity.rsplit('@', 1)[-1] or
                    freeze.get('executed_repository_code') is not False):
                raise ValueError('archive freeze identity mismatch')
            for name, key in [('source.tar.gz', 'archive_sha256'), ('inventory.jsonl', 'inventory_sha256')]:
                artifact = path.parent / name
                if artifact.is_symlink() or digest(artifact) != freeze[key]:
                    raise ValueError('archive/inventory digest mismatch')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--roots', type=Path, required=True)
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--file-seconds', type=float, default=10)
    args = parser.parse_args()
    if not 0 < args.file_seconds <= 60:
        raise ValueError('file deadline must be within (0,60] seconds')
    # La campagne doit hériter d'une véritable portée du superviseur canonique.
    if not COMMON.is_file():
        raise ValueError('canonical supervisor path missing')
    subprocess.run(['python3', str(COMMON), 'verifier'], check=True)
    binary = args.binary.resolve(strict=True)
    roots_path = args.roots.resolve(strict=True)
    roots = json.loads(roots_path.read_bytes())
    if roots['schema_version'] != 1:
        raise ValueError('roots schema')
    verify_roots(roots)
    args.proofs.mkdir(mode=0o700, parents=True, exist_ok=False)
    provenance = {'binary_sha256': digest(binary), 'manifest_sha256': digest(args.manifest),
        'roots_sha256': digest(roots_path), 'controller_sha256': digest(__file__),
        'file_seconds': args.file_seconds, 'worker_address_space_bytes': 2 * 1024**3,
        'cache_policy': 'one serial worker, restarted after panic/timeout/exit; no file retry',
        'coverage_status': 'unjudged'}
    (args.proofs / 'provenance.json').write_text(json.dumps(provenance, indent=2))
    worker = None
    completed = 0
    statuses = {}
    try:
        with args.manifest.open('rb') as manifest, (args.proofs / 'results.jsonl').open('x') as output, \
                (args.proofs / 'workers.stderr').open('xb') as errors:
            for sequence, raw in enumerate(manifest, 1):
                entry = json.loads(raw)
                if worker is None or worker.poll() is not None:
                    stop(worker)
                    worker = subprocess.Popen([str(binary), '--stream', '--roots', str(roots_path)],
                        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors,
                        start_new_session=True, preexec_fn=worker_limits)
                started = time.monotonic()
                try:
                    worker.stdin.write(raw.rstrip(b'\r\n') + b'\n')
                    worker.stdin.flush()
                    row, failure = response(worker, started + args.file_seconds)
                except BrokenPipeError:
                    row, failure = None, 'worker_exit'
                if failure:
                    row = {'schema_version': 1, 'input': entry, 'coverage_status': 'unjudged',
                        'measurement': {'execution_status': failure, 'diagnostic': 'no retry',
                            'elapsed_ns': None}, 'process_peak_rss_bytes': None}
                    stop(worker)
                    row['worker_returncode'] = worker.returncode
                    worker = None
                elif row.get('input') != entry:
                    raise ValueError('worker response identity mismatch')
                row['sequence'] = sequence
                row['controller_elapsed_seconds'] = time.monotonic() - started
                status = row['measurement']['execution_status']
                output.write(json.dumps(row, ensure_ascii=False) + '\n')
                output.flush()
                completed += 1
                statuses[status] = statuses.get(status, 0) + 1
                if status == 'validation_error':
                    raise ValueError('harness/provenance validation failed; evidence retained')
                if status == 'panic':
                    stop(worker)
                    worker = None
            if completed == 0:
                raise ValueError('empty manifest')
    finally:
        stop(worker)
        (args.proofs / 'summary.json').write_text(json.dumps({'entries_recorded': completed,
            'execution_status_counts': statuses, 'coverage_status': 'unjudged'}, indent=2))
    if digest(args.manifest) != provenance['manifest_sha256'] or digest(roots_path) != provenance['roots_sha256']:
        raise ValueError('input changed during campaign')


if __name__ == '__main__':
    main()
