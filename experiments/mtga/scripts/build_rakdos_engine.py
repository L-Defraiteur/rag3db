"""Export a new experimental deck using only card records returned by the MCP."""
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from identity import stable_id
from export_deck import arena_clipboard

RESEARCH = ROOT / 'data/deck-research-2026-09-20/farfelu'
OUT = ROOT / 'data/deck-drafts/experiments-2026-09-20/02-le-fisc-de-rakdos'
pool = [h['data'] for h in json.loads((RESEARCH / 'chosen_deck_pool.json').read_text())['result']]
land_research = ROOT / 'data/deck-research-2026-09-20/rakdos-lands/damage_lands.json'
pool.extend(h['data'] for h in json.loads(land_research.read_text())['result']
            if h['data']['name_en'] == 'Jagged Barrens')
# The snapshot remains immutable; this extra copy is reported by the user.
assert sum(c['name_en'] == 'Razorkin Needlehead' for c in pool) == 1
def available(card):
    return card['owned'] + (1 if card['name_en'] == 'Razorkin Needlehead' else 0)
assert len({c['arena_id'] for c in pool}) == len(pool)
snapshots = {c['snapshot_id'] for c in pool}
assert len(snapshots) == 1

nonlands = {
    'Razorkin Needlehead': 4,
    'Scalding Viper': 3,
    'Magebane Lizard': 2,
    'Screaming Nemesis': 2,
    'Scrawling Crawler': 2,
    'Ob Nixilis, Captive Kingpin': 1,
    'Grievous Wound': 2,
    'Burst Lightning': 4,
    'Abrade': 2,
    'Go for the Throat': 2,
    'Bitter Triumph': 1,
    'Cost of Brilliance': 2,
    'Blood Pact': 1,
    'Unearth': 2,
    'Duress': 2,
    'Soul-Guide Lantern': 1,
    'Pyroclasm': 1,
    'Extinction Event': 1,
    'Insatiable Avarice': 1,
}
lands = {
    'Mountain': 6,
    'Swamp': 6,
    'Blazemire Verge': 2,
    'Blood Crypt': 1,
    'Dark Fortress': 1,
    'Raucous Theater': 1,
    'Restless Vents': 1,
    'Razortrap Gorge': 4,
    'Bloodfell Caves': 1,
    'Jagged Barrens': 1,
}


def allocate(counts, is_land):
    result = []
    for name, quantity in counts.items():
        candidates = sorted(
            (c for c in pool if c['name_en'] == name and c['owned'] > 0 and c['is_primary']),
            key=lambda c: (-c['owned'], c['arena_id']),
        )
        assert candidates, name
        assert all(c['is_land'] == is_land for c in candidates), name
        basic = name in {'Mountain', 'Swamp'}
        if basic:
            card = candidates[0]
            result.append({**card, 'card_id': card['key'], 'quantity': quantity,
                           'owned_this_printing': card['owned'], 'basic_land_unlimited': True})
            continue
        assert quantity <= 4 and sum(available(c) for c in candidates) >= quantity, name
        for card in candidates:
            taken = min(quantity, available(card))
            if taken:
                result.append({**card, 'card_id': card['key'], 'quantity': taken,
                               'owned_this_printing': card['owned'],
                               'user_reported_extra_copies': max(0, taken - card['owned'])})
            quantity -= taken
            if quantity == 0:
                break
    return result


land_rows, nonland_rows = allocate(lands, True), allocate(nonlands, False)
assert sum(lands.values()) == 24 and sum(nonlands.values()) == 36
assert sum(c['quantity'] for c in nonland_rows if 'Creature' in c['card_types']) == 14
for card in land_rows:
    allowed = set(card['mana_symbols_possible'])
    assert allowed & {'B', 'R'}, card['name_en']

validation = {
    '60_cards': True,
    'lands': 24,
    'creatures': 14,
    'max_four_nonbasic_by_name': True,
    'nonbasic_quantities_owned_per_printing': False,
    'quantities_covered_by_snapshot_plus_user_report': True,
    'pending_snapshot_verification': {'Razorkin Needlehead': 1},
    'basic_land_printings_unlocked': True,
    'no_crafting_required_in_snapshot': False,
    'no_additional_craft_required_if_reported_purchase_confirmed': True,
    'arena_import_tested': False,
    'match_results': [],
}
mana = {
    'potential_red_land_sources': 18,
    'potential_black_land_sources': 18,
    'always_tapped_lands': 4,
    'additional_tapped_lands_unless_a_player_has_at_most_13_life': 4,
    'conditions': [
        'Blazemire Verge: red only with a Swamp or Mountain; black unconditional.',
        'Dark Fortress: colored mana only on its entry turn or while controlling a basic land.',
        'Blood Crypt: pay 2 life to enter untapped.',
        'Scalding Viper: only the red creature side is planned; no reliable blue for the Adventure.',
        'Insatiable Avarice: BBB draw mode is a later option, not assumed available turn three.',
    ],
    'limitations': 'Source counts are potential, not turn-by-turn casting probabilities.',
}
roles = {
    'draw_punishers': ['Razorkin Needlehead', 'Scrawling Crawler'],
    'spell_punishers': ['Scalding Viper', 'Magebane Lizard'],
    'damage_multiplier_by_life_halving': ['Grievous Wound'],
    'card_advantage_from_one_life_events': ['Ob Nixilis, Captive Kingpin'],
    'targeted_draw_self_or_opponent': ['Cost of Brilliance', 'Blood Pact', 'Insatiable Avarice'],
    'anti_lifegain': ['Screaming Nemesis', 'Grievous Wound'],
    'creature_recursion': ['Unearth'],
    'artifact_answer': ['Abrade'],
    'graveyard_answer': ['Soul-Guide Lantern'],
}
draft = {
    'schema_version': 'mtga.deck-draft.v1',
    'id': stable_id('mtga:experiment:2026-09-20:le-fisc-de-rakdos'),
    'revision': 2,
    'name': 'Le fisc de Rakdos — pioche et sorts taxés',
    'status': 'draft_unplayed',
    'created_at': datetime.now(timezone.utc).isoformat(),
    'ownership_note': 'Fourth Razorkin reported purchased by user; printing assumed DSK 153, pending resync.',
    'source_snapshot_id': next(iter(snapshots)),
    'source_deck_id': None,
    'format': 'Historic',
    'queue': 'Best-of-One',
    'legality_status': 'Reviewed against published Historic bans on 2026-09-20; Arena import pending.',
    'main_deck': {'lands': land_rows, 'nonlands': nonland_rows},
    'sideboard': [],
    'roles': roles,
    'mana_analysis': mana,
    'research': {
        'transport': 'official MCP SDK -> generated backend -> rag3db + lucivy + local BGE-M3',
        'directory': str(RESEARCH),
        'selection': 'Original punishment plan from owned cards; no leaderboard list copied.',
        'dense_queries': ['punish', 'sweep', 'odd_combo', 'artifacts', 'damage', 'life_gift', 'reflect'],
        'final_ownership_evidence': 'chosen_deck_pool.json',
        'additional_land_evidence': str(land_research),
        'sources': ['https://magic.wizards.com/en/banned-restricted-list',
                    'https://magic.wizards.com/en/news/mtg-arena/state-of-the-formats-2026'],
    },
    'validation': validation,
}
OUT.mkdir(parents=True, exist_ok=True)
clipboard = arena_clipboard(draft)
assert sum(int(line.split()[0]) for line in clipboard.splitlines() if line[:1].isdigit()) == 60
assert Counter({**nonlands, **lands}) == Counter({
    name: sum(c['quantity'] for c in land_rows + nonland_rows if c['name_en'] == name)
    for name in nonlands | lands
})
(OUT / 'deck.json').write_text(json.dumps(draft, ensure_ascii=False, indent=2) + '\n')
(OUT / 'arena.txt').write_text(clipboard)
(OUT / 'validation.json').write_text(json.dumps({'validation': validation, 'mana': mana}, indent=2) + '\n')
print(OUT)
print(clipboard)
