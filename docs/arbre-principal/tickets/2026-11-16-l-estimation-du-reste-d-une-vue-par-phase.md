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

## La forme du reste, au mot de Lucie (18 novembre, par c7)

L'attente bornée (6 s) compte aussi les vecteurs dus quand la recherche demande le signal `vector` :
- **le texte** : les fichiers qui restent × le temps par fichier observé ;
- **les vecteurs** : les morceaux dus de la vue × leur taille moyenne ÷ le débit du service d'embarquement de cette vue.

Le débit d'un service vient du banc d'étalonnage (banc et embarquement) ; le code attend ce débit, sans constante à
sa place.

## Ce qu'il faut pour le fermer

Un témoin rouge d'abord : sur un index de N paquets dont le premier est rapide et les suivants lents (un faux
embarqueur lent), l'estimation à mi-chemin tombe à moins de 25 % du réel. Puis la passe épinglée avec
`e2e_code_sync` si un fichier de `tools/poste/required_suites.txt` est touché.

## Le mot de Lucie sur l'embarquement dans l'attente (18 novembre 2026, relayé par c7)

« Oui pour ta suggestion » : l'attente sous `search.index_wait_s` compte aussi les morceaux dus quand la
recherche utilise le signal vector. Trois nuances, décidées avec la forme :
- **service absent** : pas d'attente, et la cause est dite ;
- **hybride** : `text_eta_s` et `vectors_eta_s` à part ;
- **outils sans vecteurs** : jamais d'attente pour l'embarquement.

Le détail est dans `docs/recherche/tickets/2026-11-16-l-attente-d-index-se-compte-par-cellule-et-pas-par-vue.md`.
Le code attend le débit du banc d'étalonnage.
