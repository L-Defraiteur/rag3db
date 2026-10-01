#!/usr/bin/env python3
"""Write the trusted facts read by the submit_deck harness hooks.

Reads the engine catalog and wildcard snapshots, writes backend/harness/cards.json
and backend/harness/wildcards.json. Rerun after each catalog/collection refresh:
the backend loads these files at startup, the model never chooses them.
"""
import json
import re
from collections import defaultdict
from pathlib import Path

MTGA = Path(__file__).resolve().parents[1]
DATA = MTGA / 'data'
OUT = MTGA / 'backend/harness'
COLORS = 'WUBRG'
BASIC_TYPES = {'Plains': 'W', 'Island': 'U', 'Swamp': 'B', 'Mountain': 'R', 'Forest': 'G'}
ANY_NUMBER = re.compile(r'A deck can have any number of cards named')
UP_TO = re.compile(r'A deck can have up to (\w+) cards named')
WORDS = {'two': 2, 'three': 3, 'four': 4, 'five': 5, 'six': 6, 'seven': 7, 'eight': 8, 'nine': 9, 'ten': 10}
SEARCH = re.compile(r'[Ss]earch your library for (?:an? |up to \w+ )?(basic land|[^.]*?)(?: cards?)?[,.]')


def copy_limit(card):
    if card['is_basic_land'] or ANY_NUMBER.search(card['text_en']):
        return 250
    match = UP_TO.search(card['text_en'])
    return WORDS.get(match.group(1).lower(), 4) if match else 4


def fetch_colors(card):
    """Basic land colors a land can fetch: counted as potential sources when the deck has those basics."""
    if not card['is_land']:
        return []
    found = set()
    for match in SEARCH.finditer(card['text_en']):
        target = match.group(1)
        if target.startswith('basic land'):
            found.update(COLORS)
        elif 'land' in target or any(t in target for t in BASIC_TYPES):
            found.update(c for t, c in BASIC_TYPES.items() if t in target)
    return sorted(found, key=COLORS.index)


def main():
    rows = [json.loads(line) for line in (DATA / 'engine-CatalogCard.jsonl').open(encoding='utf-8')]
    playable = [r for r in rows if r['is_primary'] and not r['is_token'] and r['name_en']]
    by_name = defaultdict(list)
    for r in playable:
        by_name[r['name_normalized']].append(r)
    # Other non-token printings of a playable name (reprints, promos) are what search
    # often returns; accept their arena_id too, with the same name-level facts.
    extra = defaultdict(list)
    for r in rows:
        if not r['is_primary'] and not r['is_token'] and r['name_normalized'] in by_name:
            extra[r['name_normalized']].append(r)
    cards = {}
    for key, printings in by_name.items():
        owned = max(r['owned_name_total'] for r in printings)
        fronts = [r for r in printings if r['is_craft_candidate']]
        # Same choice as catalog_model.wildcard_plan: lowest-rarity craftable front.
        craft = min(fronts, key=lambda r: (r['rarity_code'], -r['owned_printing_family'], r['arena_id'])) if fronts else None
        # Arena import resolves by name; export an owned printing, else the one to craft.
        owned_printing = max(printings, key=lambda r: (r['owned'], not r['is_rebalanced'], r['arena_id']))
        shown = owned_printing if owned_printing['owned'] or not craft else craft
        export = f"{shown['name_en']} ({shown['set'].upper()}) {shown['collector_number']}"
        for r in printings + extra[key]:
            cost = r['mana_cost']
            required = ''.join(sorted({c for c in re.findall(r'\{([WUBRG])\}', cost)}, key=COLORS.index))
            sources = ''.join(s for s in r['mana_symbols_possible'] if s in COLORS + 'C') if r['is_land'] else ''
            # One tab-separated string per printing: Rhai counts nested map/array items
            # against its collection budget, strings do not. Field order is read by
            # card() in the harness scripts; keep both in sync.
            fields = [r['canonical_name'] or r['name_en'], export, owned, int(craft is not None),
                      craft['rarity'] if craft else r['rarity'], int(r['is_land'] or r['has_land_face']),
                      int(r['is_basic_land']), required, int('/' in cost), sources,
                      ''.join(fetch_colors(r)), copy_limit(r),
                      # Une carte qui parle du commandant ne fait rien, ou presque,
                      # dans un deck de 60 (Arcane Signet : aucun mana sans commandant).
                      int(bool(re.search(r"\bcommander\b", r['text_en'], re.I))),
                      int('Legendary' in r['type_en'].split(' — ')[0] and not (r['is_land'] or r['has_land_face']))]
            if any('\t' in str(f) for f in fields):
                raise SystemExit(f'tab in card fields: {fields}')
            cards[str(r['arena_id'])] = '\t'.join(map(str, fields))
    inventory = [json.loads(line) for line in (DATA / 'engine-WildcardInventory.jsonl').open(encoding='utf-8')]
    if len(inventory) != 1:
        raise SystemExit('expected exactly one wildcard inventory snapshot')
    wildcards = {k: inventory[0][k] for k in ('common', 'uncommon', 'rare', 'mythic', 'snapshot_id', 'captured_at')}
    OUT.mkdir(exist_ok=True)
    (OUT / 'cards.json').write_text(json.dumps(cards, ensure_ascii=False, separators=(',', ':')), encoding='utf-8')
    (OUT / 'wildcards.json').write_text(json.dumps(wildcards, ensure_ascii=False, indent=1) + '\n', encoding='utf-8')
    size = (OUT / 'cards.json').stat().st_size
    print(f'{len(cards)} printings, {len(by_name)} names, cards.json {size / 1e6:.1f} MB; wildcards {wildcards}')


HARNESS_SCRIPTS = ['prepare', 'accept', 'export', 'craft_plan', 'deck_size', 'sideboard_size', 'land_count',
                   'known_card', 'copy_limit', 'craftable', 'wildcard_budget', 'mana_sources', 'mana_distribution',
                   'land_bounds', 'commander_card', 'legendary_copies']
SCRIPT_IDS = {'prepare': 'deck_facts', 'accept': 'deck_accept', 'export': 'deck_export', 'craft_plan': 'deck_craft_plan'}


def register(manifest):
    """Declare submit_deck and its Rhai scripts in the manifest.

    The preparation chain regenerates backend.json from scratch; without this
    step, a refresh would drop the harness."""
    h = 'harness/'
    manifest.setdefault('scripts', {}).update({SCRIPT_IDS.get(n, n): f'{h}{n}.rhai' for n in HARNESS_SCRIPTS})
    data = {'cards': h + 'cards.json', 'policy': h + 'policy.json', 'wildcards': h + 'wildcards.json'}
    manifest['tools']['submit_deck'] = {
        'graph': h + 'submit.mmd',
        'harness': {
            'input_schema': h + 'input.json',
            'before': [{'graph': h + 'validate.mmd', 'data': data}],
            'on_accept': [{'graph': h + 'export.mmd', 'data': {'cards': h + 'cards.json'}},
                          {'graph': h + 'craft_plan.mmd', 'data': {'cards': h + 'cards.json', 'wildcards': h + 'wildcards.json'}}],
        },
    }
    return manifest


if __name__ == '__main__':
    main()
