"""Add the full local Arena catalog to the generated backend, preserving OwnedCard."""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
P = ROOT / 'experiments/mtga/data'
B = P.parent / 'backend'
C = ROOT / 'extension/rag3weaver'
sys.path.insert(0, str(P.parent))
from identity import card_id, stable_id
from catalog_model import RARITIES, ownership_model, printed_mana_value


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def prepare():
    cards = [json.loads(s) for s in (P / 'cards.jsonl').open()]
    by_id = {c['arena_id']: c for c in cards}
    snapshot = json.loads((P / 'card-records.jsonl').open().readline())['snapshot_id']
    status = json.loads((P / 'status.json').read_text())
    assert status['collection_available'], 'Do not interpret an absent collection as zero ownership'
    ownership = ownership_model(cards)
    manifest = json.loads((B / 'backend.json').read_text())
    schema = json.loads((B / 'schemas/card.json').read_text())
    fields = schema['properties']
    for name in ['canonical_name', 'name_normalized', 'rarity', 'craftability_status']:
        fields[name] = {'type': 'string'}
    fields['rarity']['enum'] = list(RARITIES.values())
    for name in ['rarity_code', 'printing_family_id', 'owned_printing_family', 'owned_name_total',
                 'missing_for_one', 'missing_for_playset', 'printed_mana_value']:
        fields[name] = {'type': 'integer'}
    for name in ['is_basic_land', 'is_rebalanced', 'is_digital_only', 'is_craft_candidate', 'craft_preferred', 'has_owned_copy']:
        fields[name] = {'type': 'boolean'}
    fields['subtypes'] = {'type': 'array', 'items': {'type': 'string'}}
    schema['required'] = list(fields)
    write(B / 'schemas/CatalogCard.json', schema)
    records, abilities, card_links, mechanic_links = [], {}, set(), set()
    base_types = {'Plains': 'W', 'Island': 'U', 'Swamp': 'B', 'Mountain': 'R', 'Forest': 'G'}
    for c in cards:
        types = c['type_en'].split(' — ')[0].split()
        subtypes = c['type_en'].split(' — ')[1].split() if ' — ' in c['type_en'] else []
        is_land = 'Land' in types
        mana_lines = [s for s in c['text_en'].splitlines() if re.search(r'\badd\b', s, re.I)] if is_land else []
        explicit = {base_types[t] for t in subtypes if is_land and t in base_types}
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
                   card_types=[t for t in types if t not in ['Legendary', 'Basic', 'Snow', 'World', 'Ongoing']],
                   subtypes=subtypes, land_types=subtypes if is_land else [],
                   mana_symbols_possible=sorted(possible), mana_symbols_explicit=sorted(explicit),
                   mana_has_conditions=any(any(x in s.lower() for x in ['only', 'if ', 'chosen', 'choose', 'among', 'spend', 'could produce', 'that color', 'colors of']) for s in mana_lines) or (is_land and 'choose a color' in c['text_en'].lower()),
                   mana_conditions=c['text_en'] if is_land else '', linked_card_ids=[card_id(i) for i in c['linked_faces']],
                   abilities=[{k: a[k] for k in ['ability_id', 'text_en', 'text_fr']} for a in c['abilities']],
                   rarity=RARITIES[c['rarity_code']], is_craft_candidate=candidate, craft_preferred=False,
                   craftability_status='candidate_requires_arena_validation' if candidate else 'not_a_direct_craft_candidate',
                   has_owned_copy=own['owned_name_total'] > 0,
                   missing_for_one=0 if basic and own['owned_name_total'] else max(0, 1 - own['owned_name_total']),
                   missing_for_playset=0 if basic and own['owned_name_total'] else max(0, 4 - own['owned_name_total']),
                   printed_mana_value=printed_mana_value(c['mana_cost']))
        assert set(row) == set(fields), set(row) ^ set(fields)
        records.append(row)
        for a in c['abilities']:
            key = stable_id(f"mtga:ability:{a['ability_id']}:{a['text_id']}")
            ar = {'key': key, 'snapshot_id': snapshot, 'ability_id': a['ability_id'], 'text_id': a['text_id'],
                  'text_en': a['text_en'], 'text_fr': a['text_fr'], 'text': a['text_en'] + '\n' + a['text_fr'],
                  'glossary_ids': sorted(a['glossary_ids'])}
            assert key not in abilities or abilities[key] == ar, key
            abilities[key] = ar
            card_links.add((row['key'], key))
            mechanic_links.update((key, mid) for mid in a['glossary_ids'])
    preferred = {}
    for row in records:
        if row['is_craft_candidate']:
            rank = (row['rarity_code'], -row['owned_printing_family'], row['arena_id'])
            name = row['name_normalized']
            if name not in preferred or rank < preferred[name][0]:
                preferred[name] = (rank, row)
    for _, row in preferred.values():
        row['craft_preferred'] = True
    inv = json.loads((P / 'inventory.json').read_text())
    inventory = {'key': stable_id('mtga:wildcard-inventory'), 'snapshot_id': snapshot,
                 'captured_at': status['collection_captured_at'],
                 **{r: inv[k] for r, k in [('common', 'WildCardCommons'), ('uncommon', 'WildCardUnCommons'),
                                          ('rare', 'WildCardRares'), ('mythic', 'WildCardMythics')]}}
    inv_schema = {'type': 'object', 'additionalProperties': False,
                  'properties': {k: {'type': 'integer' if type(v) is int else 'string'} for k, v in inventory.items()},
                  'required': list(inventory)}
    write(B / 'schemas/WildcardInventory.json', inv_schema)
    (B / 'schemas/CatalogAbility.json').write_text((B / 'schemas/Ability.json').read_text())
    for entity, data, source in [('CatalogCard', records, 'OwnedCard'), ('CatalogAbility', list(abilities.values()), 'Ability'),
                                 ('WildcardInventory', [inventory], None)]:
        config = json.loads(json.dumps(manifest['entities'][source]['config'])) if source else {'fields': {'snapshot_id': {'type': 'string', 'isContent': True}}, 'hashsafe': ['key'], 'signals': []}
        config['returnFields'] = list(data[0])
        manifest['entities'][entity] = {'schema': f'schemas/{entity}.json', 'config': config}
        (P / f'engine-{entity}.jsonl').write_text(''.join(json.dumps(r, ensure_ascii=False) + '\n' for r in data))
        for action in ['ingest', 'select', *(['search'] if source else [])]:
            tool = f'{action}_catalog' if entity == 'CatalogCard' else f'{action}_{entity.lower()}'
            graph = {'ingest': 'ingest_snapshot', 'select': 'select_structured', 'search': 'search_structured'}[action]
            manifest['tools'][tool] = {'graph': str(C / f'templates/tools/{graph}.mmd'),
                                       'bindings': {'target' if action == 'search' else 'entity': entity}}
            if action == 'search':
                manifest['tools'][tool]['metadata'] = [{'node': 'render', 'port': 'meta'}]
    mechanic_keys = {json.loads(line)['key'] for line in (P / 'engine-Mechanic.jsonl').open()}
    assert {mid for _, mid in mechanic_links} <= mechanic_keys, 'Missing mechanic endpoints'
    links = {'CatalogCardAbility': card_links, 'CatalogAbilityMechanic': mechanic_links}
    for rel, source, target in [('CatalogCardAbility', 'CatalogCard', 'CatalogAbility'),
                                ('CatalogAbilityMechanic', 'CatalogAbility', 'Mechanic')]:
        manifest['relations'][rel] = {'from': source, 'to': target}
        manifest['tools']['link_' + rel.lower()] = {'graph': str(C / 'templates/tools/link_snapshot.mmd'), 'bindings': {'relation': rel}}
    manifest['tools']['search_catalog_ability_cards'] = {
        'graph': str(C / 'templates/tools/search_dense_related.mmd'),
        'bindings': {'target': 'CatalogAbility', 'relation': 'CatalogCardAbility', 'direction': 'Incoming'}}
    write(P / 'engine-catalog-links.json', {k: [{'from': {'key': a}, 'to': {'key': b}} for a, b in sorted(v)] for k, v in links.items()})
    write(B / 'backend.json', manifest)
    report = {'snapshot_id': snapshot, 'catalog_records': len(records), 'unique_abilities': len(abilities),
              'primary_nontoken': sum(r['is_primary'] and not r['is_token'] for r in records),
              'craft_preferred_names': len(preferred), 'relations': {k: len(v) for k, v in links.items()},
              'wildcard_inventory': inventory, 'status': 'prepared_not_ingested'}
    write(P / 'engine-catalog-preparation.json', report)
    from prepare_engine_render import prepare as prepare_render
    prepare_render()
    print(json.dumps(report))


if __name__ == '__main__':
    prepare()
