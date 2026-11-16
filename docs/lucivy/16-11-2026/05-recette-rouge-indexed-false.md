# Témoin indexed:false : recette pour revue, NON lancée

Ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Porteur lucivy ; première action prévue uniquement, pas de lot supplémentaire.
Source0c795f99e5f49926a510971c521eb056a4880adc, branchelucivy
cx-lucivy-indexed-false-warning. APIvide+testconfig, pas de correctif.
Le test ne crée ni index, ni base, ni worker ; sa compilation construit les
cratesRust usuelles et demande donc un slot lourd, pas un faux petitCPU.

## Couple et ressources proposés

Pilote `outils/qualify-indexed-false-red.py`, à publier/revoir avant GOslot.
Primitive commune extraite de rag3dbd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6 :
- confiner_validation.py SHA2566439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea ;
- poste SHA25679e3a8bbf457e3bc9104aa1558e7d03072216c70b68e08ae606ace6aa4584c7b.
Copies privées côte à côte support/, aucun écrasement des caches déployés.
Pas de reprise d’un pilote13nov ni d’un reçu ancien. La libnative59ac et la
passe_epinglee de rag3db ne sont pas appelées : lot lucivy seul, sans ABI SDK.

Proposition **600s uniques après vrai poste lourd**, compilation+inventaire+
un exact, j4/MemoryMax32Gio/HIGHmax, au moins sixCPUallowed pour en laisserdeux.
Target unique de session ~/.cache/rag3weaver-build/cx-lucivy-target-16nov,
provenance/quiescence à constater avant emploi ; worktreePC neuf de source0c,
logs/PQS neufs `cx-lucivy-indexed-warning-16nov-v1` horsW. Aucun téléchargement :
Cargo offline ; dépendance manquante = erreur de compilation, pas TDDrouge.
Pas de corpus, ABSENT_OK refusé, aucun service/7970 ni modèle/baseLucie.

## Commande exacte candidate au slot, PAS exécutée

Préparer les copies support et source lors du GO exact, vérifierSHA/mod es,
worktreepropre, cibleprivée et absence d’autre détenteur ; ne pas prendre le
target de l’ancienne session. Dans le worktreePC :

```bash
POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0 SUPPORT/poste lourd timeout --signal=TERM --kill-after=5s 610 python3 SUPPORT/confiner_validation.py lancer --delai 600 --preuves P_SCOPE_NEUF -- python3 PILOTE_FIGE/qualify-indexed-false-red.py --worktree WORKTREE_0C --target /home/lucied/.cache/rag3weaver-build/cx-lucivy-target-16nov --output P_LOGS_NEUF
```

Les chemins SUPPORT/PILOTE/P/W seront matérialisés et figés au GOslot ; les
commandes intérieures du pilote sont fixées :

```bash
cargo test --offline --release -j4 -p lucivy-core --test test_text_field_indexed_false --no-run --message-format=json-render-diagnostics
EXECUTABLE_ATTESTE --list --exact unsupported_text_indexing_options_are_reported_once_per_field
EXECUTABLE_ATTESTE --exact unsupported_text_indexing_options_are_reported_once_per_field --nocapture
```

Le pilote atteste l’exécutable produit (chemin sous target etSHA), source et
propreté, inventaireexact1 ; compile/inventaire/test en brutsdirectfichiers,
identitésPID/start. Même reçuactif/end avant/après chaque phase et posthash.
Harddeadline et arrêt des descendants/quiescence restent au confineurcommun.

## Verdicts distincts

Rouge TDD attendu : code101, running1test, assertion left:[] contre deux
avertissements body/label, résumé0passé/1échec/0ignoré. first-red.json openx/fsync,
classification expected_assertion_red_observed. Le pilote finit code1 : le
scope reste ROUGE, pas de faux VERT parce que le rouge était attendu.
Compilation/inventaire/confinement/autre assertion = autre erreur conservée,
STOP sans correctif ni retry automatique. Le rouge attendu seul ouvre l’étape
suivante déjà prévue : implémentation warnings/call/doc, puis même juge vert.
Pas de suitecore complète ni ulp qualifiés par ce seul juge, pas de PR42/fusion,
publication/tag ou installation de service.
