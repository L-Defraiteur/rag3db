"""Build the selected deck exclusively from saved real MCP results; no local catalog search."""
import json,sys,math
from collections import Counter
from datetime import datetime,timezone
from pathlib import Path
P=Path(__file__).resolve().parents[1]/'data';R=P/'deck-research-2026-09-20';OUT=P/'deck-drafts/experiments-2026-09-20/01-eldrazi-faille-d-ugin'
sys.path.insert(0,str(Path(__file__).resolve().parent))
from export_deck import arena_clipboard
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from identity import stable_id
pool={}
for f in ['eldrazi_final_pool.json','last_cards.json']:
 for hit in json.loads((R/f).read_text())['result']:pool[hit['data']['arena_id']]=hit['data']
spells=Counter({'Llanowar Elves':4,'Utopia Sprawl':4,'Malevolent Rumble':4,"Kozilek's Command":4,'Sowing Mycospawn':4,'Glaring Fleshraker':3,'Thought-Knot Seer':2,'Nulldrifter':3,'Sire of Seven Deaths':2,'Ugin, the Spirit Dragon':2,'Ugin, Eye of the Storms':1,'Breaker of Creation':2,'Ulamog, the Ceaseless Hunger':1})
lands=Counter({'Forest':13,'Island':1,"Ugin's Labyrinth":4,'Cavern of Souls':4,'Sanctum of Ugin':1,'Blast Zone':1})
assert sum(spells.values())==36 and sum(lands.values())==24
basics={'Forest','Island','Mountain','Plains','Swamp'}
def entries(counts):
 result=[]
 for name,quantity in counts.items():
  candidates=sorted((c for c in pool.values() if c['name_en']==name and c['owned']>0 and c['is_primary']),key=lambda c:(-c['owned'],c['arena_id']))
  assert candidates,(name,'not in engine results')
  assert name in basics or quantity<=4
  if name in basics:
   # Arena basic lands are unlimited once this printing is unlocked.
   c=candidates[0];result.append({**c,'quantity':quantity,'card_id':c['key'],'owned_this_printing':c['owned'],'basic_land_unlimited':True});continue
  assert sum(c['owned'] for c in candidates)>=quantity,(name,quantity)
  for c in candidates:
   take=min(quantity,c['owned']);quantity-=take
   if take:result.append({**c,'quantity':take,'card_id':c['key'],'owned_this_printing':c['owned']})
   if not quantity:break
 return result
ls,ns=entries(lands),entries(spells)
for c in ls:assert c['is_land']
for c in ns:assert not c['is_land']
def mv(c):
 import re
 return sum(int(x) if x.isdigit() else 0 if x=='X' else 1 for x in re.findall(r'\{([^}]+)\}',c['mana_cost']))
fuel=sum(c['quantity'] for c in ns if not c['colors'] and mv(c)>=7)
assert fuel==11
prob_lab=1-math.comb(56,7)/math.comb(60,7)
prob_both=1-math.comb(56,7)/math.comb(60,7)-math.comb(60-fuel,7)/math.comb(60,7)+math.comb(56-fuel,7)/math.comb(60,7)
stats={'total_cards':60,'lands':24,'nonlands':36,'labyrinth_imprint_targets':fuel,'opening_seven_forest_probability':1-math.comb(47,7)/math.comb(60,7),'opening_seven_labyrinth_and_fuel_probability':prob_both,'fuel_given_opening_labyrinth_probability':prob_both/prob_lab,'model':'uniform random seven cards, before mulligans; not Arena BO1 hand smoothing or win-rate simulation','unconditional_colorless_land_sources':10,'unconditional_blue_land_sources':1,'eldrazi_blue_sources_from_cavern':4,'sprawl_blue_sources_if_chosen':4,'first_turn_green_basic_sources':13}
substitutions=[
 {'unavailable':'Eldrazi Temple','reason':'banned in Historic','chosen':["Ugin's Labyrinth",'Utopia Sprawl','Llanowar Elves'],'tradeoff':'Labyrinth requires a colorless card with mana value >=7. Green accelerators need Forest/green mana and do not produce {C}.'},
 {'unavailable':'Devourer of Destiny','reason':'absent from verified owned results','chosen':['Nulldrifter','Sire of Seven Deaths','Breaker of Creation'],'tradeoff':'Preserves imprint fuel, card draw, lifegain and threats, but does not reproduce opening-hand selection or the Devourer exile trigger.'},
 {'unavailable':'The One Ring','reason':'absent from verified owned results; Historic ban announced effective 2026-09-22','chosen':['Nulldrifter',"Kozilek's Command"],'tradeoff':'Draw and selection distributed over spells; no protection-for-a-turn replacement and no cumulative Ring draw.'},
 {'unavailable':'Ugin, Eye of the Storms playset','reason':'only one owned','chosen':['Ugin, Eye of the Storms','Ugin, the Spirit Dragon'],'tradeoff':'One newer Ugin plus two older sweepers, not equivalent triggered engines; Spirit Dragon costs eight.'},
 {'unavailable':'Thought-Knot Seer playset','reason':'only two owned','chosen':['Thought-Knot Seer','Glaring Fleshraker'],'tradeoff':'Two actual hand-disruption effects; Fleshraker supplies mana and damage, not discard.'}
]
snapshot=next(iter(pool.values()))['snapshot_id'];assert all(c['snapshot_id']==snapshot for c in pool.values())
draft={'schema_version':'mtga.deck-draft.v1','id':stable_id('mtga:experiment:2026-09-20:eldrazi-faille-d-ugin'),'revision':1,'name':"Faille d'Ugin — Eldrazi sans craft",'status':'draft_unplayed','created_at':datetime.now(timezone.utc).isoformat(),'source_snapshot_id':snapshot,'source_deck_id':None,'format':'Historic','queue':'Best-of-One','legality_status':'published Historic banned list reviewed; Arena import not yet tested','main_deck':{'lands':ls,'nonlands':ns},'sideboard':[],'summary':stats,'substitutions':substitutions,'research':{'transport':'official MCP SDK -> generated backend -> rag3db + lucivy + local bge-m3','evidence_directory':str(R),'selection':'new deck, not derived from an existing deck','sources':['https://magic.wizards.com/en/banned-restricted-list','https://magic.wizards.com/en/news/mtg-arena/state-of-the-formats-2026']},'validation':{'60_cards':True,'max_four_nonbasic_by_name':True,'nonbasic_quantities_owned_per_printing':True,'basic_land_printings_unlocked':True,'no_crafting_required_in_snapshot':True,'no_match_results_or_winrate_claim':True}}
OUT.mkdir(parents=True,exist_ok=True)
(OUT/'deck.json').write_text(json.dumps(draft,ensure_ascii=False,indent=2)+'\n')
text=arena_clipboard(draft);assert sum(int(l.split()[0]) for l in text.splitlines() if l and l[0].isdigit())==60
(OUT/'arena.txt').write_text(text)
(OUT/'validation.json').write_text(json.dumps({'summary':stats,'validation':draft['validation']},indent=2)+'\n')
print(OUT);print(json.dumps(stats,indent=2));print(text)
