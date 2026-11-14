# La visibilité dans le prédicat du masque coûte cher et mène à l'OOM

- **État** : ouvert (14 novembre 2026).
- **Famille** : produit
- **Gravité** : bloquant pour le drapeau. Quand `tout_visible` tombe, le prédicat du masque porte la visibilité (`_couche`, `_depuis` et un `Should` sur `_jusqu_a`). Une recherche sans filtre prend alors 5,4 à 8,1 s, et le processus passe de 27,9 à 30,7 Gio en trois appels, puis il est tué à 33,5 Gio. Hors appel, les racines par la clause coûtent 1 376 ms, contre 25 ms par `select_all`
- **Atteignable en service** : non aujourd'hui. Le drapeau ne tombe qu'avec l'écrivain de 2a′-2 (`marquer_des_lignes_invisibles`), qui n'existe pas encore
- **Touche rag3weaver** : oui (`masque_en_predicat`, `src/dataflow/generic_search_nodes.rs`)
- **Pour** : everything declarative
- **Signalé par** : everything declarative, 14 novembre 2026, luciepc, `poste mesure`, lib 373b5477b, vue-publiee 959d73513, état B de `tests/sonde_recherche_sans_filtre.rs`

## Ce qu'on pourrait faire

Le cache par (couche, rang), posé par l'écrivain de 2a′-2 à chaque publication, et invalidé par
l'époque qu'il donne (page `docs/everything-declarative/14-11-2026/05-la-vue-publiee-par-couche-et-rang.md`,
§ 2 bis) : l'ensemble admis n'est alors calculé qu'une fois par rang, et non par requête. **Le drapeau
ne tombe pas avant que ce ticket soit fermé.**
