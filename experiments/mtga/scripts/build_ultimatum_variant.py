"""Owned-card Ruinous Ultimatum / token mana experiment."""
import json,re
from collections import Counter
from datetime import datetime,timezone
from build_fun_variants import ROOT,P,entries,stable_id
from export_deck import arena_clipboard
spells=Counter({'Insidious Roots':4,"Stitcher's Supplier":4,'Rubblebelt Maverick':4,'Snarling Gorehound':2,'Thunderbond Vanguard':2,'The Mycotyrant':2,'Enduring Vitality':3,'Cryptolith Rite':2,'Ruinous Ultimatum':3,'Hoarding Broodlord':3,'Unburial Rites':3,'Victimize':2,'Malevolent Rumble':3,'Nyx Weaver':2,"Sevinne's Reclamation":1,"Dryad's Revival":1,"Gaea's Blessing":2,'Mesmeric Orb':2,'Big Score':2,"Collector's Vault":1})
lands=Counter({'Temple Garden':2,'Godless Shrine':1,'Overgrown Tomb':2,'Blooming Marsh':2,'Wastewood Verge':3,'Sandsteppe Citadel':3,'Savai Triome':1,'Blood Crypt':1,'Inspiring Vantage':2,'Thriving Bluff':2,'Forest':4,'Swamp':3,'Plains':3,'Mountain':1,'Thriving Grove':2})
assert sum(spells.values())==48 and sum(lands.values())==32
ls,ns=entries(lands),entries(spells)
slug='09-80-ultimatum-desastreux';path=ROOT/slug;path.mkdir(exist_ok=True)
plan='Construire une armée avec Roots, Mycotyrant et Vanguard ; convertir les créatures en mana de toutes couleurs ; lancer Ruinous Ultimatum pour détruire les permanents non-terrain adverses puis attaquer. Broodlord cherche l’Ultimatum ou la pièce de mana manquante. Unburial Rites et Victimize permettent de ramener Broodlord après meule/défausse.'
notes=[
'Ultimatum coûte exactement RRWWWBB : ce sont sept manas colorés, sans générique. La base à quatre couleurs est un compromis ; cette variante ne promet pas un lancement rapide à chaque partie.',
'Roots donne la capacité de mana aux créatures-jetons ; Cryptolith Rite et Enduring Vitality la donnent à toutes tes créatures. Elles peuvent alors produire rouge/blanc/noir quelle que soit leur couleur, mais le mal d’invocation interdit normalement leurs capacités avec le symbole d’engagement.',
'Broodlord donne convocation aux sorts lancés depuis l’exil, dont la carte qu’il cherche. Une créature engagée pour convocation paie un mana de sa propre couleur ou un générique. Une créature verte ne paie donc aucun symbole de RRWWWBB ; les copies blanches de Vanguard peuvent payer W et les Fungus noirs B. La convocation peut engager des créatures arrivées ce tour-ci. Une même créature ne peut pas à la fois produire du mana en s’engageant et être engagée pour convocation dans ce lancement.',
'Big Score crée deux Trésors et peut défausser Broodlord pour préparer une réanimation. Il demande lui-même une source rouge. Collector’s Vault prépare le cimetière et produit un Trésor à chaque activation.',
'Nyx Weaver et Dryad’s Revival peuvent reprendre un Ultimatum meulé en main. Sevinne ne le peut pas : il sert à récupérer Roots, Cryptolith Rite, Vanguard, Mycotyrant ou Weaver. Broodlord cherche dans la bibliothèque, pas dans le cimetière.',
'Rumble cherche un permanent : il ne met pas Ultimatum en main. Son jeton peut fournir du mana générique pour développer le moteur ; son mana incolore seul ne paie pas Ultimatum. Avec Roots, il possède aussi une capacité de mana de toute couleur soumise au mal d’invocation. Sous Vanguard, il devient une copie et perd sa capacité de sacrifice native.',
'Choisir rouge avec Thriving Grove si le rouge manque ; Thriving Bluff peut choisir blanc ou la couleur manquante. Sept terrains donnent directement du rouge (dont Savai engagé et les deux Bluff engagés), les deux Grove peuvent en ajouter selon le choix ; cela reste complété par les Trésors et les créatures.',
'Détruire ne contourne pas indestructible : Ultimatum ne nettoiera pas les Slivers protégés par Sliver Hivelord tant que cette protection reste active. Le sort ne cible pas, mais ne détruit pas les terrains adverses.',
'Deux Gaea’s Blessing protègent l’auto-meule, avec la tension habituelle sur les cibles de réanimation. Les cartes à cinq/huit manas se multiplient ici : surveiller les départs lents.',
'Sans célérité ni moteur de dégagement, les créatures engagées pour payer Ultimatum ne pourront pas attaquer immédiatement. Essayer de garder des attaquants dégagés, ou utiliser prioritairement terrains et Trésors.',
'Expérience non testée en partie et non validée par Arena ; aucune garantie de classement ou de tour de lancement.'
]
d={'schema_version':'mtga.deck-draft.v1','id':stable_id('mtga:experiment:2026-09-19:'+slug),'revision':1,'name':'Ultimatum — les jetons paient le nettoyage','status':'draft_unplayed','created_at':datetime.now(timezone.utc).isoformat(),'source_snapshot_id':json.loads((P/'card-records.jsonl').open().readline())['snapshot_id'],'format':'Historic','legality_status':'not_validated_by_arena','main_deck':{'lands':ls,'nonlands':ns},'sideboard':[],'summary':{'total_cards':80,'lands':32,'nonlands':48,'library_reset_cards':2,'ruinous_ultimatum':3,'hoarding_broodlord':3},'plan':plan,'tradeoffs':notes,'validation':{'owned_quantities_by_printing':True,'total_size':True,'max_four_by_name_except_basics':True}}
(path/'deck.json').write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
txt=arena_clipboard(d);assert sum(int(l.split()[0]) for l in txt.splitlines() if re.match(r'^\d+ ',l))==80
(path/'arena.txt').write_text(txt)
(path/'README.md').write_text('# Ultimatum désastreux — 80 cartes\n\n'+plan+'\n\n## Interactions et compromis\n\n'+'\n'.join('- '+n for n in notes)+'\n\n## Test\n\nNoter : tour du premier Ultimatum, source des deux rouges et trois blancs, nombre de cartes mortes avant trois manas, Ultimatum pioché/cherché/récupéré, survie du moteur de mana. Tout est possédé dans le snapshot, sans craft ; anciens decks inchangés.\n\n## Liste\n\n'+'\n'.join(f'- {q} {n}' for n,q in (spells+lands).items())+'\n')
index=ROOT/'README.md';s=index.read_text();line=f'- [80 — Ultimatum désastreux]({slug}/README.md) : [Arena]({slug}/arena.txt), [JSON]({slug}/deck.json).'
if line not in s:index.write_text(s+'\n## Nettoyage asymétrique\n\n'+line+'\n')
print(path);print('80 cards; ownership and import quantities verified')
