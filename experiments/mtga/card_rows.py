"""The full record of an Arena card, the same for the collection and the catalog.

The two preparation scripts used to build their own records, and the
collection got fewer fields than the catalog: no subtypes, no rarity, no
name-level ownership. An agent could filter `subtypes has_any ["Dragon"]` on
the catalog and not on its own cards (27 September 2026). A card is built
once, here; `OwnedCard` is the catalog restricted to owned printings, with the
same payload.
"""
import re

from ability_facts import card_facts
from catalog_model import RARITIES, ownership_model, printed_mana_value
from identity import card_id
from vocabulary import declare

BASE_TYPES = {'Plains': 'W', 'Island': 'U', 'Swamp': 'B', 'Mountain': 'R', 'Forest': 'G'}
SUPERTYPES = ['Legendary', 'Basic', 'Snow', 'World', 'Ongoing']
MANA_CONDITION_WORDS = ['only', 'if ', 'chosen', 'choose', 'among', 'spend', 'could produce', 'that color', 'colors of']


def card_schema():
    """JSON schema of a card record, with declared vocabularies and examples."""
    props = {k: {'type': 'string'} for k in [
        'key', 'snapshot_id', 'name_en', 'name_fr', 'type_en', 'text_en', 'text_fr', 'text', 'mana_cost', 'set',
        'collector_number', 'mana_conditions', 'canonical_name', 'name_normalized', 'rarity', 'craftability_status']}
    props.update({k: {'type': 'integer'} for k in [
        'arena_id', 'owned', 'rarity_code', 'printing_family_id', 'owned_printing_family', 'owned_name_total',
        'missing_for_one', 'missing_for_playset', 'printed_mana_value']})
    props.update({k: {'type': 'boolean'} for k in [
        'is_land', 'has_land_face', 'is_primary', 'is_token', 'mana_has_conditions', 'is_basic_land',
        'is_rebalanced', 'is_digital_only', 'is_craft_candidate', 'craft_preferred', 'has_owned_copy']})
    props.update({k: {'type': 'array', 'items': {'type': 'string'}} for k in [
        'colors', 'color_identity', 'card_types', 'subtypes', 'land_types', 'mana_symbols_possible',
        'mana_symbols_explicit', 'linked_card_ids', 'ability_kinds', 'ability_costs', 'triggers', 'effects',
        'ability_restrictions']})
    props['abilities'] = {'type': 'array', 'items': {
        'type': 'object', 'additionalProperties': False, 'required': ['ability_id', 'text_en', 'text_fr'],
        'properties': {'ability_id': {'type': 'integer'}, 'text_en': {'type': 'string'}, 'text_fr': {'type': 'string'}}}}
    props['rarity']['enum'] = list(RARITIES.values())
    declare(props)
    return {'type': 'object', 'additionalProperties': False, 'properties': props, 'required': list(props)}


def build_rows(cards, snapshot):
    """Every card of the snapshot as a full record; `craft_preferred` needs them all."""
    by_id = {c['arena_id']: c for c in cards}
    ownership = ownership_model(cards)
    fields = set(card_schema()['properties'])
    rows = []
    for c in cards:
        types = c['type_en'].split(' — ')[0].split()
        subtypes = c['type_en'].split(' — ')[1].split() if ' — ' in c['type_en'] else []
        is_land = 'Land' in types
        mana_lines = [s for s in c['text_en'].splitlines() if re.search(r'\badd\b', s, re.I)] if is_land else []
        explicit = {BASE_TYPES[t] for t in subtypes if is_land and t in BASE_TYPES}
        for line in mana_lines:
            clause = re.split(r'\badd\b', line, maxsplit=1, flags=re.I)[1].split('.')[0]
            explicit.update(re.findall(r'\{([WUBRGC])\}', clause))
        possible = set(explicit)
        if any('any color' in s.lower() or 'chosen color' in s.lower() for s in mana_lines):
            possible.update('WUBRG')
        row = {k: c[k] for k in ['arena_id', 'owned', 'name_en', 'name_fr', 'type_en', 'text_en', 'text_fr',
                                 'mana_cost', 'set', 'collector_number', 'colors', 'color_identity',
                                 'is_primary', 'is_token', 'is_rebalanced', 'is_digital_only', 'rarity_code']}
        own = ownership[c['arena_id']]
        basic = is_land and 'Basic' in types
        candidate = c['is_primary'] and not c['is_token'] and not c['is_rebalanced'] and c['rarity_code'] in (2, 3, 4, 5)
        row.update(own)
        row.update(key=card_id(c['arena_id']), snapshot_id=snapshot,
                   text=c['name_en'] + ' / ' + c['name_fr'] + '\n' + c['type_en'] + '\n' + c['text_en'] + '\n' + c['text_fr'],
                   is_land=is_land, is_basic_land=basic,
                   has_land_face=is_land or any('Land' in by_id.get(i, {}).get('type_en', '').split(' — ')[0].split() for i in c['linked_faces']),
                   card_types=[t for t in types if t not in SUPERTYPES],
                   subtypes=subtypes, land_types=subtypes if is_land else [],
                   mana_symbols_possible=sorted(possible), mana_symbols_explicit=sorted(explicit),
                   mana_has_conditions=any(any(x in s.lower() for x in MANA_CONDITION_WORDS) for s in mana_lines)
                   or (is_land and 'choose a color' in c['text_en'].lower()),
                   mana_conditions=c['text_en'] if is_land else '', linked_card_ids=[card_id(i) for i in c['linked_faces']],
                   abilities=[{k: a[k] for k in ['ability_id', 'text_en', 'text_fr']} for a in c['abilities']],
                   rarity=RARITIES[c['rarity_code']], is_craft_candidate=candidate, craft_preferred=False,
                   craftability_status='candidate_requires_arena_validation' if candidate else 'not_a_direct_craft_candidate',
                   has_owned_copy=own['owned_name_total'] > 0,
                   missing_for_one=0 if basic and own['owned_name_total'] else max(0, 1 - own['owned_name_total']),
                   missing_for_playset=0 if basic and own['owned_name_total'] else max(0, 4 - own['owned_name_total']),
                   printed_mana_value=printed_mana_value(c['mana_cost']), **card_facts(c['abilities']))
        assert set(row) == fields, set(row) ^ fields
        rows.append(row)
    preferred = {}
    for row in rows:
        if row['is_craft_candidate']:
            rank = (row['rarity_code'], -row['owned_printing_family'], row['arena_id'])
            name = row['name_normalized']
            if name not in preferred or rank < preferred[name][0]:
                preferred[name] = (rank, row)
    for _, row in preferred.values():
        row['craft_preferred'] = True
    return rows
