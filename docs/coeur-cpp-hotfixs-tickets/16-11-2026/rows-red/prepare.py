#!/usr/bin/env python3
"""Explicit file/Git preparation only. Does not acquire a poste or run native code."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from receipt import ENGINE, SOURCE

ROOT = Path.home()/'.cache/rag3weaver-build'
OUT = ROOT/'cx-hotfixs-rows-outside-graph-red-v2'
SCRATCH = ROOT/'cx-hotfixs-rows-red-v2'
BASE = 'df2a5789888d16247f0dad42dcdaa3c4ceea2094'
SCRIPTS = ['entry.py', 'controller.py', 'receipt.py', 'launch.sh']


def git(root, *args):
    env = dict(os.environ, GIT_OPTIONAL_LOCKS='0', GIT_CONFIG_COUNT='2',
               GIT_CONFIG_KEY_0='gc.auto', GIT_CONFIG_VALUE_0='0',
               GIT_CONFIG_KEY_1='maintenance.auto', GIT_CONFIG_VALUE_1='false')
    return subprocess.check_output(['git', '-C', str(root), *args], text=True, env=env,
                                   timeout=60).strip()


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main(args):
    assert os.uname().nodename.lower() == 'luciepc'
    assert re.fullmatch('[0-9a-f]{40}', args.pilot)
    pilot = Path(__file__).resolve().parents[4]
    source = Path(args.source_root)
    assert source.resolve() == source and source.is_dir()
    assert git(pilot, 'rev-parse', 'HEAD') == args.pilot
    assert not git(pilot, 'status', '--porcelain')
    assert git(source, 'rev-parse', 'HEAD') == SOURCE
    assert not git(source, 'status', '--porcelain')
    engine_paths = ['src', 'extension', 'third_party', 'cmake', 'CMakeLists.txt',
                    ':(exclude)extension/rag3weaver', ':(exclude)extension/lucivy']
    assert git(source, 'log', '-1', '--format=%H', '--', *engine_paths) == ENGINE
    assert not any(path.exists() or path.is_symlink() for path in [OUT, SCRATCH])
    OUT.mkdir(mode=0o700)
    SCRATCH.mkdir(mode=0o700)
    bundle = OUT/'source.bundle'
    # Archive complète autonome : aucun alternates, accès à un autre worktree
    # ou téléchargement ne sera nécessaire dans la portée native.
    git(source, 'bundle', 'create', str(bundle), 'HEAD')
    assert 0 < bundle.stat().st_size <= 4*1024**3
    git(source, 'bundle', 'verify', str(bundle))
    assert git(source, 'bundle', 'list-heads', str(bundle)) == SOURCE+' HEAD'
    scripts = {}
    for name in SCRIPTS:
        shutil.copy2(Path(__file__).parent/name, OUT/name)
        scripts[name] = sha(OUT/name)
    recipe = {'status': 'PREPARED ONLY; EXACT RO AND C7 ADMISSION REQUIRED; NOT EXECUTED',
              'source_commit': SOURCE, 'engine_commit': ENGINE, 'base': BASE,
              'pilot_commit': args.pilot, 'source_root': str(source),
              'unit': 'cx-hotfixs-rows-red-02', 'scripts': scripts,
              'bundle_sha256': sha(bundle), 'bundle_bytes': bundle.stat().st_size,
              'native_files': {name: sha(source/name) for name in
                               ['test/transaction/vector_index_diagnostics_test.cpp',
                                'test/transaction/CMakeLists.txt',
                                'src/function/table/show_indexes.cpp']},
              'global_cap_s': 2400, 'watchdog_term_s': 2390, 'watchdog_kill_s': 10,
              'controller_end_s': 2310, 'native_end_s': 2330,
              'finalization_end_s': 2380,
              'phase_caps_s': {'staging': 60, 'configure': 120, 'build': 1800, 'red': 30},
              'memory_bytes': 34359738368, 'swap_bytes': 0, 'scratch_bytes': 34359738368,
              'jobs': 'min(8, POSTE_FILS, floor(effective admitted quota))',
              'final_summary': 'ROUGE', 'product_green': False, 'retry': False}
    with (OUT/'recipe.json').open('x') as stream:
        json.dump(recipe, stream, indent=2)
    assert git(source, 'rev-parse', 'HEAD') == SOURCE
    assert not git(source, 'status', '--porcelain')
    print(json.dumps({'status': 'PREPARED_NOT_RUN', 'out': str(OUT),
                      'recipe_sha256': sha(OUT/'recipe.json'), 'native_run': False,
                      'poste_acquired': False, 'systemd_invoked': False}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--pilot', required=True)
    parser.add_argument('--source-root', required=True)
    main(parser.parse_args())
