"""Assemble the Frog experiment from saved MCP records, validating ownership."""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from export_deck import arena_clipboard

RESEARCH = ROOT / 'data/deck-research-2026-09-20/tribal'
OUT = ROOT / 'data/deck-drafts/experiments-2026-09-20/03-la-mare-a-teleportation'
pool = [h['data'] for h in json.loads((RESEARCH / 'frog_pool.json').read_text())['result']]
assert len({c['arena_id'] for c in pool}) == len(pool)
assert len({c['snapshot_id'] for c in pool}) == 1
spells = {
    'Sunshower Druid': 4, 'Valley Mightcaller': 1, 'Pond Prophet': 4,
    'Three Tree Scribe': 4, 'Dour Port-Mage': 2, 'Fountainport Charmer': 2,
    'Clement, the Worrywort': 3, 'Long River Lurker': 3,
    'Lilysplash Mentor': 2, 'Dreamdew Entrancer': 1,
    'Splash Portal': 3, 'Polliwallop': 3, 'Snakeskin Veil': 2,
    'Pawpatch Formation': 1, 'Into the Flood Maw': 1,
}
lands = {'Forest': 7, 'Island': 6, 'Breeding Pool': 4,
         'Cavern of Souls': 4, 'Hedge Maze': 2, 'Dreamroot Cascade': 1}


def allocate(counts, is_land):
    rows = []
    for name, quantity in counts.items():
        candidates = sorted((c for c in pool if c['name_en'] == name and c['is_primary'] and c['owned'] > 0),
                            key=lambda c: (-c['owned'], c['arena_id']))
        assert candidates and all(c['is_land'] == is_land for c in candidates), name
        basic = name in {'Forest', 'Island'}
        assert basic or (quantity <= 4 and sum(c['owned'] for c in candidates) >= quantity), name
        for c in candidates:
            amount = quantity if basic else min(quantity, c['owned'])
            if amount:
                rows.append({**c, 'quantity': amount, 'card_id': c['key'],
                             'owned_this_printing': c['owned'], 'basic_land_unlimited': basic})
            quantity -= amount
            if quantity == 0:
                break
        assert quantity == 0, name
    return rows


ls, ns = allocate(lands, True), allocate(spells, False)
assert sum(lands.values()) == 24 and sum(spells.values()) == 36
creatures = [c for c in ns if 'Creature' in c['card_types']]
assert sum(c['quantity'] for c in creatures) == 26
assert all('Frog' in c['type_en'] for c in creatures)
validation = {'60_cards': True, 'lands': 24, 'frogs': 26,
              'owned_nonbasic_quantities_per_printing': True,
              'max_four_nonbasic_by_name': True, 'basic_printings_unlocked': True,
              'no_craft_required_in_snapshot': True, 'arena_import_tested': False,
              'performance_measured': False}
draft = {
    'schema_version': 'mtga.deck-draft.v1',
    'id': stable_id('mtga:experiment:2026-09-20:la-mare-a-teleportation'),
    'revision': 1, 'name': 'La mare à téléportation', 'status': 'draft_unplayed',
    'created_at': datetime.now(timezone.utc).isoformat(),
    'source_snapshot_id': pool[0]['snapshot_id'], 'source_deck_id': None,
    'format': 'Historic', 'queue': 'Best-of-One',
    'main_deck': {'lands': ls, 'nonlands': ns}, 'sideboard': [],
    'validation': validation,
    'mana_analysis': {'potential_green_land_sources_for_spells': 14,
                      'potential_blue_land_sources_for_spells': 13,
                      'additional_frog_only_colored_land_sources': 4,
                      'always_tapped_lands': 2,
                      'notes': ['Cavern names Frog; colored mana cannot pay noncreature spells or abilities.',
                                'Clement grants mana abilities to Frogs for creature spells only; summoning sickness applies.',
                                'Dreamroot Cascade requires two other lands to enter untapped.',
                                'Breeding Pool requires 2 life to enter untapped.']},
    'research': {'directory': str(RESEARCH), 'final_pool': 'frog_pool.json',
                 'transport': 'official MCP SDK -> rag3db + lucivy + local BGE-M3',
                 'approach': 'Hybrid discovery of three themes, then exact owned Frog and support selection.',
                 'source_deck': None},
}
OUT.mkdir(parents=True, exist_ok=True)
clipboard = arena_clipboard(draft)
assert sum(int(line.split()[0]) for line in clipboard.splitlines() if line[:1].isdigit()) == 60
(OUT / 'arena.txt').write_text(clipboard)
(OUT / 'deck.json').write_text(json.dumps(draft, ensure_ascii=False, indent=2) + '\n')
(OUT / 'validation.json').write_text(json.dumps(validation, indent=2) + '\n')
print(OUT)
print(clipboard)
