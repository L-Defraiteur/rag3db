# Le profil core par défaut inclut des diagnostics non isolés

- **État** : ouvert (16 novembre 2026), constat statique pendant la préparation de qualification warnings.
- **Gravité** : moyenne pour la fiabilité des preuves et l’isolation des tests.
- **Famille** : validation
- **Pour** : lucivy
- **Atteignable en service** : non ; cargo test du core par défaut.
- **Touche rag3weaver** : non.

## Constat et preuves

Source890c86af44989f3fea6c6fa62fd40d688b4084df :
bench_vs_tantivy (benches/bench_vs_tantivy.rs) n’est pas ignoré, réutilise des
index sous /home/luciedefraiteur et appelle remove_locks récursivement.
Les targets test_store_fallback, test_diagnostics, test_lock_investigation,
test_search_mismatch contiennent aussi des diagnostics par défaut sur ces
index personnels ; certains retournent normalement si l’index manque.
Aucun de ces chemins n’a été ouvert pendant l’audit.

test_luce_roundtrip/test_luce_playground_search retourne normalement avec
Skipping si playground/dataset.luce manque, ce qui est le cas dans notre W PC.
Pas de snapshot substitué ni généré pour fabriquer une réussite.

test_fuzzy_ground_truth, test_regex_ground_truth et test_merge_contains
parcourent la racine du checkout CARGO_MANIFEST_DIR/.. avec liste d’exclusions
qui ne contient pas .vault. Le W comprend des noms suivis sous .vault (lecture
des seuls noms via ls-files, aucun contenu lu). C7 interdit lecture .env/.vault ;
ces trois suites sont HOLD dans la proposition, pas exécutées silencieusement.

## Suite et fin

Dans la qualification warnings actuelle, publier les neuf targets HOLD et leurs
inventaires ; aucun de leurs tests n’est vert. Ne pas modifier le produit ni les
sources des tests pour élargir ce lot. Ce ticket distinct garde le défaut connu.
Suite à décider par C7 : rendre les diagnostics explicitement manuels ou isoler
leurs entrées/sorties via les primitives scratch/corpus existantes ; empêcher
les scans de sources de lire les racines exclues. Une future correction doit
avoir ses propres témoins rouges puis verts, et ne pas transformer une absence
de données en succès. Fin : profil par défaut isolé, prérequis manquants rendus
explicitement non qualifiés, preuve de non accès aux index personnels et racines
privées. Aucun test de ce ticket joué à ce stade.

AdmissioncorpusFS découverte et portée AVANTsourcefix substantiel dans
[le ticketFS dédié](2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md),
liéaupèrecoreetindexed-false. C7confirme : pasrerunArefusprédit, jeufixé
nonadmissibleparité ;BnatifHOLD. Prépaobservablevueordonnée/readonce/IOUTF8fatal/
panel15avantbuilder/ni0=0vert/nireuse-deleteV3,4témoinsauvrai raccordFS+spies,
ROavantimplsubstantielle/natif. Aucuncorpus/ELF/Cargo/servicecontacté,9mémoire
qualifiésSource1ee conservés,aucunequalificationFSdéduite.
