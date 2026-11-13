# Premier arrêt de qualification dédiée du garde target

Application698b952ab89fd48ac112645650000ebae34e6742 / pilote
bdffcef070d78f5a97cc7d2a1edbf60baef7b58e, GO C7 exact reçu.
Verdict ROUGE, aucun retry. Poste32GiB/HIGHmax, source et pilote propres,
codeparsers a730a9ba89ef233423bb7063f85181de26d98ce0 exact.
Reçu validation-angvfx8z, START909825.832473217 / END911625.832473217.

Dix CPU passent en0,303s, Rust toy8OK, fixture journaux7/0 en23,591s.
Compilation CPU sans features effectuée, ELF copié SHA
688fe230a86c1329e2b485177c3a8d2ec1acf36bfa192ac13bd0dbaeaef1392f.
223 blobs Git/chemins include_bytes worktree/présence octets ELF attestés,
table générée SHA84a813c3bc039103c924b6134fe0f7265218241c6f317c3db1eb5b9ca941096a.
Ledger SHA6da389b1c6f8294e6d8d306c33b71db82e78483c0e6a27cd5c29fb5975bcd070
publié au transport C7 avant runtime ; C7 a copié/réhashé ledger/ELF/attestation.
Marqueur écrit par Recherche sous GO existant ; message C7 annonçant une
écriture personnelle a ensuite été corrigé (tentative après fin refusée).

## Cause explicitement observée

Module builtin_templates :8 passés/1 échec/0ignoré, puis arrêt du pilote.
Juge a_witness_never_writes_in_the_real_state_directory, panic
src/builtin_templates.rs663 : state_dir_of retourne .../state/manifestes/...
sous XDG_STATE_HOME .../state. qualifier.py fixe XDG_STATE_HOME et
RAG3WEAVER_STATE_DIR au MÊME répertoire. Le juge interdit précisément cela.
Attribution au profil du pilote prouvée par source et log ; aucun défaut
produit du garde, de fraîcheur de l'ELF ou des gabarits établi par ce rouge.
Pas de panique perdue ni de succès global déduit des étapes préparatoires.

## Conservation et quiescence

Superviseur ROUGE/code1/restes[]/nettoyé ; bus/tmp/r3v-8y6pr_1a absent.
Scan indépendant exe/cwd/FD : found[],8 inconnus baseline consignés, aucune
absence GPU revendiquée. Deux worktrees restés propres, target conservé.
PC : cx-validation-garde-target-v1, cx-pilote-garde-target-v1,
cx-confinement-garde-target-v1. Copie locale cx-preuves-garde-target-v1-local.
252 fichiers SHA identiques contrôlés sur les deux postes, manifeste SHA
80aed869d758549ed9a8fc672eb79d690171ba9229545a79fbdf4251a0bdebb8.
Aucun service partagé/PG/SDK/NAV ni lot union touché.

## Suite à préparer, pas autorisation d'exécution

Ticket existant complété AVANT diff correctif. Proposition pilote seul :
XDG_STATE_HOME=xdg-state et RAG3WEAVER_STATE_DIR=state-fixture, siblings.
Application698/ELF/SHA/223/templates/toy/CPU/journaux/compilation conservés,
sans rebuild ; seule qualification manquante module9+attestations à reprendre
sous la même END911625.832473217 et START original. Nouvelle recette/couple
à publier et relire, nouveau GO explicite C7 requis ; aucun ancien rouge
repeint, aucun ancien GO recyclé. Ticket reste OUVERT, aucun master/retrait.
