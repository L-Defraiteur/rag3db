# Qualification isolée de read_scope — préparation

État : préparé, aucun Cargo ni runtime lancé. GO lourd C7 requis après
landing physique ; aucun tour master demandé par cette préparation.
Ticket : [erreurs des worktrees](../tickets/2026-11-13-read-scope-masque-les-erreurs-de-lecture-des-worktrees.md).

## Sources et isolation

Base exacte b0a5177c00bfaf3b37ae21ea0c630c515a19e224 ; patch rebasé
a919ad9d9, équivalent à 674f596c187d0d643d6ffec1d05b38dd00e5a511
(range-diff un patch égal). Fichier atelier_fichiers.rs byte-identique à
674f596c1 ; les trois documents du patch sont également conservés.
Branche et WT neufs cx-ed-read-scope-qualification-r2. Revues statiques
Archi et C7 favorables, tests encore non compilés et non exécutés.

Exécution uniquement sur luciepc, clone/WT correspondant au hash publié,
avec sous-modules épinglés. Target neuf dédié
~/.cache/rag3weaver-build/cx-target-ed-read-scope-r2 ; ne jamais employer
cx-target-contenu-derive ni les ressources Arbre. Preuves dédiées
~/.cache/rag3weaver-build/cx-preuves-ed-read-scope-r2. Avant création :
attester absence du target et du dossier de preuves, source propre,
aucun rebase, puis enregistrer hash/tree et état des sous-modules.

## Portée et budget proposés à C7

Un seul superviseur extérieur, budget global 6000 secondes incluant toutes
compilations, contrôles et nettoyage. Commande depuis la crate :

```sh
python3 scripts/confiner_validation.py lancer --mode mesure --delai 6000 -- bash driver.sh
```

La primitive acquiert le vrai poste exclusif Max32GiB et porte reçu/bus/budget.
Ne pas imbriquer un autre poste ou scope. CPU j8, pas de service externe,
pas de NAV/CDP/7879 demandé. SDK29 du magasin épinglé
rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.
CARGO_BUILD_JOBS=8, CARGO_INCREMENTAL=0, RAG3DB_BUFFER_POOL_SIZE=1073741824,
quatre rôles BURN CPU. TMP/cache/config/state/journaux privés sous les preuves.
Charger scripts/lib_du_magasin.sh et exporter_la_lib une seule fois dans
le driver ; garder_le_target avec la racine crate exacte avant Cargo.

Driver à matérialiser avant lancement : set -euo pipefail, chaque commande
journalisée, timeout borné par le minimum cap de phase/temps global restant,
réserve finale de nettoyage ; premier rouge inattendu STOP sans retry.
Caps proposés : compilation 2400s, chaque témoin 120s, module 600s,
frontières 1800s cumulées, nettoyage 60s, toujours sous budget global.
Si le budget ne permet plus une étape, verdict rouge/incomplet, pas de reset.

## Ordre et juges

1. Construire une seule lib de tests avec --no-default-features --features
   rag3db-native,code,daemon ; extraire l’ELF depuis les messages JSON Cargo,
   archiver commande/source/SHA/features/SDK et inventaire --list.
2. Exécuter séparément, --exact --show-output --test-threads=1, les quatre
   nouveaux témoins sous daemon_host::atelier_fichiers::tests :
   read_scope_preserves_a_backend_error_instead_of_claiming_absence ;
   read_scope_preserves_a_usages_error_instead_of_claiming_absence ;
   read_scope_refuses_unproven_uniqueness_after_a_real_definition ;
   read_scope_keeps_real_absence_and_ambiguity_after_complete_queries.
   Puis le nominal historique
   the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree.
   Exiger exactement un passé/aucun échec/aucun ignoré pour chaque invocation.
3. Inventorier puis jouer le module complet daemon_host::atelier_fichiers::
   avec le même ELF, --show-output --test-threads=8. Comparer noms/comptes
   à l’inventaire ; publier séparément les ignorés navigateur déjà déclarés
   dans la source, jamais les compter comme verts. Aucune exemption nouvelle.
4. Frontières : cargo check --tests --no-default-features pour les huit jeux
   du script check_features.sh (minimal, daemon, natif+daemon, batterie,
   batterie+LLM, paquet npm, natif+code, natif seul), target privé commun.
   Archiver chaque commande/log et arrêt immédiat au premier échec.

Les résultats des témoins ciblés et du module sont deux invocations,
pas des juges indépendants supplémentaires. Les contrôles de features
ne constituent pas une nouvelle globale physique. Le succès éventuel
ne prouve ni la cause du FAILED ancien ni l’exhaustivité des inscriptions.

## Verdict et conservation

Avant verdict composé : superviseur vert/code0/restes[], bus retiré,
quiescence PID/start/exe/cwd du WT et du target, source inchangée, ELF réhashé,
attestation maps SDK29 et événements mémoire avant/après. UNKNOWN n’est pas
une absence. Archiver logs et manifeste SHA sur les deux postes, rouges compris.
Aucun retrait du WT/target avant gardes fraîches et coordination distincte ;
aucun push master avant tour explicite C7 et procédure de passe.
