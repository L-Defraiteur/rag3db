from scripts.lib import Catalog


def charger(nom):
    return [Catalog.create(), nom]


manifest = charger('m'); cards = charger('c'); owned = [c for c in cards if c]; owned_ids = {id(c) for c in owned}; snapshot = charger('s')[0]
