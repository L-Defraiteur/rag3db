"""Build a conservative v3 from the exact v2 and MCP-discovered owned upgrades."""
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from export_deck import arena_clipboard

DATA = ROOT / 'data'
BASE = DATA / 'deck-drafts/mycotyrant-vanguard-v2.json'
RESEARCH = DATA / 'deck-research-2026-09-20/mycotyrant-upgrade'
old = json.loads(BASE.read_text())
pool = {}
for filename in ['final_pool.json']:
    for hit in json.loads((RESEARCH / filename).read_text())['result']:
        pool[hit['data']['arena_id']] = hit['data']
assert {c['snapshot_id'] for c in pool.values()} == {old['source_snapshot_id']}

changes = {'Ghalta, Stampede Tyrant': -1, 'Vile Mutilator': 1,
           'Snarling Gorehound': -1, 'Reassembling Skeleton': 1,
           'Rubblebelt Maverick': -1, 'Six': 1,
           'Victimize': -1, 'Unearth': 1,
           'Rite of Oblivion': -1, 'Skyfisher Spider': 1,
           'Plains': -2, 'Bleachbone Verge': 2}
counts = Counter()
for section in old['main_deck'].values():
    for card in section:
        counts[card['name_en']] += card['quantity']
for name, delta in changes.items():
    counts[name] += delta
assert sum(counts.values()) == 60 and all(q >= 0 for q in counts.values())
rows = []
crafts = []
for name, quantity in counts.items():
    if not quantity:
        continue
    candidates = sorted((c for c in pool.values() if c['name_en'] == name and c.get('owned', 0) > 0),
                        key=lambda c: (-c['owned'], c['arena_id']))
    assert candidates, name
    basic = name in {'Plains', 'Forest', 'Swamp'}
    assert basic or quantity <= 4
    for card in candidates:
        taken = quantity if basic else min(quantity, card['owned'])
        if taken:
            rows.append({**card, 'quantity': taken, 'card_id': card.get('key', card.get('card_id')),
                         'owned_this_printing': card['owned'], 'craft_required': 0,
                         'basic_land_unlimited': basic})
        quantity -= taken
        if quantity == 0:
            break
    if quantity:
        assert name == 'Unearth' and quantity == 1
        row = next(c for c in rows if c['name_en'] == name)
        row['quantity'] += quantity
        row['craft_required'] = quantity
        crafts.append({'arena_id': row['arena_id'], 'name_en': name, 'quantity': quantity,
                       'rarity': 'common', 'status': 'proposed_not_crafted',
                       'rarity_source': 'https://mtg.wtf/card/2x2/357/Unearth'})

lands = [c for c in rows if c.get('is_land', c['type_en'].startswith('Land'))]
nonlands = [c for c in rows if c not in lands]
assert sum(c['quantity'] for c in lands) == 24
assert sum(c['quantity'] for c in nonlands) == 36
assert counts['Gaea\'s Blessing'] == counts['Thunderbond Vanguard'] == counts['The Mycotyrant'] == 2
assert counts['Mesmeric Orb'] == 2
assert all(c['quantity'] <= c['owned'] + c['craft_required'] for c in rows if not c['basic_land_unlimited'])

draft = {
    'schema_version': 'mtga.deck-draft.v1',
    'id': stable_id('mtga:experiment:2026-09-20:mycotyrant-vanguard-v3'),
    'revision': 3, 'name': 'Mycotyrant — Vanguard / Roots — v3 résilience',
    'status': 'draft_unplayed_requires_craft', 'created_at': datetime.now(timezone.utc).isoformat(),
    'source_draft_id': old['id'], 'source_deck_id': old['source_deck_id'],
    'source_snapshot_id': old['source_snapshot_id'], 'format': 'Historic',
    'constraints': old['constraints'], 'main_deck': {'lands': lands, 'nonlands': nonlands},
    'sideboard': [], 'changes_by_name': changes, 'craft_plan': crafts,
    'research': {'directory': str(RESEARCH), 'owned_search': 'MCP -> rag3db/lucivy/local BGE-M3',
                 'craft_metadata': 'exact name lookups in local source catalog; rarity confirmation linked in craft plan'},
    'validation': {'60_cards': True, '24_lands': True, 'two_library_resets': True,
                   'owned_or_explicit_craft': True, 'arena_import_tested': False,
                   'performance_measured': False},
}
text = arena_clipboard(draft)
assert sum(int(s.split()[0]) for s in text.splitlines() if s[:1].isdigit()) == 60
out = DATA / 'deck-drafts/mycotyrant-vanguard-v3'
out.with_suffix('.json').write_text(json.dumps(draft, ensure_ascii=False, indent=2) + '\n')
out.with_suffix('.arena.txt').write_text(text)
print(text)
