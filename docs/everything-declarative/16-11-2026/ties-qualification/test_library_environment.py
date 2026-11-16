"""Real lib resolver under a private XDG cache. No Cargo, DB or library load."""
import argparse
import ast
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--worktree', required=True)
parser.add_argument('--store', required=True)
parser.add_argument('--lib', required=True)
args = parser.parse_args()
source = Path(__file__).with_name('driver.py')
function = next(n for n in ast.parse(source.read_text()).body
                if isinstance(n, ast.FunctionDef) and n.name == 'library_environment')
scope = dict(CONFIG=dict(lib=args.lib, library_store=args.store))
exec(compile(ast.Module(body=[function], type_ignores=[]), 'actual-library-environment', 'exec'), scope)
selected = scope['library_environment']()
expected = str((Path(args.store) / args.lib).resolve())
with tempfile.TemporaryDirectory(prefix='cx-ed-library-resolver-') as private:
    env = {k: os.environ[k] for k in ['HOME', 'USER', 'PATH', 'LANG'] if k in os.environ}
    env.update(XDG_CACHE_HOME=private, RAG3DB_LIB=args.lib)
    command = ['bash', '-c', '. "$1"; exporter_la_lib || exit $?; '
        'python3 -c \'import json,os; print(json.dumps({k:os.environ.get(k) for k in '
        '["RAG3DB_LIBRARY_DIR","RAG3DB_INCLUDE_DIR","RAG3DB_SHARED","LD_LIBRARY_PATH"]}))\'',
        'actual-resolver', str(Path(args.worktree) / 'extension/rag3weaver/scripts/lib_du_magasin.sh')]
    red = subprocess.run(command, env=env, text=True, capture_output=True, timeout=30)
    assert red.returncode == 2 and str(Path(private) / 'rag3db-libs') in red.stderr
    env.update(selected)
    green = subprocess.run(command, env=env, text=True, capture_output=True, timeout=30)
    assert green.returncode == 0, green.stderr
    profile = json.loads(green.stdout)
    assert profile == dict(RAG3DB_LIBRARY_DIR=expected, RAG3DB_INCLUDE_DIR=expected,
                          RAG3DB_SHARED='1', LD_LIBRARY_PATH=expected)
    assert not list(Path(private).iterdir()), 'no Cargo or library store creation'
    print(json.dumps(dict(expected_red=red.returncode, resolver_profile=profile,
                          verdict='VERT', cargo=False, db=False)))
