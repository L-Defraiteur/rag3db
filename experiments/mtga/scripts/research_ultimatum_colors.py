"""Offline reproducible thematic searches over the owned four-color card pool."""
import json
from pathlib import Path
from collections import defaultdict
P=Path(__file__).resolve().parents[1]/'data';OUT=P/'deck-drafts/experiments-2026-09-19/09-80-ultimatum-desastreux/research';OUT.mkdir(exist_ok=True)
allowed={'White','Black','Red','Green'};groups=defaultdict(list)
for c in map(json.loads,(P/'cards.jsonl').open()):
 if c['is_primary'] and c['owned'] and set(c['color_identity'])<=allowed and not c['is_token']:groups[c['name_en']].append(c)
queries={
 '01-mana-treasures':{'terms':['treasure','add one mana of any color','add three mana of any one color','convoke'],'scope':'all_four_colors'},
 '02-graveyard-selection':{'terms':['flashback','discard','return target card from your graveyard','instant or sorcery card from your graveyard'],'scope':'all_four_colors'},
 '03-token-damage-sacrifice':{'terms':['whenever a creature you control enters','whenever another creature you control enters','whenever you create or sacrifice a token','whenever you sacrifice','whenever a creature you control dies'],'scope':'all_four_colors'},
 '04-haste-untap':{'terms':['haste','untap all','untap target'],'scope':'all_four_colors'},
 '05-multipliers':{'terms':['twice that many','three times that many','additional time'],'scope':'all_four_colors'},
 '06-red-contributions':{'terms':['treasure','flashback','haste','whenever a creature you control enters','whenever you sacrifice','return target instant or sorcery'],'scope':'includes_red'}
}
summary={}
for key,spec in queries.items():
 results=[]
 for name,prints in sorted(groups.items()):
  eligible=[c for c in prints if spec['scope']!='includes_red' or 'Red' in c['color_identity']]
  matches=[(c,[t for t in spec['terms'] if t in c['text_en'].casefold()]) for c in eligible]
  matches=[(c,terms) for c,terms in matches if terms]
  if not matches:continue
  results.append({'name_en':name,'owned_total':sum(c['owned'] for c in prints),'printings':[{'arena_id':c['arena_id'],'owned':c['owned'],'set':c['set'],'collector_number':c['collector_number']} for c in prints],'evidence':[{'arena_id':c['arena_id'],'name_fr':c['name_fr'],'mana_cost':c['mana_cost'],'color_identity':c['color_identity'],'type_en':c['type_en'],'text_en':c['text_en'],'matched_terms':terms} for c,terms in matches]})
 obj={'query':spec,'matching':'casefold substring OR; lexical candidate discovery, not proof of synergy','filters':{'owned_gt':0,'is_primary':True,'is_token':False,'color_identity_subset':sorted(allowed)},'grouping':'exact English name; printing IDs retained; not a universal Oracle identity','source_snapshot_id':json.loads((P/'card-records.jsonl').open().readline())['snapshot_id'],'count':len(results),'results':results}
 (OUT/(key+'.json')).write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n');summary[key]=len(results)
(OUT/'index.json').write_text(json.dumps({'owned_distinct_names_in_color_pool':len(groups),'searches':summary,'execution':'local snapshot Python; not rag3weaver DAG'},indent=2)+'\n')
print(json.dumps(summary));print('pool',len(groups))
