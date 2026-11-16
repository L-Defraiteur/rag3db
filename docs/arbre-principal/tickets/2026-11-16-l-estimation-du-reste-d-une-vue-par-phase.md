# L'estimation du reste d'une vue, par phase et depuis le journal d'index

- **État** : ouvert (16 novembre 2026), arbre principal.
- **Famille** : produit
- **Gravité** : l'attente d'index de la page 12 d'archi (« prêt, ou au plus N s », le seuil de 6 s de Lucie) n'a
  aucune estimation juste à lire. Le journal annonçait « ~27 min » pour un repassage de 61 min : c'est 2 à 2,5 fois
  sous le réel.
- **Atteignable en service** : oui, à toute recherche lancée pendant un index.
- **Touche rag3weaver** : oui (`src/dataflow/index_nodes.rs`, qui écrit le journal d'index ; l'état
  `index_states` de `src/backend.rs`).
- **Pour** : arbre principal ; à faire par Codex, contre le contrat de
  `docs/rag3weaver-archi/15-11-2026/12-conception-l-attente-d-index-du-pas.md`.
- **Signalé par** : arbre principal et c7 (la répétition de la bascule du 15 novembre, ses relevés dans
  `~/.cache/rag3weaver-build/bascule/journal-code-*/repassage.jsonl` à luciepc), 16 novembre 2026.

## Ce que c'est

Les lignes du journal d'index (`indexation lancée`, `plein texte : N fichiers sur M`, `plein texte prêt · vecteurs X
sur Y · reste environ Z s`, `indexation terminée`) ne portent pas d'heure. L'estimation « au rythme observé » se
fonde sur le premier paquet, qui ne compte ni les paquets plus lourds ni la phase des vecteurs.

## Le correctif attendu

1. **Horodater chaque ligne du journal** (ms depuis l'époque, en tête de ligne ou en champ) sans changer la
   dernière ligne, qui est le contrat (`DONE_LINE`, `FAILED_PREFIX`, `STOPPED_PREFIX` dans `index_nodes.rs`).
2. **Un débit observé par phase** (plein texte, relations, vecteurs), lissé sur les derniers paquets.
3. **Le reste d'une vue en fichiers** : l'écart entre sa pointe et sa génération publiée (2a′-3 le saura dire ;
   avant lui, le reste du backend).
4. **Exposer `remaining_s` à l'hôte** (dans `index_states`), pour que l'attente bornée juge reste × débit contre
   le seuil. Un débit de départ viendra du banc d'étalonnage de Lucie (banc et embarquement) ; sans lui, pas de
   promesse avant le premier paquet.

## Ce qu'il faut pour le fermer

Un témoin rouge d'abord : sur un index de N paquets dont le premier est rapide et les suivants lents (un faux
embarqueur lent), l'estimation à mi-chemin tombe à moins de 25 % du réel. Puis la passe épinglée avec
`e2e_code_sync` si un fichier de `tools/poste/required_suites.txt` est touché.
