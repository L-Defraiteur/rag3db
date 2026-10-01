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
from ability_facts import ability_facts
from card_rows import build_rows, card_schema


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def prepare():
    cards = [json.loads(s) for s in (P / 'cards.jsonl').open()]
    by_id = {c['arena_id']: c for c in cards}
    snapshot = json.loads((P / 'card-records.jsonl').open().readline())['snapshot_id']
    status = json.loads((P / 'status.json').read_text())
    assert status['collection_available'], 'Do not interpret an absent collection as zero ownership'
    manifest = json.loads((B / 'backend.json').read_text())
    # La même fiche que la collection (card_rows) : toutes les cartes du snapshot.
    schema = card_schema()
    write(B / 'schemas/CatalogCard.json', schema)
    records, abilities, card_links, mechanic_links = [], {}, set(), set()
    records = build_rows(cards, snapshot)
    for row, c in zip(records, cards):
        for a in c['abilities']:
            key = stable_id(f"mtga:ability:{a['ability_id']}:{a['text_id']}")
            ar = {'key': key, 'snapshot_id': snapshot, 'ability_id': a['ability_id'], 'text_id': a['text_id'],
                  'text_en': a['text_en'], 'text_fr': a['text_fr'], 'text': a['text_en'] + '\n' + a['text_fr'],
                  'glossary_ids': sorted(a['glossary_ids']), **ability_facts(a['text_en'])}
            assert key not in abilities or abilities[key] == ar, key
            abilities[key] = ar
            card_links.add((row['key'], key))
            mechanic_links.update((key, mid) for mid in a['glossary_ids'])
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
        config = json.loads(json.dumps(manifest['entities'][source]['config'])) if source else {'fields': {'snapshot_id': {'type': 'string', 'isContent': True}}, 'hashsafe': ['key'], 'signals': [], 'contentKind': 'record'}
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
    # Les faits figés du harnais submit_deck, puis sa déclaration : la chaîne
    # régénère backend.json, le harnais doit y revenir à chaque fois.
    from prepare_deck_harness import main as prepare_harness, register as register_harness
    prepare_harness()
    register_harness(manifest)
    write(B / 'backend.json', manifest)
    report = {'snapshot_id': snapshot, 'catalog_records': len(records), 'unique_abilities': len(abilities),
              'primary_nontoken': sum(r['is_primary'] and not r['is_token'] for r in records),
              'craft_preferred_names': sum(r['craft_preferred'] for r in records), 'relations': {k: len(v) for k, v in links.items()},
              'wildcard_inventory': inventory, 'status': 'prepared_not_ingested'}
    write(P / 'engine-catalog-preparation.json', report)
    from prepare_engine_render import prepare as prepare_render
    prepare_render()
    print(json.dumps(report))


if __name__ == '__main__':
    prepare()
