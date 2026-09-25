"""Reproduce four owned-card experiments; never modifies Arena."""
import json, sys, re
from pathlib import Path
from collections import Counter
from datetime import datetime, timezone
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from identity import stable_id,card_id
from export_deck import arena_clipboard
P=Path(__file__).resolve().parents[1]/'data'
CS=list(map(json.loads,(P/'cards.jsonl').open()))
ROOT=P/'deck-drafts/experiments-2026-09-19'
CORE=Counter({'Insidious Roots':4,"Stitcher's Supplier":4,'Rubblebelt Maverick':4,'Snarling Gorehound':2,'The Mycotyrant':2,'Thunderbond Vanguard':2,'Malevolent Rumble':2,'Unearth':3,"Gaea's Blessing":2,'Rite of Oblivion':2})
LAND=Counter({'Temple Garden':2,'Godless Shrine':1,'Overgrown Tomb':2,'Blooming Marsh':2,'Wastewood Verge':4,'Sandsteppe Citadel':3,'Underground Mortuary':2,'Forest':4,'Swamp':4,'Plains':3})
SPECS=[
(70,'01-70-reanimation',{'Unburial Rites':3,'Victimize':2,'Overlord of the Balemurk':2,'Troll of Khazad-dûm':2,'Titanoth Rex':1,'Altanak, the Thrice-Called':1,'Ghalta, Stampede Tyrant':1,'Hoarding Broodlord':1,'Moonshaker Cavalry':1,'Mesmeric Orb':2},{},
'Reanimation sans attendre',
'Troll, Titanoth et Altanak peuvent se défausser via leurs capacités : ils préparent eux-mêmes les cibles. Supplier et Maverick préparent le cimetière dès le premier tour. Unearth récupère le moteur ; Unburial Rites et Victimize récupèrent les monstres.',
'Moonshaker donne un effet immédiat aux jetons ; Ghalta vide les grosses créatures de la main. Victimize demande deux cibles et une créature à sacrifier. Garder une main avec mana et action précoce, pas seulement des monstres.'),
(80,'02-80-cimetiere-autonome',{'Unburial Rites':4,'Victimize':2,'Overlord of the Balemurk':2,'Balustrade Wurm':2,'Reassembling Skeleton':2,'Lingering Souls':2,'Roar of the Wurm':1,'Gnaw to the Bone':1,"Sevinne's Reclamation":1,'Nethergoyf':1,'Ghalta, Stampede Tyrant':1,'Hoarding Broodlord':1,'Troll of Khazad-dûm':1,'Mesmeric Orb':2},{'Forest':1,'Swamp':1,'Plains':1},
'Le cimetiere comme seconde main',
'Balustrade Wurm revient pour quatre manas sous délire ; Skeleton revient à répétition pour déclencher Roots. Lingering Souls et Roar of the Wurm restent jouables une fois meulés. Sevinne récupère Roots, Vanguard ou Mycotyrant.',
'Les resets vident le cimetière et retardent délire, escape et Gnaw : conserver cette protection implique ce compromis. Le marqueur finalité de Balustrade empêche sa récursion indéfinie. Les copies créées sous Vanguard perdent le vol des Spirits. Priorité à tester pour découvrir des cartes hors de la liste originale.'),
(90,'03-90-jardin-des-terrains',{'The Necrobloom':3,'Hedge Shredder':3,'Perennial Behemoth':1,'Malevolent Rumble':2,'Overlord of the Balemurk':2,'Unburial Rites':3,'Victimize':2,'Troll of Khazad-dûm':2,'Titanoth Rex':1,'Altanak, the Thrice-Called':1,'Ghalta, Stampede Tyrant':1,'Moonshaker Cavalry':1,'Mesmeric Orb':2,"Dryad's Revival":1,"Sevinne's Reclamation":1,'Gnaw to the Bone':1,'Syr Konrad, the Grim':1},{'Khalni Garden':2,'Deceptive Landscape':2,'Forest':1,'Swamp':1,'Plains':1,'Thriving Heath':1},
'Le jardin qui pousse depuis le cimetiere',
'Necrobloom donne dredge aux terrains ; Hedge Shredder met sur le champ de bataille les terrains envoyés de la bibliothèque au cimetière. Les arrivées déclenchent Necrobloom. Behemoth permet de jouer les terrains du cimetière, dans la limite normale de terrains par tour, et possède unearth.',
'Le moteur nécessite plusieurs pièces. Sept noms de terrains différents comptent pour Necrobloom : les illustrations différentes de Forest ne comptent pas. Hedge Shredder est un véhicule, pas une cible de réanimation de créature au cimetière. Choisir noir ou vert avec Thriving Heath selon la main. Vanguard remplace les jetons Plant/Zombie, mais conserve leur quantité.'),
(100,'04-100-menagerie',{'Unburial Rites':4,'Victimize':3,'Overlord of the Balemurk':2,'Troll of Khazad-dûm':2,'Titanoth Rex':1,'Altanak, the Thrice-Called':1,'Ghalta, Stampede Tyrant':2,'Hoarding Broodlord':2,'Moonshaker Cavalry':1,'Koma, World-Eater':1,'Ulamog, the Defiler':1,'Emrakul, the Aeons Torn':1,'Mesmeric Orb':2,'The Necrobloom':2,'Hedge Shredder':2,'Paradox Engine':1,'Cryptolith Rite':1,'Enduring Vitality':1,'Syr Konrad, the Grim':1,'Dreadhound':1,'Assemble the Team':1,"Sevinne's Reclamation":1,'Gnaw to the Bone':1},{'Khalni Garden':2,'Deceptive Landscape':2,'Forest':2,'Swamp':2,'Plains':2,'Thriving Grove':1},
'La menagerie et ses sorties explosives',
'Préserve les gros monstres, trois resets (deux Blessing et Emrakul), et la possibilité de transformer les jetons en mana. Broodlord cherche une pièce ; Ghalta déploie les créatures bloquées en main. Konrad et Dreadhound ajoutent une victoire par dégâts/perte de vie autour du cimetière.',
'C’est la version la plus ambitieuse et la moins régulière attendue, sans résultat mesuré. Koma dépend de la réanimation ou des sources de mana de toute couleur pour le bleu. Réanimer Ulamog ne déclenche pas son effet de lancement ; ses marqueurs dépendent des cartes en exil. Emrakul est surtout un reset : sa capacité complique la réanimation et celle-ci ne donne aucun tour supplémentaire. Paradox Engine ne contourne pas le mal d’invocation et ne garantit pas une boucle infinie.')]

def entries(counts):
 out=[]
 for name,q in counts.items():
  assert q<=4 or name in ['Forest','Swamp','Plains'],name
  prints=sorted((c for c in CS if c['name_en']==name and c['owned'] and c['is_primary']),key=lambda c:(-c['owned'],c['arena_id']))
  assert sum(c['owned'] for c in prints)>=q,(name,q)
  for c in prints:
   take=min(q,c['owned']);q-=take
   if not take:continue
   row={k:c[k] for k in ['arena_id','name_en','name_fr','mana_cost','colors','color_identity','type_en','type_fr','text_en','text_fr','set','collector_number','abilities','mechanics','linked_faces']}
   types=c['type_en'].split(' — ')[0].split()
   row.update(card_id=card_id(c['arena_id']),quantity=take,owned_this_printing=c['owned'],is_land='Land' in types,card_types=[t for t in types if t not in ['Legendary','Basic','Snow']])
   if row['is_land']:
    basic={'Forest':['G'],'Swamp':['B'],'Plains':['W']}
    production='\n'.join(line for line in c['text_en'].splitlines() if 'Add ' in line)
    symbols=basic.get(name,sorted(set(re.findall(r'\{([WUBRGC])\}',production))))
    row['mana_production']={'fixed_or_conditional_symbols':symbols,'chosen_additional_color':name.startswith('Thriving'),'conditions_text':c['text_en'],'derivation':'rules_text_and_basic_type; not color_identity','fetches_basic_types':['Plains','Swamp','Forest'] if name=='Deceptive Landscape' else []}
   out.append(row)
   if q==0:break
 return out

def main():
    ROOT.mkdir(parents=True,exist_ok=True)
    index=['# Laboratoire Mycotyrant / Vanguard\n','Quatre expériences avec les cartes possédées du snapshot local. Pas de craft, pas de modification Arena, aucune performance mesurée. Les collections sont réutilisées entre variantes : ce ne sont pas quatre réservations de cartes.\n','Les decks gardent tous 2 Vanguard, 2 Mycotyrant, 4 Roots et au moins 2 resets. Les cartes physiques terrain sont séparées ; aucun terrain modal dans ces quatre listes.\n']
    for size,slug,extra,land_extra,title,plan,cautions in SPECS:
     spells=CORE+Counter(extra);lands=LAND+Counter(land_extra)
     total=sum(spells.values())+sum(lands.values())
     assert total==size,(slug,total,size)
     path=ROOT/slug;path.mkdir(exist_ok=True)
     ls=entries(lands);ns=entries(spells)
     stats={'total_cards':total,'lands':sum(lands.values()),'nonlands':sum(spells.values()),'always_tapped_lands':sum(c['quantity'] for c in ls if 'This land enters tapped.' in c['text_en']),'library_reset_cards':spells["Gaea's Blessing"]+spells['Emrakul, the Aeons Torn'],'one_mana_creature_cards':sum(c['quantity'] for c in ns if 'Creature' in c['card_types'] and c['mana_cost'] in ['{B}','{G}','{W}'])}
     d={'schema_version':'mtga.deck-draft.v1','id':stable_id('mtga:experiment:2026-09-19:'+slug),'revision':1,'name':title,'status':'draft_unplayed','created_at':datetime.now(timezone.utc).isoformat(),'source_deck_id':'df511012-2ab2-47e9-846d-8351e56cbc35','source_snapshot_id':json.loads((P/'card-records.jsonl').open().readline())['snapshot_id'],'format':'Historic','legality_status':'not_validated_by_arena','main_deck':{'lands':ls,'nonlands':ns},'sideboard':[],'summary':stats,'plan':plan,'tradeoffs':cautions,'validation':{'owned_quantities_by_printing':True,'total_size':True,'max_four_by_name_except_basics':True}}
     (path/'deck.json').write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
     txt=arena_clipboard(d);assert sum(int(line.split()[0]) for line in txt.splitlines() if re.match(r'^\d+ ',line))==size
     (path/'arena.txt').write_text(txt)
     (path/'README.md').write_text(f'# {title} — {size} cartes\n\n{plan}\n\n## Compromis et interactions\n\n{cautions}\n\n## Composition\n\n'+ '\n'.join(f'- {v} {n}' for n,v in (spells+lands).items())+f'\n\n{stats["lands"]} terrains, {stats["always_tapped_lands"]} systématiquement engagés, {stats["one_mana_creature_cards"]} créatures à un mana.\n\n## Essai\n\nCopier arena.txt puis Importer dans Arena. Jouer quelques parties en notant : couleurs indisponibles, premier tour utile, grosses cartes bloquées en main, sorts de réanimation sans cible, reset utile ou prématuré, tour de Vanguard. Ne pas conclure au taux de victoire sur quelques parties.\n')
     index.append(f'- [{size} — {title}]({slug}/README.md) : {stats["lands"]} terrains. [Import Arena]({slug}/arena.txt), [JSON]({slug}/deck.json).\n')
     print(slug,stats)
    index.append('\nOrdre de découverte proposé : 70 pour la réanimation, 80 pour les nouvelles cartes autonomes, 90 pour les terrains, 100 pour les sorties spectaculaires. Cet ordre est une préférence de test, pas un classement de puissance. Les changements de taille et de composition sont simultanés : ce ne sont pas des essais contrôlés de la taille seule.\n')
    (ROOT/'README.md').write_text('\n'.join(index))

if __name__ == "__main__":
    main()
