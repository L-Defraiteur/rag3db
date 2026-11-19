# La répétition de la bascule, et la recette du soir

19 novembre 2026, arbre principal, à la demande de c7. Elle est faite avant la fenêtre de G du soir, qui bascule vers
les versions les trois bases de Lucie : `code-rag3db-depot`, `code-lourd` et `code-principal`.

## Ce qui a été joué

Le matériel :

- **La copie de départ** : la copie fidèle de `code-rag3db-depot` faite par G, l'instantané du portable de 10 h 04,
  hôte arrêté. Elle est prouvée par `index_state` : le plein texte a autant de documents que de lignes (File 6 508,
  Library 400, Scope 68 585, Symbol 75 745), et les vecteurs sont prêts à 100 %.
- **Deux reflinks** :
  - A, l'ancienne forme ;
  - B, migrée.
- **Leurs manifestes** sont repointés, sans `models.llm`, les réactions et la veille retirées. L'extension notée est
  repointée sur la lib 7c2b506ec.
- **Le binaire posé** : `~/.local/bin/rag3weaver-backend`, sur la lib 7c2b506ec.

Les durées ci-dessous sont un **ordre de grandeur**, pas une mesure : le poste était chargé, sous `poste lourd`.

| étape | durée |
|---|---|
| `migration versions --base` (1b) | 667 s, 11 min |
| `migration generations --base` | 1,3 s |

## Le verdict : vert

- **`index_state` de B** :
  - l'avertissement « base de code sans versions de scope » a disparu ;
  - le plein texte a autant de documents que de lignes, avec les mêmes comptes que A ;
  - les vecteurs de Scope sont à 98 %, « running ». C'est la dette de 1b, normale (`docs/tickets/closed/2026-11-18-la-dette-de-469-morceaux-apres-1b-est-normale.md`),
    que le repassage paie.
- **`usages`** : 4 requêtes (`publier_la_generation`, `load_stack_on`, `write_batch_by_generations`, `rechercher`).
  Les réponses de A et de B sont **identiques hors uuid**, définitions et usages compris (jusqu'à 66 ko de réponse).
- **`search_code`** : 4 requêtes, chacune en signaux par défaut (dense et BM25) et en BM25 seul.
  - Aucune réponse n'est identique au caractère près.
  - Les fichiers du top 10 de A se retrouvent dans celui de B à **96 % en moyenne** (de 90 à 100 %).
  - Le premier résultat est le même dans 7 requêtes sur 8.
  - Seule la requête 0 (dense et BM25) change de premier résultat. Elle passe par les vecteurs, dont 2 % sont encore
    dus sur B.
  - L'écart est celui de la forme à versions : le contenu y est dédoublonné, et environ 9 % des morceaux ont un
    découpage neuf.

## La recette du soir, base par base

C'est l'ordre de `docs/arbre-principal/15-11-2026/02-la-fenetre-de-la-bascule.md`, avec un changement à l'étape 2 :

1. **L'annonce** aux sessions qui lisent la base : le backend s'arrête pendant la fenêtre.
2. **`maintain` de lucivy par l'hôte, purge seule** :
   - d'abord, vérifier qu'aucun index ne tourne sur la base (`indexings_running == 0`) ;
   - puis `fts-live-ids`, la lecture du graphe, et `maintain` avec la purge des orphelins et **`compact: false`**.
   - **Aucune compaction**, jusqu'au pointeur du correctif de lucivy dans rag3weaver. Le défaut de lucivy est qu'une
     compaction tombée pendant un index perd des mots (signalé par c7, le 19 novembre).
   - La place n'est donc pas rendue avant la copie. L'instantané pèse la taille pleine de la base.
3. **L'instantané** : base, wal, extensions et `.fts` ensemble, avec le manifeste de copie. C'est le retour arrière,
   gardé 72 h.
4. **Le contrôle des deux sens** des tables de relations (check_rel_directions de G).
5. **1b** : `rag3weaver-backend migration versions --base <base>`. Ordre de grandeur : 11 min pour
   `code-rag3db-depot`.
6. **La montée aux générations** : `rag3weaver-backend migration generations --base <base>`. 1 à 2 s.
7. **La réouverture par l'hôte**, puis **le repassage**, base ouverte : `index` avec `confirm=true`, jusqu'à la
   dernière ligne de son journal. Compter 50 à 60 min de vecteurs par base.
8. **La vérification** :
   - `index_state` : le plein texte a autant de documents que de lignes, et les vecteurs remontent ;
   - les mêmes `usages` qu'avant ;
   - `search_code` sur des requêtes connues, avec le top 10 retrouvé.

**Le retour arrière** : rouvrir sur l'instantané de l'étape 3, en quelques minutes. L'ancienne base est gardée 72 h,
puis retirée base par base au mot de c7.

## Les traces, à luciepc

- `~/.cache/rag3weaver-build/mesures/repetition-bascule-20261119-1647/` : journaux de 1b et des générations, et les
  réponses de A et B.
- `~/.cache/rag3weaver-build/mesures/parite-search-20261119-*/` : la parité de `search_code`.
- Les copies `code-rag3db-{ancienne,migree}-{repetition,parite}-20261119-*`. Elles partent au ménage après la fenêtre.
  Les copies `…-1623-interrompue` viennent d'un essai arrêté en cours de 1b, et ne sont jamais relues.
