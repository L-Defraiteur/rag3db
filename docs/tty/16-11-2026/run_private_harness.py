"""Run the unchanged harness except its explicit private journal-state path."""
import hashlib
import json
from pathlib import Path
import sys

source, binary, fake, fixture, themes, output, state = sys.argv[1:]
path = Path(source)
original = path.read_text()
old = 'ETAT_TTY = "$HOME/.local/state/rag3weaver/tty"'
assert original.count(old) == 1
assert Path(state).is_absolute() and Path(state).is_dir()
patched = original.replace(old, 'ETAT_TTY = ' + repr(str(Path(state) / 'rag3weaver/tty')))
evidence = Path(output).parent
(evidence / 'harness-private-source.py').write_text(patched)
(evidence / 'harness-transform.json').write_text(json.dumps({
    'source': str(path), 'sha256': hashlib.sha256(original.encode()).hexdigest(),
    'private_sha256': hashlib.sha256(patched.encode()).hexdigest(),
    'replacement_count': 1, 'state_path': state,
    'only_change': 'ETAT_TTY private path; no HOME modification, scenarios unchanged',
}, indent=2))
sys.argv = [source, 'local', binary, fake, fixture, themes, output, 'tmux-local-ouvert']
exec(compile(patched, source, 'exec'), {'__name__': '__main__', '__file__': source})
