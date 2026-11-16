# Un scope de namespace passe devant une classe du même nom

- **État** : ouvert (16 novembre 2026) ; observation
- **Famille** : produit (classement)
- **Gravité** : basse. Une recherche sur un nom rend d'abord des scopes `namespace X {` presque vides, et pousse la classe du
  même nom hors du top 10.
- **Atteignable en service** : oui, sur un dépôt C++ dont les en-têtes ouvrent tous le même namespace (`namespace rag3db {`).
- **Touche rag3weaver** : à départager (le rendu d'une version, `scope_version_configs` dans `src/code.rs` ; le découpage ;
  ou lucivy, la longueur courte qui avantage).
- **Pour** : everything declarative (ED) ; lucivy si la longueur est en cause.
- **Signalé par** : banc, la comparaison de A (copie migrée) et B (instantané réparé) contre l'oracle, 16 novembre 2026, 5 h ;
  ticket demandé par c7.

## Ce qu'on voit

Requête « rag3db », filtrée sur `src/include/`, tous signaux puis dense seul. Dans B, la classe `rag3db_database`
(`src/include/c_api/rag3db.h:146`) est au rang 4. Dans A, elle manque : les rangs 3 à 10 sont des scopes nommés `rag3db` aux
lignes 5 à 10 d'en-têtes (`semi_mask_target_type.h`, `column_evaluate_type.h`, `subquery_type.h`, `density_state.h`…). Ce
sont les `namespace rag3db {`, dont les enfants sont repliés : le scope ne porte presque que son nom.

## Ce qu'il faut faire

D'abord mesurer, puis proposer, sans changer le classement de soi-même :
1. Sur une copie de la base du banc (`banc-parite/ecr-base-ecrivain-neuf-5-lot-reindex-20261115`, à luciepc, reflink, jamais
   l'original) : le `content` indexé des namespaces `rag3db` (sa longueur, ce que le repli y laisse), leur rang par signal
   (bm25, vector) et celui de `rag3db_database`.
2. Dire laquelle des trois causes pèse :
   - le titre (`name` = la requête exactement) et son boost ;
   - un texte très court, que BM25 (longueur) et le dense avantagent ;
   - un scope conteneur qu'on ne devrait pas embarquer seul.
3. Proposer une règle en une page, sous `docs/everything-declarative/<date>/`, par exemple : un scope conteneur (namespace,
   module) dont le contenu replié tient en moins de N caractères ne s'embarque pas, ou se rend avec ses premiers enfants.
   Avec les chiffres de la mesure. **Pas de code avant le mot de c7 sur la page.**

## La preuve

La page avec les rangs mesurés. Si une règle est prise : un témoin rouge d'abord (deux scopes, un namespace replié et une
classe du même nom ; la classe passe devant) et la parité de banc rejouée.

## Comment le fermer

Par la page et la décision. Ou bien par un refus écrit ici : le comportement est accepté.
