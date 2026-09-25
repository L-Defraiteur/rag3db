"""Three additional owned-card experiments, without modifying earlier drafts."""
import json,re
from collections import Counter
from datetime import datetime,timezone
from build_fun_variants import ROOT,CS,entries,stable_id,P
from export_deck import arena_clipboard

def previous(folder):
 d=json.loads((ROOT/folder/'deck.json').read_text())
 return Counter({})+sum((Counter({c['name_en']:c['quantity']}) for c in d['main_deck']['nonlands']+d['main_deck']['lands']),Counter())
spider=previous('01-70-reanimation')
spider.subtract({'Snarling Gorehound':2,'Overlord of the Balemurk':2,'Hoarding Broodlord':1})
spider.update({'Nyx Weaver':4,"Sevinne's Reclamation":1})
crawler=previous('02-80-cimetiere-autonome')
crawler.subtract({'Balustrade Wurm':2,'Reassembling Skeleton':2,'Lingering Souls':2,'Roar of the Wurm':1,'Nethergoyf':1,'Gnaw to the Bone':1,'Victimize':1})
crawler.update({'Chitinous Crawler':4,'Nyx Weaver':2,'Chitin Gravestalker':2,'Cryptolith Rite':2})
tokens=Counter({'Doubling Season':3,'Parallel Lives':1,'Ojer Taq, Deepest Foundation':1,'Thunderbond Vanguard':2,'Prosperous Innkeeper':4,'Malevolent Rumble':4,'Chatter of the Squirrel':2,'Felidar Retreat':2,"The Huntsman's Redemption":3,'Return from the Wilds':3,"White Sun's Twilight":2,'Enduring Vitality':2,'Cryptolith Rite':2,'Stroke of Midnight':2,'Royal Treatment':2,'Moonshaker Cavalry':1,'Ghalta, Stampede Tyrant':1,"Gaea's Blessing":2,'Roar of the Wurm':1,'Llanowar Elves':2,'Temple Garden':2,'Blossoming Sands':2,'Khalni Garden':2,'Thriving Grove':2,'Forest':11,'Plains':9})
specs=[('05-70-nyx-weaver',70,spider,'Nyx Weaver — la toile de recuperation',
'Quatre Nyx Weaver remplacent une partie du support du 70 réanimation. La meule régulière nourrit les resets et les grosses cibles ; sa capacité récupère une carte de n’importe quel type, notamment Roots ou un terrain. Sevinne récupère Weaver, Roots, Vanguard ou Mycotyrant.',
'Weaver coûte trois manas puis trois manas supplémentaires pour récupérer une carte. Son activation exile la créature depuis le champ de bataille : cela ne déclenche pas Roots par soi-même. Récupérer une carte de créature du cimetière avec cette capacité peut déclencher Roots. La Weaver exilée ne sera pas récupérable par Unearth. Les jetons Mycotyrant ne comptent que la descente de ton tour ; la meule de Weaver à ton entretien est donc bien placée.',
'Comparer au 70 initial : la récupération sauve-t-elle Roots/Vanguard assez souvent pour compenser un tour trois moins explosif ?'),
('06-70-multiplicateurs-jetons',70,tokens,'Blanc vert — multiplication de jetons',
'Un deck autonome de jetons, sans moteur Mycotyrant ni réanimation. Doubling Season et Parallel Lives doublent ; Ojer Taq triple les jetons de créature. Innkeeper, Llanowar, Rumble et Return from the Wilds développent le mana. Felidar et les sorts de jetons alimentent Vanguard ou une attaque avec Moonshaker.',
'Un doubleur et Ojer Taq donnent six fois les jetons de créature ; les multiplicateurs ne créent rien seuls. Les deux Blessing conservent ta protection contre la meule, mais sont moins centrales ici. Vanguard remplace les caractéristiques des nouveaux jetons : un Spawn de Rumble perd son sacrifice pour mana ; les Mites perdent toxic. Cryptolith Rite ou Vitality rendent malgré tout les créatures capables de produire du mana, sous réserve du mal d’invocation. Ojer Taq est légendaire : on ne propose pas de le copier. Les terrains Thriving Grove doivent généralement choisir blanc.',
'Évaluer le nombre de mains qui accumulent les multiplicateurs sans producteur, et les parties où le gain de vie d’Innkeeper laisse le temps de poser le moteur.'),
('07-80-chitinous-crawler',80,crawler,'Chitinous Crawler — reserve de permanents',
'Quatre Crawler, deux Weaver et deux Chitin Gravestalker. Crawler conjure à chaque début de combat de ton tour une copie d’une carte de créature du cimetière. Sous descend 8, il donne accès aux permanents du cimetière ; Roots et Cryptolith Rite aident à payer les sorts. Gravestalker se recycle pour deux manas et devient moins cher avec les artefacts/créatures au cimetière.',
'Conjurer une carte n’est pas créer un jeton : les doubleurs de jetons ne multiplient pas ces copies. La capacité descend 8 exile un permanent comme coût ; exiler une créature déclenche Roots. Il faut encore payer le sort et respecter les restrictions de jeu, dont la limite de terrains. Le seuil de huit permanents doit être satisfait à chaque activation. Les resets peuvent désactiver ce seuil : cette tension est assumée. Les copies conjurées de créatures légendaires restent légendaires ; elles servent de réserve, pas à contourner la règle des légendes. Les sorts non permanents comme Unburial Rites ne sont pas des cibles de la capacité descend 8.',
'Noter le tour où descend 8 devient disponible, puis distinguer les activations réellement utiles des situations où le mana manque.')]
for slug,size,counts,title,plan,limits,test in specs:
 counts=+counts
 assert sum(counts.values())==size,(slug,sum(counts.values()))
 rows=entries(counts);lands=[r for r in rows if r['is_land']];other=[r for r in rows if not r['is_land']]
 d={'schema_version':'mtga.deck-draft.v1','id':stable_id('mtga:experiment:2026-09-19:'+slug),'revision':1,'name':title,'status':'draft_unplayed','created_at':datetime.now(timezone.utc).isoformat(),'source_snapshot_id':json.loads((P/'card-records.jsonl').open().readline())['snapshot_id'],'format':'Historic','legality_status':'not_validated_by_arena','main_deck':{'lands':lands,'nonlands':other},'sideboard':[],'summary':{'total_cards':size,'lands':sum(r['quantity'] for r in lands),'nonlands':sum(r['quantity'] for r in other)},'plan':plan,'tradeoffs':limits,'test_focus':test,'validation':{'owned_quantities_by_printing':True,'total_size':True,'max_four_by_name_except_basics':True}}
 path=ROOT/slug;path.mkdir(exist_ok=True)
 (path/'deck.json').write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
 txt=arena_clipboard(d)
 assert sum(int(l.split()[0]) for l in txt.splitlines() if re.match(r'^\d+ ',l))==size
 (path/'arena.txt').write_text(txt)
 (path/'README.md').write_text(f'# {title} — {size} cartes\n\n{plan}\n\n## Interactions et limites\n\n{limits}\n\n## À observer\n\n{test}\n\n{d["summary"]["lands"]} terrains. Toutes les quantités ont été vérifiées dans le snapshot de collection, sans craft. Variante non testée en partie ; aucune amélioration de classement promise. Importer arena.txt ; deck.json garde les identifiants, types, textes et capacités.\n\n## Liste\n\n'+'\n'.join(f'- {q} {n}' for n,q in counts.items())+'\n')
 print(slug,d['summary'])
index=ROOT/'README.md';s=index.read_text();heading='\n## Trois nouveaux themes\n';s=s.split(heading)[0]+heading+'\n'+'\n'.join(f'- [{title}]({slug}/README.md) — {size} cartes : [Arena]({slug}/arena.txt), [JSON]({slug}/deck.json).' for slug,size,_,title,*_ in specs)+'\n';index.write_text(s)
