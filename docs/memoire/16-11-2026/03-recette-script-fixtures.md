# Qualification des scripts de fixture — couple proposé

[Ticket existant](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md).
Source gelée `ff37eb82e52aa733503f3bf60fbf7a4d052ead85`, branche
`cx-memoire-script-fixtures-r3`, base `f83c6c4dd6a40200883c6325326f093ecdba3e23`.
Six patches range-diff égaux depuis r2 ; changement Rust dans cfg(test) seulement.
Pilote séparé `cx-memoire-script-fixtures-pilot-r1`, produit identique à APP.
Aucun Cargo/rustc/target/service lancé pendant cette préparation. GO couple exact
et créneau réservés à C7, pas d'héritage des anciennes autorisations.

## Admission et commande

Préparer par Git seul les deux worktrees PC propres et le gitlink exact APP :
`~/git_workspaces/cx-memoire-script-fixtures-r3` et un W pilote séparé.
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
