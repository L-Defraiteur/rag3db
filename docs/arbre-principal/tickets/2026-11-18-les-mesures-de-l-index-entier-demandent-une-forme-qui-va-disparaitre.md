# Les mesures de l'index entier demandent une forme de base qui va disparaître, et se sautent sans le dire

- **État** : ouvert (18 novembre 2026), arbre principal ; décision : garde de capacité ou retrait.
- **Famille** : produit
- **Gravité** : des tests muets. `tests/e2e_mesure_index_entier.rs` rend « NON MESURÉ » et passe vert quand
  `RAG3WEAVER_MESURE_BASE` manque (`base()`), et ses mesures des morceaux vides ne valent que pour la forme d'avant
  (`Scope_Chunk`), qui disparaît des bases de Lucie à leur bascule.
- **Atteignable en service** : non (des tests `#[ignore]` de mesure).
- **Touche rag3weaver** : oui (`tests/e2e_mesure_index_entier.rs`).
- **Pour** : arbre principal ; au mot de c7 du 18 novembre.
- **Signalé par** : arbre principal, en faisant A1
  (`docs/arbre-principal/tickets/2026-11-13-les-sondes-bancs-et-mesures-du-code-lisent-scope-chunk-en-dur.md`).

## Ce que c'est

- Les sept mesures des morceaux à texte vide (`les_morceaux_a_texte_vide` et suivantes) enquêtaient, en octobre, sur
  des bases de la forme d'avant. Depuis A1, elles exigent cette forme et refusent une base à versions en le nommant
  (`require_the_form_of_before`).
- Aujourd'hui, la forme d'avant existe encore : les trois bases de Lucie ne sont pas basculées, et leurs instantanés
  non plus. Après la bascule, aucune base de cette forme ne restera, sauf les instantanés de retour, pendant 72 h.
- Toutes les mesures du fichier se sautent en vert, avec un « NON MESURÉ » en sortie, quand `RAG3WEAVER_MESURE_BASE`
  manque. Une suite qui ne joue rien doit être rouge (la règle des sessions).

## Le correctif attendu, à choisir

1. **Une garde de capacité** : sans `RAG3WEAVER_MESURE_BASE`, chaque mesure échoue en nommant ce qui lui manque, au
   lieu de passer. Une base de la forme d'avant se garde à part, nommée, pour qu'on puisse encore les jouer.
2. **Le retrait** des mesures des morceaux vides, une fois la bascule faite : la question qu'elles posaient (le texte
   perdu entre le hachage et l'écriture, octobre) est close par le ticket des morceaux sans texte.

Les mesures d'`usages` du même fichier (`ou_va_le_temps_d_usages_sur_l_index_entier` et suivantes) ne dépendent pas
de la forme : elles gardent leur place, avec la garde de capacité.

## Ce qu'il faut pour le fermer

Le choix fait, son témoin : sans base, la suite est rouge en le disant (garde), ou les mesures sont retirées avec la
raison dans le message.
