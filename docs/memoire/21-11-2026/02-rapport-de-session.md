# Session mémoire (96) : note de pause du 21 novembre 2026

Note écrite avant le redémarrage de la tablette (Lucie sort /tmp de la RAM), à la demande de c7. Elle dit où la
session en est, pour la reprise par `claude --resume`.

## Ce qui a atterri aujourd'hui

- **La pile mémoire, en entier et dans l'ordre :**
  - template-version (4857134014) ;
  - graphs-on-disk (9b72078216) ;
  - check-proposal (35f52d906e) ;
  - b4-decide (10022ebe28) ;
  - script-ids (f2fb037a40).
- **fetch et web_search de l'équipier** (06951d0509).
- **La clé d'agent d'un équipier**, `Tache.agent_key` (e7ff77cbaf).
- **Les descriptions de chaque paramètre du serveur mémoire** (04a15e8604).
- **La page « ce que cet agent sait »** (docs/memoire/21-11-2026/01), et le ticket « l'équipier écrit sa mémoire mais
  ne la relit jamais » (a03932d6c3).

## En file au pousseur

**memoire-attribution** (1b, `author` posé par l'hôte, M7), branche `cx-memoire-attribution`, tête f9c3b96381.

- **Vert** : lib 2888, 0 échec ; les dix suites ; les 113 tests du crochet et de l'équipier ; `CHECK_FEATURES VERT 8/8`
  à la tête exacte (journal `~/.cache/rag3weaver-build/attest-author.log` à luciepc). Témoin :
  `a_call_made_for_an_agent_signs_its_rows_with_the_agent_key`, suite `e2e_archiviste`.
- **À corriger d'abord à la reprise** : le pousseur relève un identifiant français neuf, la méthode `appeler_pour`
  du trait `Hote` (`src/claude_code_hook.rs:314`). Le rappel de Lucie (11 h) veut tout identifiant neuf en anglais :
  - renommer en `call_for` ;
  - rejouer `claude_code_hook::` et `teammate_tests::` par un `cargo test --lib` direct ;
  - réattester ;
  - redéposer sous le même id.

## Ce qui tourne à luciepc

Rien de la session 96. Les scripts de passe (`lot-96.sh`, `suites-96.sh`, `lib-96.sh`, `attester-96.sh`) sont dans
`~/.cache/rag3weaver-build/`, et l'arbre de passe est `~/git_workspaces/rag3db-p1r-96`.

## Branches et worktrees tenus

- `memoire/21-11-2026-attribution` (cx-memoire-attribution, f9c3b96381) : le lot en file.
- `memoire/21-11-2026-descriptions` (cx-memoire-descriptions-r2, b6cd45fbc2) : lot atterri, worktree à retirer.
- `memoire/21-11-2026-page-vue-agent` : la branche de cette note, `cx-memoire-note-pause`.
- À luciepc : `rag3db-p1r-96`, et ses targets `4ad3`, `nofeat`, `p1r-96-attest` et `p1r-96-nav`.

## La file, après 1b

1. **Noms lisibles et liens des fiches.** Le miroir `fiches/{uuid}.md` devient `<sujet>/<clé-slug>.md`, et les refs et
   `cited_by` entrent dans le fichier. Il faut une migration de format 1 → 2 pour les 197 fichiers de
   `~/memoire-longue/fiches`, sur une copie d'abord.
2. **Garde-fou de `put_fiche`** : rendre les fiches proches au-dessus d'un seuil, et ne créer que sur confirmation.
3. **Clé de session MCP** : relier une session Claude Code (son `claude_name`) à sa clé d'agent.
4. **Promis ensuite** :
   - le masquage des sorties d'outils (étage 0.1 de la page d'archi sur le mode auto) ;
   - les autorisations rangées en mémoire (étage 3, catégorie `authorization`) ;
   - 5.3, la parité du banc par une tâche ;
   - la copie du gabarit memory de Lucie, qui suit le binaire sur le mot « essai fini » de c7, la copie gardée en
     `.avant-21-11-2026`.

## Prochaine action

À la reprise, dans l'ordre :
1. relire `daemon_status` ;
2. attendre l'annonce de c7 pour `confiner-poste-classes` avant toute validation ou e2e ;
3. renommer `appeler_pour` en `call_for` et redéposer 1b ;
4. retirer le worktree des descriptions ;
5. commencer « noms lisibles et liens » par la relecture du gabarit de la fiche (`gabarits/fiche.md.jinja`) et de
   `fichiers_entite.rs`, sans rien lancer de lourd.
