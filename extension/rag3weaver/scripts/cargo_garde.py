#!/usr/bin/env python3
"""Resolve Cargo's actual workspace/target, guard it, then exec unchanged args.

Metadata is not a compilation. The shared shell guard owns all adoption and
invalidation; this adapter never invents provenance or deletes fingerprints.
"""
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    args = sys.argv[1:]
    metadata_args = []
    target = None
    i = 0
    while i < len(args) and args[i] != '--':
        arg = args[i]
        for option in ('--manifest-path', '--target-dir'):
            if arg == option or arg.startswith(option + '='):
                if arg == option:
                    i += 1
                    if i == len(args):
                        raise ValueError('valeur absente pour ' + option)
                    value = args[i]
                else:
                    value = arg.split('=', 1)[1]
                if not value:
                    raise ValueError('valeur vide pour ' + option)
                if option == '--manifest-path':
                    metadata_args.extend([option, value])
                else:
                    target = value
                break
        # Config overrides may alter the workspace or target. Refuse rather
        # than guard a different destination from the one Cargo will use.
        if arg == '--config' or arg.startswith('--config='):
            raise ValueError('--config exige une entrée dédiée qualifiée')
        i += 1
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--format-version', '1', '--no-deps', *metadata_args], text=True))
    workspace = Path(metadata['workspace_root']).resolve(strict=True)
    destination = Path(target if target is not None else metadata['target_directory']).absolute()
    env = dict(os.environ, CARGO_TARGET_DIR=str(destination))
    guard = Path(__file__).with_name('garde_target.sh')
    subprocess.run(['bash', '-c', '. "$1"; garder_le_target "$2" "$3"',
                    'cargo-guard', str(guard), str(destination), str(workspace)], env=env, check=True)
    os.execvpe('cargo', ['cargo', *args], env)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print('✗ entrée Cargo gardée : ' + str(error), file=sys.stderr)
        sys.exit(2)
