# Le terminal montre la carte d'accord générique et son rappel

- **État** : ouvert le 21 novembre 2026 par la session tty. Il attend l'accord générique d'archi (mode auto, étage 3, `docs/rag3weaver-archi/21-11-2026/03-le-mode-auto-contextuel.md`), puis passe avant le lot 2 de l'outil navigateur.
- **Famille** : produit
- **Gravité** : sans lui, le terminal montre une carte vide (il ne lit que `command`) pour tout outil autre que `run_command`, et ne dit rien du rappel.
- **Atteignable en service** : oui, dès que l'accord générique atterrit.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty` : `boite.rs`, `vue.rs`).

## Ce que c'est

**La forme a changé, Lucie, le 21 novembre vers 10 h** : la carte ne se cache jamais. Une seule carte reste ouverte du début à la fin, son état change, et elle n'a qu'une seule fermeture (archi, mode auto, étage 3, page 03 du 21 novembre).

1. **L'ouverture** : `{"from":"teammate","kind":"approval","id":"L12","tool":…,"args":…,"class":…,"scope":"once"}`.
2. **L'état** : une ligne par étape, chacune remplace la précédente à l'écran.
   - Sa forme : `{"from":"host","kind":"approval_state","reply_to":"L12","stage":"recall"|"appeal"|"waiting_survivor"|"survivor","text":…,"until":<ms>|null}`.
   - `until` sert au compte à rebours de `waiting_survivor`.
   - Pour `appeal`, `text` porte ce que l'agent a fait valoir, et une ligne suivante dit pourquoi cela n'a pas suffi.
3. **La fermeture**, une seule : `{"kind":"answer","reply_to":"L12","approved":true|false,"from":"user"|"recall"|"appeal"|"verifier",…}`.
   - Elle dit qui a tranché.
   - `recall` porte `authorization{quote, at}`.
   - `verifier` porte `model` et `reasons`, plus `alternative{text, class}` si le refus en propose une.

L'utilisateur peut répondre à tout instant : sa ligne `answer` ferme la carte et arrête tout ce qui court. La frise : le rappel en parallèle dès le début, l'appel de l'agent à 30 s (`gate.user_wait_s`), puis le survivant 10 min après l'échec de l'appel (`survivor.wait_s`), s'il est allumé.

## Ce qu'il faut au terminal

- `Attente` porte `tool`, `args`, `class` et `scope`. La ligne d'état dit l'outil et ses arguments, pas le seul `command`, et « pour cet envoi seulement » quand `scope` vaut `once`.
- La carte reste dans la zone vivante, avec o / n, du début à la fin. Une ligne `approval_state` remplace son état à l'écran (« cherche une autorisation… », « l'agent fait valoir : … », « le survivant tranchera dans 9 min »), avec le compte à rebours de `until`, sans rien écrire à l'historique.
- La fermeture laisse une ligne d'historique qui dit qui a tranché : « accordé par toi », « validé par ton autorisation du … : « … » », « accordé sur l'appel de l'agent », ou « refusé par le vérificateur (<modèle>) : <raisons> », avec l'alternative s'il y en a une.
- Les témoins sont rouges d'abord, sur les trois formes de lignes. Puis la séance du harnais, avec un faux hôte qui les publie.
