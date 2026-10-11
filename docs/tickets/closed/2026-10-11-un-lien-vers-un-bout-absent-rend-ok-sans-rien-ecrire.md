# Un lien vers un bout absent rend `Ok` sans rien écrire — et `AddRefNode` l'annonce comme posé

- **état** : corrigé par la marche 1 bis du lien générique (le commit qui déplace ce fichier dans `closed/`, 11 octobre 2026) — `Catalog::link_jusqu_a` vérifie ses bouts quand le lien se pose maintenant (exigence non vide), par `Select::by_uuids` (rag3db et PostgreSQL), et refuse un bout absent en le nommant ; les liens différés (régime par lot, indexation du code) se comptent au drain comme avant, et un témoin tient cette borne
- **gravité** : réponse fausse — un outil annonce un lien qui n'existe pas
- **atteignable en service** : oui, par `add_ref` du gabarit `memory` avec une `fiche` qui ne désigne aucune mémoire
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

`Catalog::link(relation, de, vers, props)` met le lien en file et le draine. La
relation se pose par `MATCH (a {_uuid}), (b {_uuid}) MERGE (a)-[r]->(b)`
(rag3db) ou `INSERT … ON CONFLICT` (PostgreSQL). Quand un bout n'existe pas, le
`MATCH` ne produit rien, **sans erreur** : le verbe rend `Ok`, et rien n'est en
base.

`RelationBatchNode` s'en garde : il vérifie l'existence des bouts (`get_many`)
avant d'écrire, et refuse le lot entier en nommant le bout manquant.
**`AddRefNode` ne s'en garde pas** : il recompose l'uuid de la fiche depuis ses
champs d'identité, appelle `cat.link`, et rend
`executed: true, lien: {de, relation, vers}` — pour une fiche qui peut ne pas
exister (une faute de frappe dans `claim`, un `reach` différent).

## La recette minimale

`tests/e2e_garde_des_liens.rs`, `un_lien_vers_un_bout_absent_rend_ok_et_n_ecrit_rien` :
une `Fiche` écrite, un `Sujet` jamais écrit, `cat.link("ANCRE", fiche, sujet)` →
`Ok`, et `MATCH ()-[r:ANCRE]->() RETURN count(r)` → 0.

## Le témoin

Celui ci-dessus, qui épingle le comportement **tel qu'il est** : il rougira le
jour où le verbe refuse un bout absent, et se retournera alors.

## La cause

Le `MERGE` sur deux `MATCH` est idempotent et rapide, et c'est pour ça qu'il a été
choisi ; son revers est qu'un bout absent n'est pas une erreur. Le contrôle
d'existence a été mis dans **un** appelant (`RelationBatchNode`) au lieu du verbe.

## Ce qu'il faut pour le fermer

Au choix :

1. **le verbe vérifie** : `link` et `mettre_en_file_les_liens` comptent les liens
   effectivement posés au drain et refusent (ou avertissent, nommément) quand un
   bout manque. Ferme le défaut pour tous les appelants d'un coup, y compris
   l'indexation du code — à mesurer, elle pose des centaines de milliers de liens ;
2. **chaque appelant vérifie**, comme `RelationBatchNode` : `AddRefNode` vérifie
   la fiche avant de lier. Plus local, et le prochain appelant l'oubliera.

Le nœud de lien générique (page
`extension/rag3weaver/docs/11-octobre-2026-memoire-longue/01-un-lien-entre-deux-identites-declarees.md`)
vérifie l'existence lui-même dans tous les cas ; s'il remplace `AddRefNode` pour
la partie « lien », la 2 tombe d'elle-même pour lui.
