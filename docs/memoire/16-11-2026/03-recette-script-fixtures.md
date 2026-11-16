# Qualification des scripts de fixture — couple proposé

[Ticket existant](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md).
Source gelée `734087a4e99d54331aac2f6321df9ef6da3b6d9c`, branche
`cx-memoire-script-fixtures-r4`, base `d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6`.
Six patches range-diff égaux depuis r2 ; changement Rust dans cfg(test) seulement.
Pilote séparé `cx-memoire-script-fixtures-pilot-r2`, produit identique à APP.
Aucun Cargo/rustc/target/service lancé pendant cette préparation. GO couple exact
et créneau réservés à C7, pas d'héritage des anciennes autorisations.

## Admission et commande

Préparer par Git seul les deux worktrees PC propres et le gitlink exact APP :
`~/git_workspaces/cx-memoire-script-fixtures-r4` et un W pilote séparé.
Ne pas basculer/reconfigurer les hôtes927. Le pilote ne lance ni CLI7970 ni manager.
Après RO et GO exact C7 seulement, depuis le W pilote :

```bash
python3 docs/memoire/16-11-2026/script-fixtures-qualification/qualify.py --launch <SHA40_DU_PILOTE>
```

P `cx-memoire-script-fixtures-qualification-v1`, Q `cx-memoire-script-fixtures-controller-v1`,
S `cx-memoire-script-fixtures-scope-v1` et T `target-cx-memoire-script-fixtures`
sous `~/.cache/rag3weaver-build`, tous ABSENTS au départ. Aucun autre target touché.
Contrôleur/confineur/poste copiés horsW, SHA relus. Vrai poste lourd quota parplace,
32GiB/HIGHmax/swap0/j8 maximum. Timeout7260 APRÈSposte et reçu NEUF7200 aprèsposte ;
le budget inclut les deux passages et le nettoyage. Toutes les phases MIN END−30,
finalisation après SHA/scan sous END absolu, puis contrôle frais prépublication.
Pas de reset ni reprise automatique. Admission vérifie cpu.max fini et mémoire.

Profil whitelisted, bus privé du confineur commun, XDG et RAG_STATE frères distincts ;
SDK59ac complet et SO SHA identiques aux deux postes (rapport02), magasin réel fixé
`RAG3DB_LIBS_DIR=/home/lucied/.cache/rag3db-libs`. Aucun MOTEUR_ANCIEN hérité.
Le pilote ne touche aucun service. **La passe canonique non modifiée configure
elle-même les services d'embarquement7878/7879 et le repli igpu:0 PC** ; leurs
éventuelles utilisations par lib/NAV font partie du périmètre à arbitrer par C7.
Les rôles Burn CPU ne sont pas une preuve d'absence GPU. Aucun appel API7970 requis.

## Parcours et preuves

1. Source/pilote/gitlink propres, pin/SO/copie canoniqueSHA2150 recontrôlés. T absent
   après vrai poste, scan exe/cwd/FD de source/target ; aucun cache ancien adopté.
2. Canonique `SUITE_DU_TEMOIN=lib SUITES=-`, filtre `codex_threads`, cap6600MINEND−30 :
   nouveaux+historiques ciblés, F8/preuve, lib entière large, NAV réels. Aucune suite
   code_sync requise par les chemins du lot. Premier test FAILED stop, should_panic
   normal conservé. Confineur attribue/arrête les descendants ; aucun kill par nom.
3. Lire la ligne Cargo `Running unittests src/lib.rs (ELF)` de lib.log, ELF exact
   sousT, copie+SHA, ledger publié avant SECONDE lib. Cette attestation est après
   la première lib : pas de revendication de ledger avant tous les premiers tests.
4. Rejouer DIRECTEMENT ce même ELF entier, horsGDB, cap600MINEND−30, avec export SDK
   canonique et même profil PC ; aucun Cargo ni deuxième F8/NAV à cette étape.
   Nombres passés/ignorés identiques, filtrés0, SHA original/copie inchangés.
5. Résumé canonique inchangé et son SHA conservé ; second-lib.log séparé. Verdict
   composé ne remplace jamais resume.log. Ignorés non verts, répétitions non comptées
   comme nouveaux juges. Pas de preuve interbase ou stabilité générale inventée.
6. Après scope : rc0/scopeVERT0/restes[]/nettoyé, autorité terminée, bus absent,
   scan frais + toutes identités PID/start du superviseur absentes ; UNKNOWNowned
   bloque. UNKNOWN globaux non attribués consignés, pas d'absence GPU déduite.
   Target conservé, pic/events du confineur conservés, aucun nettoyage de lot.

Témoins Python hermétiques prépubliés avant exécution : parse de résumés et ignorés,
refus zéro/échec/doublons, should_panic vs first-red, chemin/ELF exact/symlink,
deadlineégalité, premier rouge immutable, finalisation après deadline refusée.
Leurs résultats futurs ne qualifient pas le helper Rust ni la passe canonique.

## Actualisation avant gel

C7 impose le master ties `d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6` :
source r4 NEUVE `734087a4e99d54331aac2f6321df9ef6da3b6d9c`, six patches
range-diff égaux. Pilote r2 NEUF ; r1 jamais lancé, ne vaut aucun GO.
Les huit fonctions hermétiques prépubliées4cef0f8ae9 ont passé en0,002s sous
timeout20, Python-B, aucune lecture /proc réelle ni signal/process/Cargo.
La fonction de finalisation publie bien RED quand le temps simulé est expiré.
Nouvelle constante source/W/base propagée explicitement avant la RO finale.

## Refus du watcher récursif, avant exécution lourde

Pilote1e3a3d005d retiré/non lancé après RO ED. Réserve ticket0990449574
avantdiff, témoins vrais run/fauxPopen publiés760b94b843 puis rouge acquis :
journal historique `cache/builtin/llm-gemini.log` provoque à tort un arrêt.
Journal horsW `journaux-cx-memoire-script-fixtures-16nov/rouge-watcher-760b.log`.
Correction : liste fermée canonical-pass.log, second-lib.log et official/{temoins,
check_features,preuve,lib,navigateur,resume}.log ; aucun parcours des caches.
Témoins actifs FAILED et expected should_panic conservés, aucune exception de
contenu ni whitelist du mot FAILED dans les sorties actives. Nouveau pilote à RO.

Après code publié36da4238cb : 11fonctions hermétiques/0échec en0,003s,
timeout20, sans process réel/Cargo/proc/signal. Journal vert horsW
`journaux-cx-memoire-script-fixtures-16nov/vert-watcher.log` ; rouge conservé.
App734087 inchangée, aucune Auth/PQS/T créée. RO+GO exact nouveau pilote requis.

Demande C7 après correction liste fermée : réemploi BYTEIDENTIQUE des vraies
primitives ED13c8 `first_red/active_logs/watch_logs`, provenance SHA complète
et parfonction dans watcher-provenance.json. Phase appelante renommée official-pass ;
les chemins e2e déclarés par ED ne sont pas produits sous SUITES=-. Aucun rglob
dans watcher, lecture/stat/hash du seul fichier producteur actif ; premiers
match/rouge exclusifs. Vrai run testé avec historique FAILED, actif FAILED,
should_panic et re-latch sans écrasement. Journal canonique demeure intact.
