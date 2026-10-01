"""Build and validate structured ingestion records, separate from embedding text."""
import json
from contract import (
    SCHEMA_VERSION, stable_id, card_id, CardPayload, CardRecord, CollectionPayload,
    CollectionRecord, DeckPayload, DeckRecord, MechanicPayload, MechanicRecord,
)

KINDS = {'cards':'card','collection':'collection_entry','decks':'deck','mechanics':'mechanic','boosters':'booster_holding'}

def make_record(dataset, data, status, snapshot_id):
    source = status['source_id']
    if dataset == 'cards':
        logical_id = f"mtga:card:{data['arena_id']}"
        payload = {k:data[k] for k in CardPayload.model_fields if k in data}
        payload.update(set_code=data['set'],linked_card_ids=[card_id(i) for i in data['linked_faces']],
                       rebalanced_card_id=card_id(data['rebalanced_arena_id']) if data['rebalanced_arena_id'] else None,
                       power_numeric=int(data['power']) if data['power'].lstrip('-').isdigit() else None,
                       toughness_numeric=int(data['toughness']) if data['toughness'].lstrip('-').isdigit() else None)
        payload = CardPayload.model_validate(payload)
        text = '\n'.join([data['name_fr'],data['name_en'],data['mana_cost'],data['type_fr'],data['text_fr'],data['type_en'],data['text_en']])
        model, uri = CardRecord, f"mtga://card/{data['arena_id']}"
    elif dataset == 'collection':
        logical_id = f"mtga:{source}:collection:{data['arena_id']}"
        payload = CollectionPayload(source_id=source,card_id=card_id(data['arena_id']),arena_id=data['arena_id'],
                                    quantity=data['owned'],captured_at=status['collection_captured_at'])
        text = f"{data['name_fr']} / {data['name_en']}"
        model, uri = CollectionRecord, f"mtga://source/{source}/collection/{data['arena_id']}"
    elif dataset == 'decks':
        logical_id = f"mtga:{source}:deck:{data['id']}"
        entries = [dict(card_id=card_id(c['cardId']),arena_id=c['cardId'],quantity=c['quantity'],pile=pile)
                   for pile,cards in data['piles'].items() for c in cards]
        payload = DeckPayload(source_id=source,arena_deck_id=data['id'],name=data['name'],format=data['format'],
                              attributes=data['attributes'],entries=entries,
                              main_deck_count=sum(e['quantity'] for e in entries if e['pile']=='MainDeck'),
                              sideboard_count=sum(e['quantity'] for e in entries if e['pile']=='Sideboard'))
        text = '\n'.join([data['name'],data['format'] or '', *[
            f"{pile}: {c['quantity']} {c['name_en'] or c['cardId']}" for pile,cards in data['piles'].items() for c in cards]])
        model, uri = DeckRecord, f"mtga://source/{source}/deck/{data['id']}"
    elif dataset == 'mechanics':
        logical_id = 'mtga:mechanic:'+data['key']
        payload = MechanicPayload.model_validate(data)
        text = '\n'.join([data['name_en'],data['name_fr'],data['definition_en'] or '',data['definition_fr'] or ''])
        model, uri = MechanicRecord, 'mtga://mechanic/'+data['key']
    else:
        raise ValueError('No captured booster inventory')
    return model(id=stable_id(logical_id),logical_id=logical_id,snapshot_id=snapshot_id,
                 source_uri=uri,text=text,payload=payload)

def source_manifest(status):
    return {
        'cards':dict(kind='card',available=True,count=status['catalog_entries'],embedding_recommended=True),
        'collection':dict(kind='collection_entry',available=status['collection_available'],count=status['owned_entries'],
                         embedding_recommended=False, joins={'payload.card_id':'cards.id'}),
        'decks':dict(kind='deck',available=True,count=status['decks'],embedding_recommended=True,
                     joins={'payload.entries[].card_id':'cards.id'}),
        'mechanics':dict(kind='mechanic',available=bool(status.get('mechanics')),count=status.get('mechanics',0),embedding_recommended=True),
        'boosters':dict(kind='booster_holding',available=False,count=None,embedding_recommended=False,
                       reason='Aucune quantité de boosters non ouverts dans la capture. Schéma réservé, aucun record inventé.'),
    }
