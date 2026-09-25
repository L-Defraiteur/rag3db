"""Persistent identity contract; standard library only for offline export scripts."""
from uuid import UUID, uuid5

NAMESPACE = UUID('c10f10dc-4686-50c2-9637-5a694b84c33c')

def stable_id(logical_id: str) -> str:
    return str(uuid5(NAMESPACE, logical_id))

def card_id(arena_id: int) -> str:
    return stable_id(f'mtga:card:{arena_id}')
