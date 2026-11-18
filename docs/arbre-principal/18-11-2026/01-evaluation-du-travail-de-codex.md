# Arbre principal : l'évaluation du travail de Codex (16 nov. 11 h → 17 nov. 21 h)

18 novembre 2026, à la reprise, au mot de Lucie relayé par c7. Relevé en lecture seule : `git` sur master
(`927941a67a..99fef92ce6`, 76 commits) et sur les branches `origin/cx-arbre-*` du 16 et du 17 ; `du` et `stat` aux
deux postes ; aucune base ouverte, aucune passe jouée.

## Ce qu'il a fait, en clair

- **Sur master, dans mon périmètre : rien.** Aucun des 76 commits ne touche les vues, les générations, les suivies,
  la migration ni `docs/arbre-principal/`. Ceux qui touchent la bascule sont de G (le plan, la pose de la lib
  59ac9c86bc, l'outil de retour d'un instantané).
- **Sur 18 branches `cx-arbre-*` (141 commits, ~19 000 lignes, rien sur master)** : un harnais de recette pour
  convertir les bases. Ce sont des pages, des scripts Python, deux petits `.cpp` et deux `.rs` de lecture, et des
  manifestes de preuves. Aucune ligne de code du produit.
- **La bascule des trois bases de Lucie n'a pas eu lieu.** La fenêtre sur les vraies bases a été annulée le 16 à
  13 h 34. Les trois bases vivantes n'ont pas été écrites depuis le 16 midi (mtimes relevés) ; elles sont toujours de
  la forme d'avant.
- **Une conversion verte sur une copie** de code-principal (la « v9 ») : 97 247 vecteurs, aucune dette. C'est le même
  constat que ma répétition du 15, faite en 2 h avec un script de 120 lignes ; elle a demandé ici neuf essais et neuf
  branches.
- **« CLI99 »** : un binaire `rag3weaver-backend` privé, bâti depuis master 99fef92ce6, pour jouer `migration versions`
  et `migration generations` sur une copie. Ce n'est pas l'hôte de G (927 et 59ac) : c'est un exécutable à côté. Il a
  été bâti et contrôlé statiquement (« A »), jamais exécuté sur une base (« B » préparé, non joué ; « C », les trois
  bases, non fait).
- **A1 n'a pas été touché** : `arbre-principal/scope-chunks-des-sondes` (2e0f986825) est intact, et ses docs disaient
  « A1 suspendu ».

## 1. Le tableau

| Quoi | Où | Verdict | Pourquoi |
|---|---|---|---|
| Le plan de la bascule, la pose de la lib 59ac (G) : `806715b2d6`, `dc32e7073a`, `58798c9c22`, `f83c6c4dd6` | master | **utile, à garder** | La lib et les hôtes sont posés. Il dit honnêtement « FINI lib/pose ne signifie pas FINI migrations ». |
| L'outil de retour d'un instantané (G) : `3db06a9d48` … `57fbd9aa56` | master | **utile, à garder** | C'est le prérequis du retour de 72 h. Un rouge précède chaque correctif. |
| `cx-arbre-bascule-bases-20261116-r1` (fdf62741ba) | branche | **utile mais à requalifier** | Trois constats réels sont à sauver en tickets courts, sans le harnais : (1) `POST /backend/maintain` en essai à blanc refusé en **HTTP 422** au portable, de cause inconnue ; (2) la fermeture de code-principal refusée ; (3) les 469 morceaux sans vecteur après 1b. |
| `cx-arbre-audit-ro-dette`, `dette-copie-a`, `fin-ro-preuves`, `fin-ro-dette` (16 nov.) | branches | **utile mais à requalifier** | Leur conclusion est juste, mais elle tient en une ligne. Les 469 morceaux ont un découpage neuf, sans texte ancien compatible : c'est une dette normale, que le rattrapage ou le repassage paie. Ce n'est pas un défaut de 1b. Le ticket « cohorte 469 » se ferme sur ce constat. |
| `cx-arbre-cycle-*` (b5, ready-borne, v4, producer-controller, finaliseur-borne, v8, v9, v9-precisions) et `cx-arbre-contrat-postwriter` | branches | **inutile** | Un orchestrateur Python de « cycle de vie » (fermer, migrer, rouvrir), avec superviseur, gardes readonly et postwriter, et neuf essais. Les défauts qu'il trouve sont les siens. La bascule est une suite de cinq commandes, que mon script de répétition et la page du 15 décrivent déjà. Les tickets « garde readonly après le writer » et « cycle avant l'armement du producteur » sont caducs avec lui. |
| `cx-arbre-prepa-trois-bases`, `cli99-static-a-preuves`, `cli99-runtime-b` | branches | **inutile** | Un second binaire privé et un lecteur instrumenté sous GDB, pour une exécution que le binaire de master fait déjà. Les défauts R1 et R2 sont ceux de ce lecteur, pas du produit. |
| Le sujet de `c34f16373e` (« Transmettre le chantier arbre-principal à Claude… ») | branche | **nuisible, petit** | Il nomme un assistant dans un message de commit, contre la règle. Il est sur une branche, pas sur master, et ne se réécrit pas (pas de force push) : il reste tel quel. |
| Les copies sous `~/.cache/rag3weaver-build/bascule-live-20261116/` | **19 Go** au portable, **39 Go** à luciepc | **inutile, à retirer** (au ménage, au mot de c7) | Ce sont des copies de travail du harnais : `cold-*`, `immutable-*`, `id-sets-*`, `fresh-*`, `lifecycle-copy-v1/v4/v8/v9`, `debt-*`, `migration-rehearsal-*`. Aucune ne sert à la bascule réelle. |
| Les instantanés de G du 16 (`avant-59ac9c86b-20261116-927`, sa variante `-controle`, `diagnostic-code-principal-20261116-122933`) | `~/.local/share/rag3weaver/instantanes/` | **à garder** | Ils sont à G, et ils sont le retour tant que la bascule n'est pas faite. |

Identité : les 76 commits de master et les 141 des branches sont en gmail. Le seul manquement est le sujet de
`c34f16373e`.

## 2. Le verdict d'ensemble

1. Dans mon périmètre, Codex n'a rien cassé : aucun code de produit, rien sur master, les trois bases intactes.
2. Il n'a pas non plus avancé la bascule : elle est au même point que le 16 au matin, plus la lib 59ac posée par G.
3. Il en sort trois constats utiles : le `maintain` refusé en 422, la fermeture refusée, et la dette des 469
   morceaux, qui est normale et que le repassage paie.
4. Le reste est un harnais de qualification (~19 000 lignes, 18 branches, 58 Go de copies) très au-delà de la
   recette, qui tient en cinq commandes déjà répétées vertes le 15.
5. Il faut retirer les copies, fermer ses tickets d'outillage comme caducs, et reprendre la bascule par la recette
   courte, avec le 422 en préalable.

## 3. Ce que ça change à ma page du 16 (`16-11-2026/02-passation-pour-codex.md`)

- **La bascule (partie 3)** gagne un préalable : comprendre le **HTTP 422** de `maintain` en essai à blanc au
  portable. Le ticket est à ouvrir chez lucivy et G, avec la recette du refus (sa capture est dans
  `cx-arbre-bascule-bases-20261116-r1`).
- **Le ménage (B4)** s'étend aux 58 Go de `bascule-live-20261116`, en gardant les instantanés de G. Mes propres
  copies du 14 et du 15 (`1b-copie` : 48 Go au portable et 8,9 Go à luciepc ; `bascule` : 45 Go à luciepc) y étaient
  déjà.
- **A1** reprend sur le master courant, par moi, depuis 2e0f986825. La lib posée (59ac) dispense désormais de
  `RAG3WEAVER_MOTEUR_ANCIEN`.
- Les autres entrées (A2 à A7, B, C) sont inchangées : Codex n'en a commencé aucune.

## 4. La lisibilité de sa passation

**Mauvaise**, pour Lucie comme pour une session neuve.
`docs/arbre-principal/17-11-2026/99-passation-pour-claude.md` est un relevé de preuves, pas une passation :
- un jargon jamais défini : coupons, GO, END, RO, Qv8, Qv9, A, B, C, R1, R2, « CPU12 », « proposition900 » ;
- des mots collés sans espace : « Sourcehôte/CLI927941a… », « 102002ancienschunks » ;
- des empreintes SHA à chaque ligne ;
- aucune phrase qui dise ce qui est fait pour l'utilisateur, ni pourquoi.

On n'y apprend qu'à la fin, et en passant, que les trois bases n'ont pas été migrées. Une passation dit cela en
première ligne.
