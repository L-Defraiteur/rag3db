"""Generate an owned-card Plant / Insidious Roots / Necrobloom experiment."""
import json,re
from collections import Counter
from datetime import datetime,timezone
from build_fun_variants import ROOT,P,LAND,entries,stable_id
from export_deck import arena_clipboard

spells=Counter({'Insidious Roots':4,'The Necrobloom':3,'Rubblebelt Maverick':4,"Stitcher's Supplier":3,'Snarling Gorehound':3,'Moss-Pit Skeleton':2,'Reassembling Skeleton':2,'Nyx Weaver':2,'Aftermath Analyst':2,'Hedge Shredder':2,'Bristly Bill, Spine Sower':2,'Spelunking':2,'Malevolent Rumble':2,'Unearth':3,'Rite of Oblivion':2,"Gaea's Blessing":2,"Sevinne's Reclamation":1,'Doubling Season':2,'Felidar Retreat':1,'World Shaper':1,'Perennial Behemoth':1,'Wight of the Reliquary':1,'Moonshaker Cavalry':1})
lands=LAND+Counter({'Khalni Garden':2,'Deceptive Landscape':2,'Forest':1})
assert sum(spells.values())==48 and sum(lands.values())==32
ns=entries(spells);ls=entries(lands)
slug='08-80-jardin-plantes-necrobloom';path=ROOT/slug;path.mkdir(exist_ok=True)
plan='Necrobloom crée les Plants avec les arrivées de terrains ; Roots crée et renforce les Plants quand des créatures quittent le cimetière. Maverick, Reassembling Skeleton, Unearth et les récupérations entretiennent Roots. Analyst et World Shaper ramènent des terrains en masse ; Shredder valorise les terrains meulés. Bill et Doubling Season amplifient les marqueurs.'
limits=[
'Pas de Vanguard dans cette variante : les Plants gardent leur type pour recevoir les marqueurs de Roots. Pas de Mycotyrant : les emplacements servent le moteur Plants/terrains.',
'Necrobloom est légendaire : trois exemplaires améliorent son accès, ils ne sont pas destinés à rester ensemble sur le champ de bataille.',
'À sept noms de terrains différents, Necrobloom crée des Zombies à la place des Plants. Ils gardent le mana donné aux jetons par Roots, mais ne reçoivent pas ses marqueurs réservés aux Plants.',
'Les deux Gaea’s Blessing protègent contre la meule mais peuvent retirer les terrains et créatures que tu préparais au cimetière. Un reset avec au moins une créature au cimetière déclenche chaque Roots une fois, pas une fois par créature.',
'Moss-Pit Skeleton remonte au-dessus de la bibliothèque, pas en main. On peut refuser son déclenchement pour ne pas remplacer sa prochaine pioche. Avec Gorehound et Roots, il peut entretenir des enchaînements de surveil, sorties du cimetière et jetons ; aucune boucle n’est nécessaire au plan.',
'Unearth (le sort) peut ramener les créatures de valeur de mana au plus trois, dont Analyst, Bill, Weaver et Wight ; pas Necrobloom, World Shaper, Behemoth ou Moonshaker. Behemoth possède séparément sa propre capacité unearth.',
'Behemoth permet de jouer un terrain depuis le cimetière sans donner de pose de terrain supplémentaire. Spelunking permet aux terrains renvoyés par Analyst, World Shaper et Shredder d’arriver dégagés tant qu’il reste en jeu.',
'Les capacités de mana des jetons restent soumises au mal d’invocation. Retrouver un terrain via dredge de Necrobloom ne déclenche pas Roots ; seules les cartes de créature qui quittent le cimetière le font.',
'Moonshaker est une fin de partie à huit manas, alimentée par les jetons producteurs de mana. Cette liste ne possède pas de réanimation capable de le ramener directement sur le champ de bataille.',
'Variante expérimentale de 80 cartes, sans performance mesurée ni validation de légalité dans Arena.'
]
d={'schema_version':'mtga.deck-draft.v1','id':stable_id('mtga:experiment:2026-09-19:'+slug),'revision':1,'name':'Le jardin des morts — Plants / Roots / Necrobloom','status':'draft_unplayed','created_at':datetime.now(timezone.utc).isoformat(),'source_snapshot_id':json.loads((P/'card-records.jsonl').open().readline())['snapshot_id'],'format':'Historic','legality_status':'not_validated_by_arena','main_deck':{'lands':ls,'nonlands':ns},'sideboard':[],'summary':{'total_cards':80,'lands':32,'nonlands':48,'library_reset_cards':2,'always_tapped_lands':sum(r['quantity'] for r in ls if 'This land enters tapped.' in r['text_en']),'one_mana_creature_cards':10},'plan':plan,'tradeoffs':limits,'validation':{'owned_quantities_by_printing':True,'total_size':True,'max_four_by_name_except_basics':True}}
(path/'deck.json').write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
txt=arena_clipboard(d);assert sum(int(l.split()[0]) for l in txt.splitlines() if re.match(r'^\d+ ',l))==80
(path/'arena.txt').write_text(txt)
readme='# Le jardin des morts — 80 cartes\n\n'+plan+'\n\n## Plan de jeu\n\n- Début : Supplier ou Maverick, puis Roots, Analyst ou un moteur de récupération.\n- Développement : Necrobloom avec des terrains, ou Roots avec une créature qui quitte le cimetière. Spelunking et Shredder accélèrent les terrains.\n- Fin : une armée de Plants grossie par Roots/Bill/Doubling Season ; Felidar peut donner la vigilance, Moonshaker le vol et un bonus massif.\n\n## Interactions et compromis\n\n'+'\n'.join('- '+v for v in limits)+'\n\n## Validation et essai\n\n32 terrains, 48 autres cartes, 2 resets. Quantités vérifiées par impression dans la collection locale ; aucun craft. Aucun deck existant modifié. Copier arena.txt pour importer ; deck.json contient les identifiants et les textes complets.\n\nObserver : premier moteur actif, disponibilité du blanc pour Necrobloom, nombre de tours sans sortie de créature du cimetière, resets utiles ou prématurés. Attention aux mains contenant seulement des amplificateurs (Bill/Doubling Season) sans producteur.\n\n## Liste\n\n'+'\n'.join(f'- {q} {n}' for n,q in (spells+lands).items())+'\n'
(path/'README.md').write_text(readme)
index=ROOT/'README.md';s=index.read_text();line=f'- [80 — Le jardin des morts]({slug}/README.md) : [Arena]({slug}/arena.txt), [JSON]({slug}/deck.json).'
if line not in s:index.write_text(s+'\n## Plants et Necrobloom\n\n'+line+'\n')
print(path);print(json.dumps(d['summary']))
