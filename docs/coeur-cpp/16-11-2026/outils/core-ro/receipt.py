"""Read-only pins and verification of existing artifacts, no inferior execution."""
import json
from pathlib import Path


def admitted_cpu(text, threads):
    quota, period = text.split()
    if quota == 'max' or int(quota) <= 0 or int(period) <= 0:
        raise ValueError('invalid effective quota')
    jobs = min(8, int(threads), int(quota) // int(period))
    if jobs < 1:
        raise ValueError('no effective whole core')
    return {'jobs': jobs, 'quota_percent': int(quota) * 100 / int(period)}


def verify_artifact_manifests(out, checksum, complete=True):
    recipe = json.loads((out / 'recipe.json').read_text())
    # Also recheck inputs after a failed analysis; no changed input can be silent.
    for item in recipe['inputs']:
        assert checksum(Path(item['path'])) == item['sha256'], item['name']
    receipt = out / 'raw/parsing/receipt.json'
    if complete:
        data = json.loads(receipt.read_text())
        assert data['inferior_started'] is False and data['core_sha256'] == recipe['raw_core_sha256']
        assert data['gdb_code'] == 0
    return ['analysis'] if receipt.exists() else []


def gdb_command(gdb, elf, core, sysroot):
    # Deliberately no live process, inferior command, Python extension or library autoload.
    initial = ['set auto-load off', 'set debuginfod enabled off', 'set auto-solib-add off',
               'set sysroot ' + str(sysroot), 'set solib-search-path ' + str(sysroot),
               'set pagination off', 'set confirm off', 'set print frame-arguments none',
               'set print entry-values no']
    command = [str(gdb), '--nx', '--nh', '--batch', '--quiet']
    for setting in initial:
        command += ['-iex', setting]
    command += ['--se=' + str(elf), '--core=' + str(core)]
    for query in ['info files', 'info sharedlibrary', 'thread apply all bt 12',
                  'info registers rip rsp', 'quit']:
        command += ['-ex', query]
    return command
