"""Build the Simic clone experiment from records returned by the actual MCP."""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from export_deck import arena_clipboard

RESEARCH = ROOT / 'data/deck-research-2026-09-23/paradox'
OUT = ROOT / 'data/deck-drafts/experiments-2026-09-23/01-paradoxe-des-miroirs'
pool = [hit['data'] for hit in json.loads((RESEARCH / 'final_pool.json').read_text())['result']]
spells = {
    'Llanowar Elves': 4, 'Ilysian Caryatid': 4, 'Fuel Tank Feaster': 2,
    'Pond Prophet': 4, 'Transit Mage': 2, 'Mulldrifter': 1,
    'Overlord of the Hauntwoods': 1, 'Vaultborn Tyrant': 1, 'Meteor Golem': 1,
    'Paradox Engine': 3, 'Heirloom Epic': 1,
    'Rite of Replication': 2, 'Mirror Room // Fractured Realm': 2,
    'Waxen Shapethief': 1, 'Surgical Metamorph': 1, 'Applied Geometry': 1,
    'Snakeskin Veil': 2, 'Into the Flood Maw': 2, 'Three Steps Ahead': 1,
}
lands = {'Breeding Pool': 4, 'Dreamroot Cascade': 1, 'Hedge Maze': 2,
         'Forest': 9, 'Island': 8}


def allocate(counts, is_land):
    result = []
    for name, count in counts.items():
        candidates = sorted((c for c in pool if c['name_en'] == name and c['is_primary'] and c['owned'] > 0),
                            key=lambda c: (-c['owned'], c['arena_id']))
        basic = name in {'Forest', 'Island'}
        assert candidates and all(c['is_land'] == is_land for c in candidates), name
        assert basic or (count <= 4 and sum(c['owned'] for c in candidates) >= count), name
        for card in candidates:
            amount = count if basic else min(count, card['owned'])
            if amount:
                clean = {k: v for k, v in card.items() if not k.startswith('_')}
                result.append({**clean, 'quantity': amount, 'card_id': card['key'],
                               'owned_this_printing': card['owned'], 'basic_land_unlimited': basic})
            count -= amount
            if count == 0:
                break
        assert count == 0, name
    return result


nonlands, land_rows = allocate(spells, False), allocate(lands, True)
rows = nonlands + land_rows
assert sum(spells.values()) == 36 and sum(lands.values()) == 24
assert len({c['snapshot_id'] for c in rows}) == 1
assert all(set(c['color_identity']) <= {'Green', 'Blue'} for c in rows)
owned_snapshot = {c['key']: c for c in map(json.loads, (ROOT / 'data/engine-collection.jsonl').open())}
for c in rows:
    original = owned_snapshot[c['key']]
    assert original['arena_id'] == c['arena_id']
    assert c['basic_land_unlimited'] or c['quantity'] <= original['owned']
sources = {symbol: sum(c['quantity'] for c in land_rows if symbol in c['mana_symbols_possible'])
           for symbol in ('G', 'U')}
assert sources == {'G': 16, 'U': 15}
searches = {}
for filename in ['clones', 'untap', 'mana', 'draw_creatures', 'finish', 'repeat', 'big_targets']:
    metadata = json.loads((RESEARCH / (filename + '.json')).read_text())['metadata']['render.meta']
    assert metadata['vectorCount'] > 0 and not metadata['partial']
    searches[filename] = metadata
validation = {
    'cards': 60, 'lands': 24, 'nonlands': 36, 'land_ratio': 0.4,
    'owned_quantities_checked_per_printing_against_snapshot': True,
    'max_four_nonbasic_by_name': True, 'crafts_required_in_snapshot': 0,
    'color_identity': ['Green', 'Blue'], 'potential_land_sources': sources,
    'mana_creatures': 10, 'dedicated_copy_cards': 7,
    'arena_import_tested': False, 'performance_measured': False,
    'banlist_review': {'format': 'Historic', 'checked_on': '2026-09-23',
                       'source': 'https://magic.wizards.com/en/banned-restricted-list',
                       'selected_names_on_historic_banlist': []},
}
deck = {
    'schema_version': 'mtga.deck-draft.v1',
    'id': stable_id('mtga:experiment:2026-09-23:paradoxe-des-miroirs'),
    'revision': 1, 'name': 'Le paradoxe des miroirs', 'status': 'draft_unplayed',
    'created_at': datetime.now(timezone.utc).isoformat(),
    'source_snapshot_id': rows[0]['snapshot_id'], 'source_deck_id': None,
    'format': 'Historic', 'queue': 'Best-of-One',
    'main_deck': {'lands': land_rows, 'nonlands': nonlands}, 'sideboard': [],
    'validation': validation,
    'research': {'directory': str(RESEARCH), 'transport': 'MCP stdio -> rag3weaver -> rag3db + lucivy + local BGE-M3',
                 'hybrid_searches': searches, 'final_pool': 'final_pool.json'},
    'mana_analysis': {'potential_land_sources': sources, 'always_tapped_lands': 2,
                      'turn_one_green_sources_with_shock_payment': 13,
                      'notes': ['Breeding Pool costs 2 life to enter untapped.',
                                'Dreamroot Cascade needs two other lands.',
                                'Paradox Engine does not untap lands or remove summoning sickness.',
                                'Ilysian Caryatid produces two mana only with a power-4 creature.',
                                'Fuel Tank Feaster cost reduction requires its beginning-of-main-phase trigger.']},
}
OUT.mkdir(parents=True, exist_ok=True)
clipboard = arena_clipboard(deck)
assert sum(int(line.split()[0]) for line in clipboard.splitlines() if line[:1].isdigit()) == 60
for name, content in [('deck.json', deck), ('validation.json', validation)]:
    (OUT / name).write_text(json.dumps(content, ensure_ascii=False, indent=2) + '\n')
(OUT / 'arena.txt').write_text(clipboard)
print(OUT)
print(clipboard)
