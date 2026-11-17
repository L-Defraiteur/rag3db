"""Common physical grant watcher. No provider policy belongs in this module."""
def validate_armed(*args):
    raise ValueError('actor admission not implemented')

def validate_heartbeat(*args):
    raise ValueError('actor heartbeat not implemented')

def run_actor(*args, **kwargs):
    raise ValueError('actor loop not implemented')
