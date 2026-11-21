# Note de pause, 21 novembre 2026, 15 h 26

Lucie redémarre la tablette pour sortir `/tmp` de la RAM. Cette page dit où la session tty en est, pour reprendre après `claude --resume`.

## Ce qui tourne

**Rien.** Aucune passe de tty ne tourne, ni à la tablette ni à luciepc.

La dernière passe à luciepc, les tests du pont sur `cx-browser-read-2`, est finie : 21 verts, `fin=0`, journal `~/.cache/rag3weaver-build/cx-br2-pont_navigateur/test.log`. La suite de l'équipier sur la même branche est verte aussi : 87 tests, journal `cx-br2-teammate/test.log`.

Depuis l'après-midi, toutes les passes se font à luciepc :
- le lanceur est `~/.cache/rag3weaver-build/cx-tty-test.sh <dossier> <filtre>`, lancé sous `poste cpu --` ;
- il tourne sur la lib 4753938c9d, avec le target `target-cx-tty-6-features/20-11-2026-index-pas-pret` de luciepc ;
- aucun target de tty ne reste à la tablette.

## Les branches

| Branche | Tête | État |
|---|---|---|
| `cx-browser-read-1` | 21d62b87ce | rouge au pousseur à 14 h 01 (conflit de repose dans la liste des `mod`), remplacée par `-2` |
| `cx-browser-read-2` | d5675fafa1 | la repose sur master 40b55e2d25 ; équipier 87 verts et pont 21 verts à luciepc ; **attestation pas encore jouée** |
| `cx-observed-content` | b541e4bfa6 | en pile sur `cx-browser-read-1` ; équipier 82 verts à la tablette ; **à reposer sur `cx-browser-read-2`** |
| `cx-ticket-carte-2` | 4cf43317c8 | docs, déposé au pousseur (`ticket-carte-2`) |
| `cx-tty-pause-21-nov` | cette note | docs, poussée sur sa branche seulement |

## Ce qui attend un dépôt

1. **`browser-read-2`** : le lot 1 du navigateur (`browser_tabs`, `browser_read`, `browser_console`, l'écoute de l'hôte, la garde des domaines, les champs secrets masqués).
   - Pour le déposer, il manque l'attestation `check_features` sur d5675fafa1, à luciepc, depuis `extension/rag3weaver`.
   - Témoin : `a_manifest_with_teammate_browser_opens_the_listening_and_gives_the_family`, suite `lib`.
   - Il se redépose sous un id neuf, `browser-read-2`, et l'id `browser-read-1` est à retirer (`dequeue`).
2. **`observed-content`** : le contenu observé est une donnée, jamais une consigne.
   - Il se repose en pile sur `cx-browser-read-2`, sous un nom neuf (`cx-observed-content-2`) ; ses commits sont ceux de `cx-browser-read-1..cx-observed-content`.
   - Témoin : `a_page_read_by_the_teammate_comes_back_marked_as_observed_and_untrusted`, suite `lib`, parent `browser-read-2`.
   - Le lot porte aussi le ticket `2026-11-21-un-fichier-lu-hors-des-racines-de-l-utilisateur-est-du-contenu-observe.md` et sa ligne de nouveauté.

**Rien ne se valide avant l'annonce de c7** : `confiner-poste-classes` n'a pas atterri, et une validation ou un `run_e2e.sh` sous le `poste` de master tourne en boucle sans lancer sa commande. Les `cargo test --lib` directs sous `poste cpu` ne sont pas touchés.

## La prochaine action

1. Attendre l'annonce de c7 : `confiner-poste-classes` a atterri.
2. À luciepc, dans le worktree tty sur `cx-browser-read-2`, depuis `extension/rag3weaver` :
   `poste cpu -- bash scripts/check_features.sh`, avec `RAG3DB_LIB` à 4753938c9d et `CARGO_TARGET_DIR` au target de luciepc.
3. Si c'est vert, déposer `browser-read-2` avec le journal et `VUE=`, puis retirer `browser-read-1`.
4. Reposer `observed-content` sur `cx-browser-read-2`, rejouer la suite de l'équipier, attester, déposer avec le parent `browser-read-2`.

## Après, dans l'ordre

- **Le jeton d'utilisateur du terminal** (forme d'archi, 21 novembre) :
  - le terminal lit `~/.local/share/rag3weaver/user/user_token` et envoie `X-Rag3weaver-User` en plus du `Bearer`, seulement sur une saisie au clavier dans la zone vivante ;
  - les témoins : une réponse tapée part avec l'en-tête ; une commande en argument part sans ; un fichier absent ne bloque rien.
- **La carte d'accord du terminal** (ticket `2026-11-21-le-terminal-montre-la-carte-d-accord-generique-et-son-rappel`) :
  - une carte jamais cachée, ses lignes `approval_state`, une seule fermeture qui dit qui a tranché ;
  - elle attend l'accord générique d'archi.
- **Le lot 2 du navigateur** : `browser_open`, `browser_click`, `browser_fill`, sous accord, `scope: once`.
- **Le lot 3 du navigateur** : `browser_handoff`.
- **Plus tard** :
  - la fenêtre « mémoire à côté » de l'IDE ;
  - le cadrage du kanban (sans code avant le mot de Lucie).

## Questions à Lucie, non bloquantes

- Installer l'extension et le manifeste Native Messaging dans le Chrome du portable, pour un premier essai réel du lot 1.
- Les domaines du premier essai (`teammate.browser.domains`).
