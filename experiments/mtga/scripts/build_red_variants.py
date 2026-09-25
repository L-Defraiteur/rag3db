"""Four owned-card branches of the Ultimatum experiment."""
import json,re
from collections import Counter
from datetime import datetime,timezone
from build_fun_variants import ROOT,P,entries,stable_id
from export_deck import arena_clipboard
base=json.loads((ROOT/'09-80-ultimatum-desastreux/deck.json').read_text())
base_spells=Counter();base_lands=Counter()
for c in base['main_deck']['nonlands']:base_spells[c['name_en']]+=c['quantity']
for c in base['main_deck']['lands']:base_lands[c['name_en']]+=c['quantity']
# Each experiment keeps the same 32 lands and 80-card size to ease comparison.
specs=[
('10-80-rouge-mixte','Le laboratoire rouge — moteurs combines',
 {'Snarling Gorehound':2,'Malevolent Rumble':2,'Mesmeric Orb':1,'Hoarding Broodlord':1,'Ruinous Ultimatum':1},
 {'Faithless Looting':2,'Enterprising Scallywag':2,'Impact Tremors':2,'Enduring Courage':1},
 'Version de découverte : Looting prépare le cimetière, Scallywag produit des Trésors, Tremors transforme les arrivées en dégâts et Courage donne la célérité. Ultimatum reste présent en deux exemplaires avec deux Broodlord pour le chercher.',
 'Les pièces rouges se partagent les places : chacune apparaît moins souvent que dans une version spécialisée. Courage donne +2/+0 aux arrivants et peut améliorer les attaques, mais les créatures engagées pour leur mana ne peuvent pas aussi attaquer. Cette version sert à découvrir le moteur que tu préfères.',
 'Observer quelle pièce rouge est réellement utile, et lesquelles restent en main faute de rouge ou de créatures.'),
('11-80-tresors-ultimatum','Tresors — financer RRWWWBB',
 {'Snarling Gorehound':2,'Malevolent Rumble':2,'Mesmeric Orb':2,'Thunderbond Vanguard':1,'Enduring Vitality':1,"Sevinne's Reclamation":1},
 {'Faithless Looting':3,'Enterprising Scallywag':3,'Crime Novelist':2,"Sorceress's Schemes":1},
 'Priorité au lancement des trois Ultimatum : Looting/Scallywag, Big Score et Vault produisent et préparent les ressources ; Novelist ajoute du rouge quand un Trésor est sacrifié. Broodlord cherche Ultimatum ; Schemes, Weaver et Revival récupèrent un exemplaire meulé.',
 'Scallywag donne un Trésor par déclenchement en fin de tour si tu as descendu, pas un par carte meulée. Les copies de Scallywag ont chacune leur déclenchement, mais cette liste ne les copie pas volontairement. Novelist déclenche une capacité : pour utiliser son rouge dans le même Ultimatum, sacrifier les Trésors et laisser résoudre avant de commencer le lancement. Schemes reprend en main, sans rendre Ultimatum gratuit.',
 'Noter le tour du premier Ultimatum et les couleurs manquantes. Comparer la fréquence des lancements au 09 original.'),
('12-80-jetons-degats','Jetons — chaque arrivee fait mal',
 {'Ruinous Ultimatum':2,'Hoarding Broodlord':1,'Unburial Rites':1,'Victimize':1,'Big Score':1,'Enduring Vitality':1,'Cryptolith Rite':1,"Sevinne's Reclamation":1,"Collector's Vault":1},
 {'Impact Tremors':4,'Witty Roastmaster':1,'Mirkwood Bats':1,'Parallel Lives':1,'Doubling Season':1,'Faithless Looting':2},
 'Les quatre Tremors sont la priorité rouge. Roots/Mycotyrant créent les arrivées, Parallel Lives et Doubling Season les multiplient. Bats voit aussi la création et le sacrifice des Trésors. Un Ultimatum reste comme sortie de secours, cherchable par Broodlord.',
 'Tremors et Roastmaster infligent des dégâts à l’arrivée ; Bats fait perdre de la vie quand tu crées ou sacrifies un jeton. Ce sont des conditions distinctes. Les doubleurs ne créent rien seuls. Cette version conserve un petit module réanimation, mais ne vise plus principalement le lancement répété d’Ultimatum.',
 'Noter les victoires sans attaque et les mains qui ont beaucoup de multiplicateurs sans production de jetons.'),
('13-80-celerite-mana','Celerite — les nouveaux jetons travaillent',
 {'Snarling Gorehound':2,'Mesmeric Orb':1,'Malevolent Rumble':1,'Ruinous Ultimatum':1,'Hoarding Broodlord':1,'Big Score':1},
 {'Enduring Courage':1,'Ogre Battledriver':2,'Draconautics Engineer':1,'Faithless Looting':2,'Impact Tremors':1},
 'Courage et Battledriver donnent la célérité aux autres créatures qui arrivent. Roots/Rite/Vitality permettent alors aux nouveaux jetons de produire immédiatement toutes les couleurs. Engineer peut donner la célérité à tes autres créatures déjà présentes avec sa capacité exhaust. Deux Ultimatum restent pour nettoyer, Tremors fournit un petit complément de dégâts.',
 'Courage/Battledriver coûtent 2RR : ce sont des moteurs de milieu de partie, pas une garantie de départ rapide. Ils n’accordent pas rétroactivement la célérité aux créatures déjà présentes. Exhaust s’active une seule fois par capacité pour cet objet tant qu’il reste le même permanent. Un jeton utilisé pour du mana reste engagé et ne pourra pas attaquer sans autre effet de dégagement. Aucune boucle infinie n’est revendiquée.',
 'Compter les tours où la célérité accélère vraiment Ultimatum ou donne une attaque gagnante, et ceux où le double rouge bloque.')]
for slug,title,cuts,adds,plan,limits,test in specs:
 spells=base_spells.copy();spells.subtract(cuts)
 assert all(q>=0 for q in spells.values()),slug
 spells.update(adds);spells=+spells
 assert sum(spells.values())==48,(slug,sum(spells.values()))
 ns,ls=entries(spells),entries(base_lands)
 d={'schema_version':'mtga.deck-draft.v1','id':stable_id('mtga:experiment:2026-09-19:'+slug),'revision':1,'name':title,'status':'draft_unplayed','created_at':datetime.now(timezone.utc).isoformat(),'source_snapshot_id':base['source_snapshot_id'],'parent_draft_id':base['id'],'format':'Historic','legality_status':'not_validated_by_arena','main_deck':{'lands':ls,'nonlands':ns},'sideboard':[],'summary':{'total_cards':80,'lands':32,'nonlands':48,'library_reset_cards':spells["Gaea's Blessing"]},'changes_from_09':{'remove':cuts,'add':adds},'plan':plan,'tradeoffs':limits,'test_focus':test,'validation':{'owned_quantities_by_printing':True,'total_size':True,'max_four_by_name_except_basics':True}}
 path=ROOT/slug;path.mkdir(exist_ok=True)
 (path/'deck.json').write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
 txt=arena_clipboard(d);assert sum(int(l.split()[0]) for l in txt.splitlines() if re.match(r'^\d+ ',l))==80
 (path/'arena.txt').write_text(txt)
 (path/'README.md').write_text(f'# {title} — 80 cartes\n\n{plan}\n\n## Compromis\n\n{limits}\n\nLes 32 terrains sont ceux du 09 pour une première comparaison : sept sources rouges directes, dont certaines arrivent engagées, et deux Thriving Grove pouvant choisir rouge. Le rouge au premier tour et le double rouge ne sont pas garantis. Les Trésors et créatures à mana complètent ensuite les couleurs. Après les premiers essais, ajuster les terrains séparément si nécessaire.\n\nDeux Gaea’s Blessing préservées ; les resets restent en tension avec la préparation du cimetière. Toutes les cartes sont possédées dans le snapshot, aucune réservation entre variantes ni craft nécessaire. Variantes non testées en partie, sans classement de puissance mesuré.\n\n## Changements depuis le 09\n\n'+ '\n'.join(f'- Retirer {q} {n}' for n,q in cuts.items())+'\n'+ '\n'.join(f'- Ajouter {q} {n}' for n,q in adds.items())+f'\n\n## À observer\n\n{test}\n\n## Liste\n\n'+'\n'.join(f'- {q} {n}' for n,q in (spells+base_lands).items())+'\n')
 print(slug,'80 cards, 32 lands, quantities owned verified')
index=ROOT/'README.md';s=index.read_text();heading='\n## Quatre branches rouges\n';s=s.split(heading)[0]+heading+'\n'+'\n'.join(f'- [{title}]({slug}/README.md) : [Arena]({slug}/arena.txt), [JSON]({slug}/deck.json).' for slug,title,*_ in specs)+'\n';index.write_text(s)
