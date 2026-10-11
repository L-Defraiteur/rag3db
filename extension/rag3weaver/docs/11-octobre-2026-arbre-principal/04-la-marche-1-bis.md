# La marche 1 bis — plus de cellule courante dans le catalogue

*Arbre principal, 11 octobre 2026. Page courte avant le code : la marche 1 bis de la forme C
(`01-le-demon-possede-le-catalogue.md`, §7, décision 12 de Lucie). La carte du code a été faite
par un agent ; les numéros de ligne sont ceux de master à midi.*

## Ce qui est à retirer

Le catalogue a **une cellule courante** : un champ `scope` (`catalog.rs:327`), le défaut
`default/default`, et un seul écrivain, `set_scope` (`catalog.rs:4248`). Celui-ci échange les
index plein texte et creux de la cellule avec ceux qu'il gare (`parked_fts`, `parked_sparse`). En
service, `set_scope` n'est appelé que par `Catalog::rechercher` : il bascule la cellule **du
catalogue entier** pour chaque cellule demandée (`options.scope`, `options.scopes`), relâche le
verrou, lance le graphe, puis restaure.

Une trentaine de lectures de cette cellule :
- le tampon des lignes écrites (`_org`/`_project`) ;
- le filtre vectoriel ;
- les clés des sessions de synchronisation et de leur fin, et la marque des relations en cours ;
- le bus d'événements ;
- les services `scope` et `cellule` donnés aux nœuds.

Les nœuds la lisent **au moment où ils prennent le verrou**, sans la recevoir :
`EntityRecordNode`, `ReactTransitionNode`, `AddRefNode`, la recherche vectorielle, les nœuds de
synchronisation. Le réacteur fige sa cellule et ses index au montage, et ne la reçoit jamais de
l'événement.

**Pourquoi la sérialisation de la marche 1 ne suffit pas.** `rechercher` relâche le verrou entre
les nœuds, et le réacteur tourne hors du verrou de l'appel. Tenir un verrou d'appel qui couvre
aussi le réacteur sérialiserait tout, alors que la décision 12 veut que deux sessions lisent en
même temps.

## La forme

**La cellule passe par la requête, jamais par un état partagé.**

1. **`Backend::call_in(cell, name, args)`.** L'hôte du démon passe la cellule de la session.
   `call(name, args)` reste, avec la cellule par défaut.
2. **Les services de l'appel portent sa cellule.** `build_services` la reçoit, et les services
   `scope` et `cellule` sont ceux de l'appel, pas ceux du catalogue. C'est déjà le cas pour
   `InsertRecordNode`, qui lit `scope` dans ses services.
3. **Les méthodes du catalogue qui lisaient `self.scope` la reçoivent en paramètre** :
   l'ingestion, la mise en file des créations, `update`, le filtre vectoriel, les clés de session
   et de fin, `relations_pending`, le bus. Les nœuds la lisent dans leurs services et la passent.
4. **Les index plein texte et creux, par cellule.** Une table `cellule → index`, ouverte à la
   demande, remplace l'échange et `parked_*`. Les fonctions `fts_handles_in` et
   `sparse_handles_in` existent déjà, sans appelant.
5. **`rechercher` boucle sur ses cellules sans rien basculer.** Chaque recherche reçoit sa
   cellule ; le fan-out fusionne comme avant. Disparaît en même temps le défaut d'une panique
   qui ne restaurait pas la cellule.
6. **Le réacteur prend la cellule de l'événement déclencheur.** Le bus porte déjà la cellule de
   l'événement (`events.rs:124`). Les services de la réaction se construisent à chaque exécution,
   plus au montage.
7. **Le filet de la marche 1** saute pour les lectures et reste pour les écritures (décision 12).

## Trois choses trouvées en chemin, à trancher

1. **`_uuid` ne dépend pas de la cellule** (`hashsafe_uuid`, `catalog.rs:1781`). **Confirmé par
   le témoin** (`e2e_scope::la_meme_fiche_dans_deux_cellules_fait_deux_lignes`, rouge sur la
   branche `marche-1-bis`). La même fiche écrite dans une seconde cellule ne fait pas de seconde
   ligne : elle **déplace** la ligne de la première cellule, qui perd sa fiche sans un mot
   (ticket `2026-10-11-la-meme-fiche-dans-deux-cellules-n-a-qu-une-ligne.md`). **La forme
   candidate**, une migration à faire valider par Lucie avant le code : l'identité dérivée de
   (cellule, identité déclarée). La cellule par défaut peut garder ses `_uuid` (seules les autres
   entrent dans le hachage), et les bases d'aujourd'hui n'ont presque toutes que celle-là.
   Ce qu'elle change : le lien générique de la mémoire nomme ses bouts par leur identité, et devra
   dire dans quelle cellule il les cherche ; un lien entre deux cellules devient une décision
   explicite, et non plus un accident de hachage.
2. **Des marques ne sont pas par cellule** : `FTS_PENDING{entité}`, les marques de progression et
   les marques de dette. **Décidé** (orchestration) : préfixées de la cellule, comme les clés de
   session. Le ticket « index plein texte hors du point de reprise » le dira.
3. **`set_scope`** : **décidé**, il reste la cellule par défaut du catalogue, pour la ligne de
   commande et les tests seulement. Sous l'hôte, un chemin de service qui l'atteindrait est
   **refusé en le disant**, et non laissé à un commentaire.

## Les marches, testables une à une

| pas | quoi | témoin | coût |
|---|---|---|---|
| a | `call_in` ; les services portent la cellule ; `EntityRecordNode`, `AddRefNode` et le filtre vectoriel la lisent là | deux sessions sur deux cellules écrivent la même identité : chacune relit la sienne (et le témoin du `_uuid`, point 1) | 1 |
| b | les index par cellule ; `rechercher` sans bascule | deux recherches concurrentes sur deux cellules, chacune ne rend que la sienne | 1 |
| c | le réacteur prend la cellule de l'événement | le ticket de la réaction qui écrit dans la cellule basculée, retourné en témoin et fermé | ½ à 1 |
| d | les marques par cellule (point 2) ; le filet ne sert plus aux lectures | deux synchronisations de la même source dans deux cellules | ½ |

En tout, 3 à 3,5 jours d'agent, plus que les 2 à 3 de la page C : la carte a trouvé les index
garés et le réacteur figé au montage.
