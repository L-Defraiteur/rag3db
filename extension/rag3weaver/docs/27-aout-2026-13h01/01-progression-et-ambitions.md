# 01 — Où on en est, et où on va

27 août 2026, 13 h 01. Trente-neuf commits depuis le rapport du 26 au soir.
Ce document est le point de reprise : ce qui est fait, ce qui est visé, et
**qui a voulu quoi** — parce que la moitié des bonnes idées de cette nuit
sont venues d'une objection de Lucie, pas d'un plan.

## 1. Ce qui est en place

| Domaine | État |
|---|---|
| **Identité d'un scope** | ne dépend plus ni de la signature ni du contenu. Un `edit` ne détruit plus rien |
| **Identité d'un fichier** | `(source, chemin absolu)` — trois versions pour y arriver ([04](../26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md)) |
| **Coordonnées** | `repo` / `repo_path` / `revision`, par fournisseurs souscrits ; deux clones se reconnaissent |
| **Relations** | toute référence laisse un `MENTIONS` **typé** ; l'ordre d'ingestion ne change plus rien, y compris pour `IMPLEMENTS` |
| **Ingestion** | index en masse (21,3 s → 6,2 s), court-circuit de l'inchangé (19,5 s → 2,1 s) |
| **Pré-filtre** | **exact** sur les trois signaux, après la correction du masque HNSW |
| **Domaine de travail** | `WorkDomain` — une sélection, câblée par le registre de services |
| **Rendu** | lentille de chemins : stockage absolu, affichage relatif |
| **Outils asynchrones** | accusé + poignée, résultat plus tard dans la boîte ([10](../26-aout-2026-20h29/10-outils-asynchrones.md)) |
| **Se taire / raccrocher** | `pause_dialogue`, `confirm_pause`, cinq genres, blocage circulaire détecté ([11](../26-aout-2026-20h29/11-le-droit-de-se-taire.md), [12](../26-aout-2026-20h29/12-conversations-a-plusieurs.md)) |
| **Conversations** | `Conversation`, `Participant`, nature sur l'arête ; cherchables, datées |
| **Dates** | `at` lisible, intervalles de borne à borne, fuseau réel (`jiff`) |
| **Machine** | `jobs = -2`, build confiné en cgroup, `unswap.sh`, carte par rôle |

**33 suites, 276 E2E, 862 unitaires.**

## 2. Les ambitions — celles de Lucie

Reprises de ses mots, avec où elles sont écrites.

1. **« Tout est un graphe. »** La recherche, les outils, la trace, la
   session, les politiques. C'est l'axiome, et il tient : un rendu est un
   nœud, une politique est une fiche, une conversation est un objet.
2. **« Une racine devrait rester qu'un point de vue. »** → [doc 04](../26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md).
   Fait, après deux versions fausses.
3. **« Un projet ne veut rien dire en soi ; un git s'appelle un git. »** →
   [doc 05](../26-aout-2026-20h29/05-origine-cellule-domaine.md). `Origin`,
   cellule, domaine : quatre axes séparés, rangés par vitesse de changement.
4. **« Domaine d'agent — ce qu'il a dans sa vision, indépendamment du
   projet. »** → `WorkDomain`, fait. Le défaut est **étroit et dérivé**, pour
   qu'un agent lâché sur un disque entier ne soit pas perdu.
5. **« Les tools asynchrones, dès maintenant — en vocal c'est chiant si
   l'agent sait plus parler. »** → [doc 10](../26-aout-2026-20h29/10-outils-asynchrones.md), fait.
6. **« Qu'il sache faire une pause, et terminer une discussion tout seul. »**
   → [docs 11](../26-aout-2026-20h29/11-le-droit-de-se-taire.md) et
   [12](../26-aout-2026-20h29/12-conversations-a-plusieurs.md), faits.
7. **« Pas de plafond codé en dur. »** La correction la plus utile de la
   nuit : mon plafond traitait le symptôme, sa pause traite la cause.
8. **« Les conversations cherchables, avec la date. »** Fait, avec le fuseau
   qui connaît ses règles — parce qu'elle a demandé « pourquoi on ajouterait
   pas la dépendance ? » et qu'il n'y avait pas de bonne raison.
9. **Les ponts entre projets** — relations multi-projets, projet-pont, ou
   `policyDomain` par entité. Noté au [doc 05 §10.4](../26-aout-2026-20h29/05-origine-cellule-domaine.md),
   **non tranché**, et aucune des trois voies ne demande de migration.

## 3. Les ambitions — celles que je vois

Ce que je pousserais, et pourquoi.

1. **La session comme graphe** ([doc 13](../25-aout-2026-18h58/13-la-session-comme-graphe.md)).
   C'est la pièce qui manque **deux fois** : le bloc d'attentes doit y être
   injecté, et l'`absorb` y vit. Garder le markdown entier d'un `read` au
   tour 8, c'est le payer huit fois. **Le plus gros gain mesurable qui
   dorme** — chiffre de vérité : dix tours, jetons avec et sans.
2. **`Origin` comme entité du graphe**, avec `local_path` par poste. Débloque
   le cloud, et la fiche de promotion du [doc 16](../25-aout-2026-18h58/16-le-monde-est-ouvert.md)
   (« tu touches souvent à ce dépôt, je l'ingère ? »).
3. **Honorer `Consistency`**, et corriger `flush_insertions` qui écrit sans
   indexer. Déclaré depuis longtemps, jamais tenu — c'est la plus vieille
   dette de la maison.
4. **Le fil à plus de deux** : un identifiant de conversation dans
   l'enveloppe. Petit, et il débloque « plusieurs humains, deux agents ».
5. **Le partage entre projets d'une org** par `ExportableStats` — lucivy a
   livré `search_filtered_with_global_stats`, on ne s'en sert pas encore.
6. **Le TZif**, si un jour on veut afficher une heure locale sans que le
   lecteur déclare son décalage.

## 4. Ce qui a le mieux marché, et qu'il faut continuer

**Les canaris.** Des tests qui *affirment un défaut*, avec le mode d'emploi
de leur propre mort. Trois sont tombés en vingt-quatre heures, dont un écrit
par une session précédente qui disait quoi faire quand il tomberait.

**Mesurer plutôt que déduire.** Chaque fois qu'on a cru savoir, on s'est
trompé : le masque HNSW « qui fuit » (il ne fuyait pas, il n'était pas lu),
la lenteur « des embeddings » (c'était l'index), le poste qui rame (c'était
le zram, pas le CPU).

**Et la phrase de lucivy**, qui vaut pour nous :

> Une explication convaincante d'un artefact est plus dangereuse que
> l'artefact.

## 5. Les documents, par sujet

| Sujet | Où |
|---|---|
| Vision, chaos contrôlé | `25-aout/01`, `vision_roadmap_08_2026/` |
| Feuille de route | `25-aout/06`, `25-aout/08` |
| Relations à travers les lots, couche `Symbol` | [`25-aout/17`](../25-aout-2026-18h58/17-relations-a-travers-les-lots.md) |
| Index vectoriel, coût, différé | [`25-aout/18`](../25-aout-2026-18h58/18-index-vectoriel-differe.md) |
| Identité d'un fichier | [`25-aout/15`](../25-aout-2026-18h58/15-identite-d-un-fichier.md), [`26-aout/04`](../26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md) |
| Monde ouvert, politiques de lecture | [`25-aout/16`](../25-aout-2026-18h58/16-le-monde-est-ouvert.md) |
| Session comme graphe, poignées | [`25-aout/13`](../25-aout-2026-18h58/13-la-session-comme-graphe.md) |
| Tout est écoutable | [`25-aout/14`](../25-aout-2026-18h58/14-tout-est-ecoutable.md) |
| Origine / cellule / domaine | [`26-aout/05`](../26-aout-2026-20h29/05-origine-cellule-domaine.md) |
| Cahier des charges lucivy, et leurs réponses | [`26-aout/06`](../26-aout-2026-20h29/06-cahier-des-charges-lucivy-partage.md), `07`, `08`, `09` |
| Outils asynchrones | [`26-aout/10`](../26-aout-2026-20h29/10-outils-asynchrones.md) |
| Se taire, raccrocher, conversations | [`26-aout/11`](../26-aout-2026-20h29/11-le-droit-de-se-taire.md), [`12`](../26-aout-2026-20h29/12-conversations-a-plusieurs.md) |
| Commandes, mémo | [`26-aout/03`](../26-aout-2026-20h29/03-commandes.md), et [`03`](03-knowledge-dump.md) ici |
