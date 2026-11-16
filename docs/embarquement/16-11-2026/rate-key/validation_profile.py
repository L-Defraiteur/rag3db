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
    # État précédant le correctif : extraction du profil actuel pour son témoin rouge.
    env = dict(inherited)
    env.update(RAG3WEAVER_STATE_DIR=str(Path(proof_dir)/'state'),
               RAG3WEAVER_JOURNAUX_DIR=str(Path(proof_dir)/'journals'),
               RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(Path(proof_dir)/'templates'))
    return env


def check_private_environment(proof_dir, env):
    # Précontrôle initial permissif ; les témoins doivent montrer son insuffisance.
    return {key: env[key] for key in PRIVATE_PATHS if key in env}
