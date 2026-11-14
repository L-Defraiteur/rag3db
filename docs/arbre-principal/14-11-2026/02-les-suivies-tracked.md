# Les suivies (`tracked`) : la déclaration, la résolution, et l'identifiant stable d'une couche

14 novembre 2026, arbre principal. C'est la généralisation des branches suivies que Lucie a demandée
(« tout paramétrable, une abstraction où même des cas pas prévus marchent »). La synthèse est dans
`docs/orchestration/14-11-2026/02-les-sources-suivies-une-abstraction-pour-les-couches.md`. La
conception a été validée par c7 et relue par ED. Le code est dans `extension/rag3weaver/src/suivies.rs`,
branche `arbre-principal/suivies`.

## Le champ du manifeste

```json
"tracked": {
  "root": "computed",
  "sources": [
    {"select": "worktree:backend"},
    {"select": "tasks"},
    {"select": "branch:develop", "parent": {"declared": "main"}},
    {"select": "refs:refs/heads/release/*", "history": {"last": 5}},
    {"select": "commit:4f6f80ef4", "parent": "none"}
  ]
}
```

- **`root`** : `"computed"` (défaut), `"empty"` ou `{"declared": "<rev>"}`.
- **`select`**, préfixé : `branch:<nom>`, `refs:<motif for-each-ref>`, `worktree:backend`,
  `worktree:*`, `worktree:<chemin>`, `commit:<rev>`, `tasks`.
- **`parent`** : `"inferred"` (défaut), `"none"` ou `{"declared": "<branche>"}`.
- **`history`** : `"tip"` (défaut), `"all"`, `{"last": N}` ou `"cited"`.
- **`work`** : vrai par défaut pour un worktree et pour les tâches, faux sinon ; vrai sur une source
  sans worktree est un refus.
- **Le défaut sans `tracked`** : `worktree:backend` et `tasks`, avec la racine calculée.
- **Les refus** : toute clé ou valeur inconnue est un refus qui la nomme.
- **L'ancienne clé** `branches_suivies:<dépôt>` se lit encore (son ancre devient une racine déclarée)
  et ne s'écrit plus.

## Ce qui est livré (verts, rouges d'abord)

1. **La lecture de la déclaration** (dbbae6668 → 3621d97d0).
2. **La résolution des sélecteurs** (33bc61983 → 91ae53888) : un `for-each-ref` et un
   `worktree list`.
   - Les refs par leur nom complet.
   - Le motif jugé contre la sortie de git lui-même.
   - Une couche de travail par worktree, nommée par son chemin.
   - Un sélecteur vide se dit.
3. **La pile** (f6d28a2bf → 6f85088f2, puis 352ae8617 → 32cc1fbd7).
   - **La racine** : calculée (`merge-base --octopus`, avec les suivies qui la fixent), vide, déclarée,
     introuvable, ou sans ancêtre commun.
   - **La parente inférée, parmi les suivies et sans ancre** : la fourche la plus récente. Elle est
     admise seulement si elle est unique ; à égalité de pointes, c'est « égales ».
   - **Le flux** : un cycle de fusions de premier parent entre des pointes qui ne se contiennent pas.
     Il est nommé, et ses couches restent sans parente inférée.
   - **Une fonctionnalité finie** (contenue dans la branche qui l'a fusionnée) a celle-ci pour
     parente et ne fait pas de flux (décision de c7).
4. **L'identifiant du dépôt et la migration des clés** (74111fd35 → c1a2bac3b).
   - L'identifiant est la racine du plus vieux commit des historiques suivis (date d'auteur ; à
     égalité, les racines sont nommées et la première par hash décide).
   - `couche:` et `couche_fermee:` migrent en nom complet. Une cible déjà posée n'est jamais écrasée
     (la garde de c7).
5. **Le résolveur et son cache** (eede6cc32 → b3bd55d88) : sans mouvement de refs, la résolution entière
   vient du cache, avec deux appels git et aucun merge-base.

## La décision : un identifiant stable de couche, distinct de son nom (ED et arbre principal, 22 h 30)

Les lignes neuves de 2a′-3 portent leur couche dans leur uuid (`[clé, couche, _depuis]`). Il fallait que
les lignes déjà posées ne se recalculent ni ne se dédoublent. Deux voies ont été pesées :
- **(a)** un `_couche` au nom de la suivie, avec la relecture de l'ancien nom. C'est un double
  prédicat, jusqu'à une migration ;
- **(b)** un **identifiant stable de couche**, distinct de son nom d'affichage.

**Retenu : (b).** Il n'y a qu'un seul prédicat de visibilité, aucun uuid existant ne bouge, et le
ticket « le nom comme étiquette » se ferme. **Un renommage de branche ne termine plus une couche** : il
ne change que son nom d'affichage (ED retire sa règle d'hier). Les trois précisions d'ED :

1. **L'id se dérive de la naissance de la couche**, au lieu d'être tiré au hasard :
   - pour une couche d'histoire : `id = hash(identifiant du dépôt, nom complet à la naissance, commit de
     naissance)`, où le commit de naissance est le premier commit publié dans la couche ;
   - pour une couche de travail : `hash(identifiant du dépôt, chemin du worktree à sa naissance,
     commit extrait)`.

   L'id est **gardé** dans `_catalog_meta` (`couche:<dépôt>:<id>` → `{nom courant, naissance,
   parente}`, plus l'index nom → id), et il est **recalculable**. Deux copies de la base, ou une base
   reconstruite, retrouvent les mêmes id sans se coordonner. Une branche effacée puis recréée sous le
   même nom depuis un autre commit reçoit un id neuf : son histoire ne se mêle pas à celle de
   l'ancienne.
2. **La couche du backend garde l'id `COUCHE_DU_BACKEND`** (`backend`), posé tel quel et non dérivé.
   C'est la seule exception, pour que rien de 2a′-1 ne bouge. Son nom d'affichage devient celui de la
   suivie du worktree du backend.
3. **Un id ne se réutilise jamais pour une autre lignée.** Une couche finie, ref effacée, garde son id
   dans `_catalog_meta` avec sa fin, tant que la rétention garde ses lignes. La recherche nom → id ne
   rend que les couches vivantes.

**Le coût** : un tour par `_catalog_meta` par résolution, en cache avec la résolution. Sous 2a′-3, ED lit
l'id par la suivie résolue, jamais par le nom de la ref ; il l'écrit dans sa page 03.

**Ce que ça change à ce lot** : les clés d'aujourd'hui (`couche:<dépôt>:<ref>` → parente estampillée,
`couche_fermee:`) deviennent l'index nom → id plus la fiche de la couche sous son id. La migration des
clés (le pas 4) se prolonge en conséquence ; la garde reste.

## Ce qui reste

- **L'identifiant stable des couches** : la dérivation, la fiche, l'index nom → id, la fin.
- **Le branchement à la place de `couches.rs`** : l'ouverture et la fermeture des tâches,
  `index_states`, et la migration appelée à la résolution qui observe un identifiant neuf. Seul
  `couches.rs` lit les clés `couche:`, `couche_fermee:` et `branches_suivies:` (relevé par grep).
- **Les témoins par ligne du tableau de la page 02.**
- **La passe**, avec e2e_lectures.
