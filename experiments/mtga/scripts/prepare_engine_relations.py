"""Extend the complete owned snapshot with ability/glossary/deck entities and links.
No thematic selection. Deck entries for unowned printings remain explicitly unlinked.
"""
import json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data';B=ROOT/'experiments/mtga/backend';C=ROOT/'extension/rag3weaver'
sys.path.insert(0,str(P.parent))
from identity import stable_id,card_id

def lines(name):return [json.loads(l) for l in (P/name).open()]
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
manifest=json.loads((B/'backend.json').read_text());cards=lines('cards.jsonl');owned=[c for c in cards if c['owned']>0];owned_ids={c['arena_id'] for c in owned};snapshot=lines('engine-collection.jsonl')[0]['snapshot_id']
entities={};links={};abilities={};mechanics=[];decks=[];entries=[]
for record in lines('mechanic-records.jsonl'):
 m=record['payload'];mechanics.append({'key':record['id'],'snapshot_id':snapshot,'name':m['key'],'text':record['text'],'definition_status':m['definition_status']})
mechanic_keys={r['key'] for r in mechanics}
def link(rel,a,b):links.setdefault(rel,set()).add((a,b))
for card in owned:
 for a in card['abilities']:
  key=stable_id(f"mtga:ability:{a['ability_id']}:{a['text_id']}")
  row={'key':key,'snapshot_id':snapshot,'ability_id':a['ability_id'],'text_id':a['text_id'],'text_en':a['text_en'],'text_fr':a['text_fr'],'text':a['text_en']+'\n'+a['text_fr'],'glossary_ids':sorted(a['glossary_ids'])}
  if key in abilities:assert abilities[key]==row,('conflicting ability identity',key)
  abilities[key]=row;link('CardAbility',card_id(card['arena_id']),key)
  for mid in a['glossary_ids']:
   assert mid in mechanic_keys,('unknown glossary reference',mid)
   link('AbilityMechanic',key,mid)
for record in lines('deck-records.jsonl'):
 d=record['payload'];decks.append({'key':record['id'],'snapshot_id':record['snapshot_id'],'arena_deck_id':d['arena_deck_id'],'name':d['name'],'format':d['format'],'text':record['text'],'main_deck_count':d['main_deck_count'],'sideboard_count':d['sideboard_count']})
 for e in d['entries']:
  key=stable_id(f"mtga:deck-entry:{record['id']}:{e['pile']}:{e['card_id']}")
  row={'key':key,'snapshot_id':snapshot,'deck_key':record['id'],'arena_deck_id':d['arena_deck_id'],'card_key':e['card_id'],'arena_id':e['arena_id'],'quantity':e['quantity'],'pile':e['pile'],'text':d['name']+' / '+e['pile']+' / '+str(e['arena_id']),'in_owned_collection':e['arena_id'] in owned_ids}
  entries.append(row);link('DeckEntries',record['id'],key)
  if row['in_owned_collection']:link('EntryOwnedCard',key,e['card_id'])
aggregated={}
for row in entries:
 if row['key'] in aggregated:aggregated[row['key']]['quantity']+=row['quantity']
 else:aggregated[row['key']]=row.copy()
entries=list(aggregated.values())
entities={'Ability':list(abilities.values()),'Mechanic':mechanics,'Deck':decks,'DeckEntry':entries}
for entity,rows in entities.items():
 props={}
 for k,v in rows[0].items():
  props[k]={'type':'boolean' if isinstance(v,bool) else 'integer' if isinstance(v,int) else 'array' if isinstance(v,list) else 'string'}
  if isinstance(v,list):props[k]['items']={'type':'string'}
 schema={'type':'object','additionalProperties':False,'properties':props,'required':list(props)}
 write(B/f'schemas/{entity}.json',schema)
 (P/f'engine-{entity}.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in rows))
 config={'fields':{},'hashsafe':['key'],'returnFields':list(props),'signals':[]}
 if 'text' in props:config['fields']['text']={'type':'string','isContent':True};config['signals']=['bm25','vector'] if entity!='DeckEntry' else []
 if 'name' in props:config['fields']['name']={'type':'string','isTitle':True}
 manifest['entities'][entity]={'schema':f'schemas/{entity}.json','config':config}
 manifest['tools'][f'ingest_{entity.lower()}']={'graph':str(C/'templates/tools/ingest_snapshot.mmd'),'bindings':{'entity':entity}}
 manifest['tools'][f'select_{entity.lower()}']={'graph':str(C/'templates/tools/select_structured.mmd'),'bindings':{'entity':entity}}
 if 'text' in props:manifest['tools'][f'search_{entity.lower()}']={'graph':str(C/'templates/tools/search_structured.mmd'),'bindings':{'target':entity},'metadata':[{'node':'render','port':'meta'}]}
relations={'CardAbility':('OwnedCard','Ability'),'AbilityMechanic':('Ability','Mechanic'),'DeckEntries':('Deck','DeckEntry'),'EntryOwnedCard':('DeckEntry','OwnedCard')}
for name,(a,b) in relations.items():
 manifest.setdefault('relations',{})[name]={'from':a,'to':b}
 manifest['tools'][f'link_{name.lower()}']={'graph':str(C/'templates/tools/link_snapshot.mmd'),'bindings':{'relation':name}}
(P/'engine-links.json').write_text(json.dumps({k:[{'from':{'key':a},'to':{'key':b}} for a,b in sorted(v)] for k,v in links.items()}))
manifest['tools']['search_deck_mechanics']={'graph':str(C/'templates/tools/search_related_scoped.mmd'),'bindings':{'mechanic_entity':'Mechanic','ability_entity':'Ability','card_entity':'OwnedCard','scope_entity':'DeckEntry','ability_mechanic_relation':'AbilityMechanic','card_ability_relation':'CardAbility','scope_card_relation':'EntryOwnedCard'}}
manifest['tools']['search_ability_cards']={'graph':str(C/'templates/tools/search_dense_related.mmd'),'bindings':{'target':'Ability','relation':'CardAbility','direction':'Incoming'}}
write(B/'backend.json',manifest)
report={'snapshot_id':snapshot,'entities':{e:len(r) for e,r in entities.items()},'relations':{e:len(r) for e,r in links.items()},'unowned_deck_entries':sum(not r['in_owned_collection'] for r in entries),'scope':'all owned printing abilities; all glossary entries; all captured decks and entries; only owned cards have entry links'}
write(P/'engine-relations-preparation.json',report);print(json.dumps(report))
