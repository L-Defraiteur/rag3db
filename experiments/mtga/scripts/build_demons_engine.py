"""Assemble the Demon experiment from saved MCP records, validating ownership."""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from export_deck import arena_clipboard

RESEARCH = ROOT / 'data/deck-research-2026-09-20/demons'
OUT = ROOT / 'data/deck-drafts/experiments-2026-09-20/04-le-notaire-des-enfers'
pool_by_id = {}
for filename in ['final_pool.json', 'all_demons.json']:
    for hit in json.loads((RESEARCH / filename).read_text())['result']:
        pool_by_id[hit['data']['arena_id']] = hit['data']
pool = list(pool_by_id.values())
assert len({c['snapshot_id'] for c in pool}) == 1
spells = {
    'Greedy Freebooter': 4, 'Shambling Ghast': 1, 'Reassembling Skeleton': 2,
    'Rune-Scarred Demon': 4, 'Vilis, Broker of Blood': 1, 'Vile Mutilator': 2,
    'Bloodletter of Aclazotz': 1, 'Desecration Demon': 1, 'Abyssal Harvester': 1,
    'Faithless Looting': 4, 'Victimize': 3, 'Zombify': 3, 'Demand Answers': 2,
    'Bitter Triumph': 2, 'Withering Torment': 2, 'Exsanguinate': 1,
    'Rush of Dread': 1, 'Duress': 1,
}
lands = {'Swamp': 8, 'Mountain': 4, 'Blazemire Verge': 2, 'Blood Crypt': 1,
         'Dark Fortress': 1, 'Raucous Theater': 1, 'Restless Vents': 1,
         'Razortrap Gorge': 4, 'Bloodfell Caves': 2}


def allocate(counts, is_land):
    rows = []
    for name, quantity in counts.items():
        candidates = sorted((c for c in pool if c['name_en'] == name and c['is_primary'] and c['owned'] > 0),
                            key=lambda c: (-c['owned'], c['arena_id']))
        assert candidates and all(c['is_land'] == is_land for c in candidates), name
        basic = name in {'Swamp', 'Mountain'}
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
assert sum(c['quantity'] for c in creatures) == 17
assert sum(c['quantity'] for c in creatures if 'Demon' in c['type_en']) == 10
validation = {'60_cards': True, 'lands': 24, 'creatures': 17, 'demons': 10,
              'owned_nonbasic_quantities_per_printing': True,
              'max_four_nonbasic_by_name': True, 'basic_printings_unlocked': True,
              'no_craft_required_in_snapshot': True, 'arena_import_tested': False,
              'performance_measured': False}
draft = {
    'schema_version': 'mtga.deck-draft.v1',
    'id': stable_id('mtga:experiment:2026-09-20:le-notaire-des-enfers'),
    'revision': 1, 'name': 'Le notaire des enfers', 'status': 'draft_unplayed',
    'created_at': datetime.now(timezone.utc).isoformat(),
    'source_snapshot_id': pool[0]['snapshot_id'], 'source_deck_id': None,
    'format': 'Historic', 'queue': 'Best-of-One',
    'main_deck': {'lands': ls, 'nonlands': ns}, 'sideboard': [],
    'validation': validation,
    'mana_analysis': {'potential_black_land_sources': 20, 'potential_red_land_sources': 16,
                      'always_tapped_lands': 4, 'additional_conditional_tapped_gorges': 4,
                      'notes': ['Verge needs Swamp/Mountain for red.',
                                'Dark Fortress needs a basic land after its entry turn.',
                                'Blood Crypt can cost 2 life to enter untapped.',
                                'Potential source counts are not casting probabilities.']},
    'research': {'directory': str(RESEARCH), 'final_pools': ['final_pool.json', 'all_demons.json'],
                 'transport': 'official MCP SDK -> rag3db + lucivy + local BGE-M3',
                 'approach': 'Hybrid discovery plus exact Demon type filtering, then owned support selection.',
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
