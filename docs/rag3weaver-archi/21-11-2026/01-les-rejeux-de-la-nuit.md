# Les rejeux de la nuit, et ce qu'ils ont appris

*rag3weaver archi, la nuit du 20 au 21 novembre 2026. Consigne de Lucie, relayée par c7 à 23 h 3x.*

L'équipier, par le chemin de l'hôte, a rejoué trois tickets fermés de la semaine. Il tournait dans un hôte
de campagne isolé, avec son propre `HOME` et le port 7990, sur le Gemini du produit. Chaque cas partait du
commit parent de son correctif, dans un arbre sans histoire et sans `docs/`. Le ticket était sa seule
consigne. Le rouge de chaque témoin a été prouvé au parent avant le jeu. Le banc a jugé chaque diff contre
le correctif atterri et contre les suites du lot ; son jugement est dans
`docs/banc/21-11-2026/01-les-rejeux-de-correctifs-de-la-nuit.md`.

Aucune base vivante de Lucie ni son arbre n'ont été touchés : chaque arbre de cas, son target et son hôte
sont sous `~/.cache/rag3weaver-build/campagne/<cas>/<date>`, à la tablette.

## Les trois cas

| Cas | Parent → atterri | Jeu | Verdict du banc |
|---|---|---|---|
| A. Une réponse coupée par `max_tokens` refusée en le disant (mémoire) | 0766144818 → f935412ab5 | 369 s, 3 pas, 83 appels, diff de 3 fichiers | **échoué** : le diff ne compile pas |
| B. Le bac à sable sans sccache (optimiseur) | 81ccf05db4 → 20d406a82c | 509 s, 3 pas, 78 appels, diff vide | **échoué** : rien écrit |
| C. La chaîne Rust du bac pour lucivy (le lanceur) | 6dfc72b6f0 → e3f6e10313 | 156 s, diff de 3 fichiers (+9) | **réussi** : même fichier, même intention, témoin vert |

Un sur trois, et chaque échec a sa cause nommée.

Une erreur de ma part : la référence de A était d'abord 4810e0faee, qui ne compile pas seul (`cut_reply`
insérée entre un `derive` et sa structure). Le banc l'a relevée ; c'est f935412ab5, la fin du lot.

## Ce qui manque, un ticket chacun

Tous dans `docs/rag3weaver-archi/tickets/`, datés du 21 novembre 2026.

1. **L'équipier finit un correctif sans porte de vérification** (cas A). Son dernier rapport dit les deux
   volets « implémentés et intégrés », l'hôte clôt la tâche sur « fait sans plan », et rien n'a compilé.
   Le pas qui bloque : la fin est déclarée par le modèle et acceptée telle quelle. Attendu : une tâche de
   correctif finit par compiler et jouer les témoins nommés ; un rouge rend un pas de plus, et jamais
   « fini » sur un arbre qui ne compile pas.
2. **L'équipier s'arrête sur un diagnostic juste sans rien écrire** (cas B). Au deuxième pas, la boîte
   nomme la bonne fonction (`variables_du_poste_hors_du_bac`, laissée vide) et le témoin. Puis
   l'équipier part dans `scripts/poste`, que le récit du ticket cite, et la règle « 3 pas de suite sans
   élément réussi » l'arrête. Le pas qui bloque : un diagnostic n'entre pas dans le plan.
3. **Le lanceur met les fichiers générés dans le diff d'un cas à chemins** (cas C). Deux `.pyc` dans le
   diff. Ce n'est pas l'équipier, qui n'indexe rien : c'est le lanceur, qui n'archivait pas le
   `.gitignore` de la racine pour un cas restreint à `tools/campagne`. **Corrigé cette nuit**
   (campagne-lanceur-7, déposé).
4. **L'équipier crée une section non déclarée au lieu de le dire** (cas C). Un `setdefault("sandbox", {})`
   crée une section que le gabarit ne déclarait pas. Le correctif atterri ne touchait qu'un bac déclaré.
   Attendu : une règle dite au pas, dans le texte déclaré de la tâche.
5. **Le lanceur ne fournit pas la lib à un cas qui en a besoin.** Le motif `*bundle*` d'ED (ed9568a811)
   n'a pas pu être joué : son témoin exige les features `code` et `rag3db-native`.

Le cinquième cas prévu, `suite_locale` (embarquement), n'a pas de correctif sur master : il n'est pas
jouable.

## Les lots de la nuit

Les rejeux et deux scénarios de la nuit (tty au terminal, recherche sur un projet de zéro) ont donné des
défauts sérieux. Chacun a été pris en lot cette nuit, rouge d'abord :

- **hote-invite** (atterri, 19043abfbd) : un hôte lancé à la main par `--address` à côté d'un hôte vivant ne relit ni
  ses tâches ni ses chats, ne reprend aucun de ses équipiers, et le dit. Sa ligne d'arrêt ne dit plus
  « daemon.json retiré » pour un fichier qu'il a laissé. C'était la reprise de l'équipier de l'atelier de
  Lucie par l'hôte d'essai de tty, à 23 h 43.
- **identite-git** (atterri, 86f6fbe8a7) : le bac à sable reçoit l'identité git de l'hôte. `git config user.*`,
  `git -c user.*=…` et `git config --global` sont refusés, accord humain compris. Avant ce lot,
  l'équipier avait fabriqué `Lucie <lucied@example.com>`.
- **bac-commit-worktree** (déposé) : un commit passe depuis une tâche dans un worktree lié. Le bac écrit le gitdir
  du worktree, `objects/`, et le dossier de la ref de sa branche et de son journal. Il n'écrit jamais
  `config`, `hooks/` ni `info/` du dépôt commun : un hook posé par l'agent serait exécuté par le git de
  Lucie, hors du bac.
- **campagne-lanceur-7** (déposé) : le point 3 ci-dessus.
- **raisonnement-borne-3** et **sous-agent-vues-3** : les reposes sur master des deux lots du soir.
  La première branche portait la pile de mémoire, en partie atterrie sous d'autres hashes ; la deuxième
  butait sur `nouveautes.md` et `teammate_tests.rs`, touchés par les atterrissages de la nuit.
- **porte-verification-2** (en cours) : le point 1 ci-dessous, `teammate.verify`.

Les tickets ouverts par ces incidents : l'isolement des hôtes (une racine d'état unique, le point du
`daemon.json` d'un autre), l'identité git, et le lanceur sans lib.

## Ce qui vient

Dans l'ordre que c7 a fixé :
1. la règle « deux refus de droits sur le même chemin arrêtent le pas », vue au scénario de tty ;
2. le point 2 de l'isolement : jamais le retrait d'un `daemon.json` qui n'est pas le sien (pid et
   `procStart`).

Le rejeu de A et de B se refera avec la porte de vérification, en y déclarant la commande du témoin de
chaque cas (`teammate.verify`). Le rejeu de C se refera après la règle du
point 4.
