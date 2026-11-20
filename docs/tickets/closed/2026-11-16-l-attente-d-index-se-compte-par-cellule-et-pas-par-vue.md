# L'attente d'index se compte par cellule, et pas encore par vue

- **État** : fermé (20 novembre 2026, session recherche) : tenu par les lots atterris, vérifié sur master. Ouvert le 16 novembre 2026, session recherche.
- **Gravité** : réponse approximative. Sous une vue d'une autre couche que celle du backend, « index pas
  prêt » parle du retard de la cellule, pas de celui de la couche lue.
- **Atteignable en service** : oui, dès que des couches de travail par worktree s'indexent (2a′-3).
- **Touche rag3weaver** : oui (`src/catalog/retard.rs`, `src/backend.rs`, `wait_for_a_lagging_index`).
- **Famille** : produit
- **Pour** : recherche (ou Codex, par la passation du 16 novembre).
- **Signalé par** : recherche, au lot « index pas prêt » (master 8b7686025e, plan `docs/recherche/16-11-2026/02-plan-index-pas-pret.md`).
- **Bloqué par** : 2a′-3 d'arbre principal. Sa page dit que le reste d'une vue se compte en fichiers,
  comme l'écart entre sa pointe (HEAD et fichiers non commités) et sa dernière génération publiée.

## Ce que c'est

`Lag` (`catalog/retard.rs`) tient, **par cellule**, les fichiers changés et pas encore réindexés, et le
débit observé. `Backend::wait_for_a_lagging_index` lit l'estimation de la cellule de l'appel, quelle que
soit la `view` demandée. Sous 2b-C, une seule couche s'indexe (`backend`), et c'est donc exact. Avec 2a′-3,
chaque worktree aura sa couche de travail et sa montre : le retard doit se lire pour la couche de la vue.

## Ce qu'il faut pour le fermer

- `Lag` tient ses entrées par `(cellule, couche)`. La montre de 2a′-3 note avec la couche du worktree qui a
  changé. `estimate` et `wait_caught_up` prennent la couche.
- `wait_for_a_lagging_index` lit la couche de `CALL_VIEW` s'il y en a une, sinon celle du backend.
- Le contrat d'archi (`Lag::estimate`, `Lag::wait_caught_up`) garde sa forme à une couche près. Le lui
  dire avant le lot.
- Les témoins, rouges d'abord : deux couches, un fichier en retard dans l'une. Sous la vue de l'autre, la
  lecture ne l'attend pas ; sous la sienne, elle l'attend, ou répond « index pas prêt ».

## L'attente compte aussi l'embarquement (c7, 16 novembre 2026, à relire par Lucie)

**Le constat du banc** (`docs/banc/16-11-2026/03-le-banc-d-etalonnage.md`) : un fichier sort du registre de
retard dès que son **texte** est écrit. Les vecteurs de ses morceaux neufs arrivent plus tard, le temps de
l'appel au service. Une recherche qui utilise le signal vector, et qui a attendu sous `search.index_wait_s`,
manque donc encore ces morceaux.

**La forme décidée par c7** :
- `eta = eta_texte + eta_vecteurs` quand la recherche utilise le signal vector ; en bm25 seul,
  `eta_texte` suffit ;
- `eta_texte` est celui d'aujourd'hui : les fichiers en attente × le débit observé par fichier (`Lag`) ;
- `eta_vecteurs` = les morceaux dus des fichiers de la vue (`count_marqueur_manquant`, restreint à ces
  fichiers) × la taille moyenne d'un morceau ÷ le débit du service de la vue. Le débit vient d'abord du
  banc, puis il est corrigé par la moyenne glissante des lots embarqués.

**Ce qui existe et sert** : `IndexState::with_time_left` (`src/catalog/progress.rs`) calcule déjà la dette
de vecteurs en secondes (morceaux manquants × caractères par morceau ÷ `Rate` du service), pour l'état
d'index (`vectors_seconds_left`). Le lot le reprend plutôt que d'écrire une seconde formule. Il ajoute la
restriction aux fichiers de la vue et le départ du débit au banc.

**Les nuances de recherche** :
1. **Un service absent ou en panne** : les vecteurs ne viendront pas dans l'appel. `eta_vecteurs` est alors
   `null`, la cause est dite (`vectors: "service absent"`), et l'appel n'attend pas pour rien. La recherche
   dense dit déjà « not available » dans ce cas.
2. **La recherche hybride** : la branche bm25 voit déjà le texte neuf. Au-dessus du seuil, la réponse dit
   les deux restes à part (`text_eta_s`, `vectors_eta_s`), pour qu'un agent sache que ses résultats
   lexicaux sont complets et que seuls les denses peuvent manquer.
3. **Les outils sans signal vector** (`usages`, `outline`, `read_file`, `grep_files`) ne comptent jamais
   l'embarquement.

**Décidé par Lucie** (18 novembre 2026, « oui pour ta suggestion », relayé par c7) : la forme et les trois
nuances ci-dessus. **Bloqué par** : le débit du banc d'étalonnage seulement. Rien ne se code avant.

## Ce qui le ferme (vérifié sur master le 20 novembre 2026)

- **Le retard par `(cellule, couche)`** : `LagKey { cell, layer }`, `Lag::note_in`, `estimate_of` et
  `wait_caught_up_of` (`catalog/retard.rs`).
- **L'attente lit la couche de la vue de l'appel** : `wait_for_a_lagging_index` prend la couche de `CALL_VIEW`, à
  défaut celle du backend (`backend.rs`, « by view », Lucie, 18 novembre).
- **Ses témoins** : `a_read_under_another_view_waits_only_for_its_own_layer` et
  `a_read_under_its_up_to_date_view_does_not_wait_for_another_layer`.
- **L'attente compte aussi les vecteurs dus** (la forme de c7 du 16 novembre) : lot `recherche-attente-vecteurs`
  (PR 197). Texte et vecteurs ont chacun leur estimation (`text_eta_s`, `vectors_eta_s`) ; bm25 seul n'attend que
  le texte ; sans service, pas d'attente et la cause dite (`vectors_unavailable`). Les témoins sont
  `a_hybrid_search_owing_long_vectors_answers_at_once_and_says_them` et
  `a_hybrid_search_without_an_embedding_service_does_not_wait_and_says_why`. La notification unique
  « index prêt » suit (lots `recherche-notification-index-2` et `recherche-notification-e2e`).
