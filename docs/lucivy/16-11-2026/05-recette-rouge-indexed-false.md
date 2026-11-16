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

## Chemins matérialisés et précontrôle (aucun Cargo)

[PrécontrôlePC](preuves/indexed-warning-preflight-v2.json) :
- Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-16nov-v2`,
  créé neuf, UID1000, dev55/inode164532587, poste mode755 ;
- W `/home/lucied/git_workspaces/cx-lucivy-indexed-false-warning-16nov`, branchecx
  neuve depuis0c795f99 et propre ;
- T `/home/lucied/.cache/rag3weaver-build/cx-lucivy-target-16nov`, **ABSENT**,
  aucune adoption d’un ancien target ;
- P `/home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-16nov-v2/logs`,
  **ABSENT**, et scope sous mêmeQ `/scope` **ABSENT** ;
- nonce de provenance `3552903efab590b44f44cd8002f7b028` (pas un jeton d’autorité).

Le paquetv1 de préparation, jamais admis/lancé, est conservé. Le v2 corrige la
lecturecpu.max à la racine cgroupv2 sans contrôleur racine et fige le couple.
Hashes copiés et vérifiés avant tout build :
- qualify-indexed-false-red.py `0a269dd90b0ca745811713e21f66b45fda563074bfc06dd6993f04ab03bd2352` ;
- admit-indexed-false-red.py `32c7e0936e6ad29f83224c948856851a78f622fa1f5c4cbb4fbea5a4cda2835c` ;
- support/confiner_validation.py `6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea` ;
- support/poste `79e3a8bbf457e3bc9104aa1558e7d03072216c70b68e08ae606ace6aa4584c7b`.

Le précontrôle par lstat rejette T/P existants et tout composant symlink,
au lieu de les masquer avec resolve. Il vérifie UID/nonce/inodeQ/hashes4copies.
L’admission refait ces contrôles APRÈS vrai poste, crée scope neuf+provenance
et END=START+600, puis exec le confineur pour le reste entier moins2s, aucun
reset. Le pilote vérifie receipt/scope/nonce et crée T avec mkdir exclusif,
markerprovenance nonce/UID/source ; logs neufs exclusifs. Le quota cpu.max est
relu sur toute hiérarchie du scope, jobs=min(4,allowedmoins2,quotaentier),
minimum1job si quota fractionnaire ; le cap4 est donc abaissé si quota<4.
La compilation prend ce même nombre dans -j et CARGO_BUILD_JOBS.

## Commande littérale candidate au GO final, PAS exécutée

```bash
POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-16nov-v2/support/poste lourd timeout --signal=TERM --kill-after=5s 610 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-16nov-v2/admit-indexed-false-red.py
```

Cette admission transmet les chemins W/T/P/scope ci-dessus au pilote figé et
au helper de SHA explicite. Pas de scope systemd imbriqué ni d’ancien reçu.
La commandeCargo est --offline --release -jN -p lucivy-core --test
 test_text_field_indexed_false --no-run --message-format=json-render-diagnostics,
N issu du quota réel. Puis l’exécutable attesté est inventorié --list --exact
unsupported_text_indexing_options_are_reported_once_per_field, et joué une
fois avec --exact et --nocapture. ExécutableSHA/source/receiptEND/identités
et bruts conservés ; harddeadline et quiescence au confineur commun.

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

## Résultat du seul lancement admis

Source0c795f99/pilote86989d7ab5 exécutés : compilationoffline réussie, inventaire1,
rouge d’assertion attendu0/1/0 code101, aucunretry. ScopeROUGEcode1 sansrestes,
quiescence et sourcespropres confirmées. Bruts dans
`preuves/indexed-warning-red-0c795/` et ticketmisàjour. Ce GO ne vaut pas un
second lancement sur un correctif ; aucun ancien END réactivé.
