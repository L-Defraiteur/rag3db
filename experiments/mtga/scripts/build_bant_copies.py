"""Assemble the user's clone-heavy variant from fresh deck-log and MCP records."""
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from export_deck import arena_clipboard

RESEARCH = ROOT / 'data/deck-research-2026-09-23/bant-copies'
OUT = ROOT / 'data/deck-drafts/experiments-2026-09-23/02-bant-miroirs-et-mana'
chosen = json.loads((RESEARCH / 'chosen.json').read_text())
pool = [h['data'] for h in json.loads((RESEARCH / 'assembly_pool.json').read_text())['result']]
source = json.loads((RESEARCH / 'current-deck.json').read_text())
catalog = {c['arena_id']: c for c in map(json.loads, (ROOT / 'data/cards.jsonl').open())}
original = Counter()
source_rows = []
for entry in source['piles']['MainDeck']:
    card = catalog[entry['cardId']]
    original[card['name_en']] += entry['quantity']
    source_rows.append({**entry, 'name_en': card['name_en']})
assert sum(original.values()) == 69
snapshot = {c['key']: c for c in map(json.loads, (ROOT / 'data/engine-collection.jsonl').open())}


def allocate(counts, is_land):
    rows = []
    for name, count in counts.items():
        candidates = sorted((c for c in pool if c['name_en'] == name and c['is_primary'] and c['owned'] > 0),
                            key=lambda c: (-c['owned'], c['arena_id']))
        basic = name in {'Forest', 'Island', 'Plains'}
        assert candidates and all(c['is_land'] == is_land for c in candidates), name
        assert basic or (count <= 4 and sum(c['owned'] for c in candidates) >= count), name
        for c in candidates:
            amount = count if basic else min(count, c['owned'])
            if amount:
                assert basic or amount <= snapshot[c['key']]['owned'], name
                assert snapshot[c['key']]['arena_id'] == c['arena_id']
                rows.append({**{k: v for k, v in c.items() if not k.startswith('_')},
                             'quantity': amount, 'card_id': c['key'],
                             'owned_this_printing': c['owned'], 'basic_land_unlimited': basic})
            count -= amount
            if not count:
                break
        assert count == 0, name
    return rows


nonlands, lands = allocate(chosen['spells'], False), allocate(chosen['lands'], True)
rows = nonlands + lands
assert sum(c['quantity'] for c in nonlands) == 42
assert sum(c['quantity'] for c in lands) == 28
assert all(set(c['color_identity']) <= {'Green', 'Blue', 'White'} for c in rows)
assert len({c['snapshot_id'] for c in rows}) == 1
new = Counter({**chosen['spells'], **chosen['lands']})
assert not any(n in new for n in ['Paradox Engine', 'Transit Mage', 'Heirloom Epic'])
assert all(new[n] >= original[n] for n in ["Relm's Sketching", 'Extravagant Replication',
                                          'Doppelgang', 'For the Common Good'])
copy_names = ['Reflection Net', 'Doppelgang', 'Rite of Replication', 'Mirror Room // Fractured Realm',
              "Relm's Sketching", 'Extravagant Replication', 'For the Common Good',
              'Applied Geometry', 'Surgical Metamorph', 'Niko, Light of Hope']
copy_count = sum(new[n] for n in copy_names)
assert copy_count == 15
mana_sources = {s: sum(c['quantity'] for c in lands if s in c['mana_symbols_possible']) for s in 'GUW'}
assert mana_sources == {'G': 17, 'U': 15, 'W': 9}
research = {}
for n in ['mass_mana', 'white_copies']:
    meta = json.loads((RESEARCH / (n + '.json')).read_text())['metadata']['render.meta']
    assert meta['vectorCount'] > 0 and not meta['partial']
    research[n] = meta
validation = {
    'cards': 70, 'lands': 28, 'nonlands': 42, 'land_ratio': 0.4,
    'copy_cards': copy_count, 'additional_token_multipliers': 2,
    'natural_mana_creatures': 10, 'grant_mana_to_all_creatures': 4,
    'land_color_sources_potential': mana_sources,
    'owned_quantities_checked_per_printing': True, 'max_four_nonbasic_by_name': True,
    'crafts_required_in_snapshot': 0, 'source_deck_cards': 69,
    'arena_import_tested': False, 'performance_measured': False,
    'collection_refreshed_this_run': False,
    'banlist_review': {'format': 'Historic', 'source': 'https://magic.wizards.com/en/banned-restricted-list',
                       'checked_on': '2026-09-23', 'selected_names_on_historic_banlist': []},
}
deck = {
    'schema_version': 'mtga.deck-draft.v1',
    'id': stable_id('mtga:experiment:2026-09-23:bant-miroirs-et-mana'),
    'revision': 1, 'name': 'Bant — Miroirs et mana', 'status': 'draft_unplayed',
    'created_at': datetime.now(timezone.utc).isoformat(),
    'source_snapshot_id': rows[0]['snapshot_id'], 'source_deck_id': source['id'],
    'source_deck_name': source['name'], 'format': 'Historic', 'queue': 'Best-of-One',
    'main_deck': {'lands': lands, 'nonlands': nonlands}, 'sideboard': [],
    'validation': validation,
    'mana_analysis': {'potential_land_sources': mana_sources, 'always_tapped_lands': 4,
                      'conditional_lands': ['Breeding Pool', 'Temple Garden', 'Hallowed Fountain',
                                            'Dreamroot Cascade', 'Port Town'],
                      'notes': ['White is a splash, supported by Birds, Caryatids, Kami and global mana abilities.',
                                'Creature mana is vulnerable to removal and summoning sickness.',
                                'Cryptolith Rite and Enduring Vitality do not double mana if both are present.']},
    'research': {'directory': str(RESEARCH), 'transport': 'MCP -> rag3db + lucivy + local BGE-M3',
                 'hybrid_searches': research, 'assembly_pool': 'assembly_pool.json',
                 'assembly_provenance': ['final_pool.json', 'key_cards.json']},
}
diff = {'source_deck_id': source['id'], 'source_deck_name': source['name'],
        'source_cards': sum(original.values()), 'target_cards': sum(new.values()),
        'removed': dict(original - new), 'added': dict(new - original)}
OUT.mkdir(parents=True, exist_ok=True)
clipboard = arena_clipboard(deck)
assert sum(int(l.split()[0]) for l in clipboard.splitlines() if l[:1].isdigit()) == 70
for filename, obj in [('deck.json', deck), ('validation.json', validation), ('changes-from-current.json', diff),
                      ('source-deck.json', {**source, 'resolved_main_deck': source_rows})]:
    (OUT / filename).write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')
(OUT / 'arena.txt').write_text(clipboard)
print(OUT)
print(json.dumps(validation, ensure_ascii=False, indent=2))
print(clipboard)
