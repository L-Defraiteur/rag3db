# Aucun nœud ne relie deux entités gérées par leur identité

- **état** : corrigé `6d212551e` (marche 1 du lien générique, 11 octobre 2026) — il n'y avait pas de nœud manquant, il y avait une garde de trop : `RelationBatchNode` relie désormais toute paire d'entités déclarées, gérées ou non, et sa garde d'états est déclarée (`13c6d950f`)
- **gravité** : capacité absente — une relation déclarée entre deux entités versionnées n'a aucun verbe
- **atteignable en service** : oui ; c'est le cas du gabarit `memory` (`ANCHORED_TO`, `REPLACES`)
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

Un manifeste déclare ses relations, et trois nœuds seulement en écrivent :

| nœud | ce qu'il relie | la limite |
|---|---|---|
| `RelationBatchNode` | deux identités déclarées, relation sans propriété | **refuse si un bout est géré** (cycle de vie, révision, dates serveur) |
| `AddRefNode` | une fiche gérée → une `Ref` | fabrique la `Ref` en chemin ; ne relie pas deux entités du schéma |
| `LinksNode` | lecture seule | — |

Donc une relation déclarée **entre deux entités gérées** n'a aucun verbe. Dans
le gabarit `memory` c'est `ANCHORED_TO` (`Memory → Subject`) et `REPLACES`
(`Memory → Memory`) : deux relations déclarées, zéro moyen de les écrire, et une
réaction (`review_anchored`) qui remonte la première et ne trouve donc jamais
rien. Voir
`2026-10-11-aucun-outil-n-ancre-une-memoire-a-un-sujet.md`, dont ceci est la
cause générale.

## La recette minimale

Jouée le 11 octobre 2026 par MCP, sur une copie hors dépôt du gabarit `memory` :

```
anchor(links: [{from: {claim, reach}, to: {name}}])   # link_snapshot.mmd, relation=ANCHORED_TO
→ "snapshot links cannot modify relations of managed entities"
```

## Le témoin

Aucun. Il n'y a rien à éprouver avant que le verbe existe ; le témoin naîtra
avec lui.

## La cause

`RelationBatchNode` a été écrit pour les liens d'un **instantané** — sa propre
documentation le dit (« Snapshot links use stable endpoint identities »). Sa
garde (`backend_nodes.rs:757-766`) écarte les entités gérées, ce qui est
cohérent avec cette portée. Rien n'a été écrit depuis pour le cas général, parce
qu'aucune réaction ne remontait de relation : la capacité n'avait pas de lecteur.

## Ce qu'il faut pour le fermer

**Un `LinkRecordsNode(relation, links)` générique** : deux identités déclarées
par bout (les champs `hashsafe`), une relation **sans propriété**, les gardes de
cycle de vie des deux bouts **inchangées** — poser une relation n'est pas éditer
un record, donc ni la révision ni `updated_at` ne bougent. Dans la liste blanche
de base (`BASE_NODES`), donc disponible aux outils **et** aux réactions ; il
n'écrit qu'en base, par les verbes du catalogue, ne lit aucun fichier et ne lance
aucune commande.

Puis, **dans le même geste**, exposer les deux relations du gabarit `memory` :
`anchor` (`ANCHORED_TO`) et `supersede_link` (`REPLACES`) — la vision de la
mémoire longue emploie la seconde et aucun outil ne l'écrit non plus.

**Ordre de grandeur : une à deux passes.** Le nœud et son schéma, une fiche
générique dans `templates/tools/`, deux déclarations dans le gabarit, un témoin
e2e (`e2e_reaction_montee` existe déjà : il pose son ancre par
`Catalog::link`, il suffira de la poser **par l'outil** — ce qui en fait du même
coup le témoin de bout en bout de la surface exposée), et le compte de
`BASE_NODES` à corriger dans le même commit (`les_listes_blanches_ne_s_elargissent_pas_en_silence`).

**Ce qu'il ne faut PAS faire** : élargir la portée de `RelationBatchNode`. C'est
le moins de code et le plus de risque — **aucun témoin n'épingle sa garde**,
donc ce qu'elle protège n'est pas éprouvé, et une garde ne se relâche pas sans
prouver ce qu'elle mesurait.

## Pourquoi c'est à Lucie

C'est une **signature nouvelle** : un type de nœud de plus, et une entrée de
plus dans une liste blanche de sécurité. Les deux se décident, pas se
constatent.
