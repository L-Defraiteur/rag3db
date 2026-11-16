"""Construction et contrôle du profil privé du pilote, sans lancer de programme."""
from pathlib import Path

PRIVATE_PATHS = {
    'XDG_STATE_HOME': 'xdg-state',
    'RAG3WEAVER_STATE_DIR': 'state-fixture',
    'XDG_CONFIG_HOME': 'xdg-config',
    'XDG_DATA_HOME': 'xdg-data',
    'XDG_CACHE_HOME': 'xdg-cache',
    'TMPDIR': 'tmp', 'TMP': 'tmp', 'TEMP': 'tmp',
    'RAG3WEAVER_CACHE_DIR': 'cache-fixture',
    'RAG3WEAVER_JOURNAUX_DIR': 'journals',
    'RAG3WEAVER_BUILTIN_TEMPLATES_DIR': 'templates',
}


def private_environment(proof_dir, inherited):
    root = Path(proof_dir)
    env = dict(inherited)
    env.update({key: str(root/name) for key, name in PRIVATE_PATHS.items()})
    return env


def check_private_environment(proof_dir, env):
    root = Path(proof_dir)
    if not root.is_absolute() or root.resolve() != root:
        raise ValueError('private proof root must be absolute and unlinked')
    paths = {}
    for key, name in PRIVATE_PATHS.items():
        expected = root/name
        if env.get(key) != str(expected) or expected.resolve() != expected:
            raise ValueError('private profile path escaped: '+key)
        paths[key] = str(expected)
    state = Path(paths['XDG_STATE_HOME'])
    fixture = Path(paths['RAG3WEAVER_STATE_DIR'])
    if state.parent != fixture.parent or state.is_relative_to(fixture) or fixture.is_relative_to(state):
        raise ValueError('XDG state and fixture state must be disjoint siblings')
    store = Path(env['RAG3DB_LIBS_DIR'])
    if not store.is_absolute() or store.is_relative_to(root) or root.is_relative_to(store):
        raise ValueError('SDK store must stay external to private profile')
    # Liste positive uniquement : pas de jeton, credentials ou bus personnel dans le reçu.
    return dict(paths=paths, shared_sdk_store=str(store))
