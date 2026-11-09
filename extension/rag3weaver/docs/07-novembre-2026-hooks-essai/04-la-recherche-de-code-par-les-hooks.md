# La recherche de code par les hooks (conception)

*hooks-essai, 9 novembre 2026, à la demande de c7 ; page de conception pour Lucie, avant tout code.*

## Le constat

Nos sessions se servent de la mémoire par les hooks : 461 appels dans la mesure de c7. Elles ne se
servent jamais des outils de code de l'index : 0 appel de `search_code`, `usages` ou `outline` sur
500 runs.

Ma propre session, hooks-essai, compte depuis le 7 novembre :
- 1 255 appels Bash ;
- 485 `grep` dans `src/` ;
- 218 lectures de fichiers Rust par `sed -n` ;
- **0** appel aux outils `mcp__code-rag3db__*`.

Trois raisons, lues dans la session plutôt que supposées :
- les outils de code y sont **différés**, et il faut un `ToolSearch` pour les charger ;
- rien ne les rappelle au moment où l'on cherche ;
- `grep` donne la réponse attendue du premier coup, si bien qu'on n'a jamais de raison d'essayer
  autre chose.

La mémoire, elle, arrive sans qu'on la demande, et c'est pour cela qu'elle sert. L'idée est de
faire pareil pour le code : ne pas compter sur la session pour y penser, mais lui apporter l'index
au moment où elle en a besoin.

## Ce que je propose

Trois pas, chacun mesurable seul, du moins coûteux au plus coûteux.

### 1. `SessionStart` : dire que l'index existe, et comment s'en servir

Si le dossier de travail de la session est sous la racine d'un arbre de code inscrit (les
`~/.config/rag3weaver/code-*`, dont la `workspace.root` contient le `cwd`), le bloc de session
ajoute une ligne :

> Ce dépôt est indexé (code-rag3db, master, à jour) : `search_code` (mots et sens), `usages`,
> `outline`. Ils sont différés : `ToolSearch select:mcp__code-rag3db__search_code,…`.

Ce pas ne fait aucun appel à l'hôte en plus (l'état de l'index est déjà dans `/etat`), et il
coûte environ 60 jetons, une fois par session et à chaque compaction.

### 2. `UserPromptSubmit` : glisser le code que le prompt nomme

Quand le prompt **nomme du code**, le bloc du tour ajoute une section « Code en rapport ». On
reconnaît un nom de code à sa forme :
- un chemin avec `/` et une extension (`src/daemon_host/taches.rs`) ;
- un nom qualifié (`Backend::call`) ;
- un identifiant entre accents graves ;
- un `snake_case` ou un `CamelCase` de plus de 6 lettres.

La section est faite ainsi :
- **Ce qu'on y met** : les trois premiers résultats de `search_code` (chemin:ligne et la ligne
  elle-même), et pour un symbole, le nombre de ses usages avec les deux premiers.
- **Comment on cherche** : par le **plein texte seul**, sans embarquer la requête. C'est la leçon
  du 8 novembre : un embarquement en file emporte le tour. Il n'y a donc pas de dépendance au
  service d'embarquement.
- **Les bornes** : un budget de 600 ms et 600 caractères, à côté du rappel (1 s, 1 500 caractères
  pour tout le bloc). Le déjà-vu par session vaut aussi ici : le même résultat n'est pas redit
  deux tours de suite.
- **Quand on se tait** : si le prompt ne nomme rien, si le `cwd` n'est sous aucun arbre indexé ou
  si l'index est absent.

### 3. `PostToolUse` : après un `grep` ou un `Read` sur le dépôt indexé, une ligne

Le hook voit déjà passer chaque outil (`PostToolUse`). Après un `Bash` qui lance `grep` ou `rg`
sous la racine indexée, ou après un `Read` d'un fichier indexé, il ajoute une ligne. Par exemple,
pour `grep -n "fn ouvrir_une_tache"` :

> l'index sait répondre : `usages ouvrir_une_tache` → 4 appelants (teammate.rs:1066,
> taches/tests.rs:73…)

Une seule ligne au plus tous les cinq outils, et une seule fois pour un même motif. C'est pour
qu'on l'essaie, pas pour la remplacer.

Pourquoi pas `PreToolUse` : il passerait **avant** chaque outil, donc dans la latence de chacun, et
il peut refuser un outil, ce qui serait une conception plus lourde. `PostToolUse` est déjà
branché, il ne retarde rien d'utile, et il suffit pour donner l'envie d'essayer.

## Ce que ça coûte par tour (à confirmer par la mesure)

| pas | appels à l'hôte | temps ajouté | jetons |
|---|---|---|---|
| 1, SessionStart | 0 | ~0 | ~60, une fois |
| 2, prompt qui nomme du code | 1 à 2 (`search_code` plein texte, `usages`) | à mesurer, budget 600 ms | ≤ 150 |
| 2, prompt sans code | 0 | ~0 (une regex) | 0 |
| 3, grep ou read sur le dépôt | 1 (`usages` ou `search_code`) | à mesurer, hors du chemin de la réponse | ≤ 40, au plus tous les cinq outils |

Je n'ai pas encore chiffré `search_code` en plein texte seul par l'hôte du portable. Le rappel des
fiches, qui lui embarque sa requête, prend 124 à 168 ms quand le service est libre, et
`recall_apports` 50 à 90 ms. Je m'attends à moins de 200 ms pour le plein texte seul, mais c'est la
première chose que la mesure dira.

## Comment on le mesure, sur hooks-essai d'abord

1. **La ligne de base, déjà relevée** (ci-dessus) : grep et lectures par sed par heure, et zéro
   appel aux outils de code.
2. **Un réglage par session**, `code_recall: true` dans le `hooks.json` local de hooks-essai
   seulement, jamais dans le global. Chaque pas peut se couper seul.
3. **Ce que le journal des hooks consigne déjà**, plus trois lignes : la section ajoutée ou tue (et
   pourquoi), sa taille, la durée de chaque appel.
4. **Ce qu'on compare sur une journée de travail réel** :
   - les appels `mcp__code-rag3db__*` par heure (aujourd'hui 0) ;
   - les `grep` dans `src/` par heure (aujourd'hui environ 10) ;
   - la latence p50 et p95 du hook `UserPromptSubmit` (aujourd'hui environ 60 ms hors rappel) ;
   - la part des tours où la section « Code en rapport » a paru, et la part où la session a
     ensuite utilisé ce qu'elle disait (un fichier ou une ligne cités, repris dans l'outil
     suivant).
5. **Le seuil pour l'étendre aux autres sessions** : au moins un appel aux outils de code sur cinq
   tours où la section a paru, une latence p95 du hook sous 1 s, et aucune plainte de bruit.
   Les bilans du juge ont montré qu'un rappel à côté du sujet coûte plus qu'il ne rapporte.

## Les risques

- **Le bruit**, la leçon des bilans du juge : on ne déclenche que sur une forme de code, on se
  tait sinon, et le déjà-vu s'applique.
- **Un index en retard sur l'arbre de travail** : un worktree non indexé, ou une modification non
  commitée. La section dit l'état de l'index (`à jour`, `en cours`) et ne cite jamais une ligne
  qu'un fichier modifié depuis aurait pu déplacer, sans le dire.
- **La latence** : le plein texte seul, un budget court, et le reste en fond comme le rappel.
  Aucun embarquement dans le chemin du tour.

## Questions pour Lucie

1. Les trois pas, ou seulement le 1 et le 2 d'abord ?
2. Le 3 en `PostToolUse`, ou préfères-tu `PreToolUse`, avant l'outil, plus visible mais dans la
   latence de chaque outil ?
3. La mesure d'une journée sur hooks-essai seule, puis la décision d'étendre : est-ce que ça te
   va ?
