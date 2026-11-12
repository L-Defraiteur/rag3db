# Compilation et lien attestés, lancement toujours à relire

Porteur hotfixs, [même ticket](../tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
Aucun changement produit, aucune compilation moteur ou gtest, aucun lancement
ni ouverture de base par le témoin. Aucun résultat fonctionnel moteur annoncé.

## Deux tentatives distinctes

La première tentative sur source `2b6b61983` s’est arrêtée au compilateur :
qualifications `common` ambiguës et `visible_func` non qualifié.
[Preuve compile-01](preuves/compile-01/build-report.json), sans objet valide,
lien ou lancement. Tous ses artefacts restent conservés dans le staging initial.

La seule correction du source publiée en `9aa2ecdc9` qualifie ces noms en
`rag3db::common` et `storage::visible_func`. Diff source relu avant la reprise,
conforme à l’autorisation explicite de cette correction uniquement. Les flags,
archives et header de fixture ne changent pas ; chemins de staging neufs suffixés02.

Seconde tentative dans
`~/.cache/rag3db-hotfixs-notes/cx-quarantaine-0511-structurel-02/` à luciepc :
[rapport brut](preuves/compile-02/build-report.json),
[journal compilateur](preuves/compile-02/compile.log),
[journal de lien](preuves/compile-02/link.log),
[ELF dynamique](preuves/compile-02/elf-dynamic.log),
[en-tête ELF](preuves/compile-02/elf-header.log),
[compilateur](preuves/compile-02/compiler-version.log) et
[sortie du superviseur](preuves/compile-02/service-summary.txt).

Sous `poste lourd`, cgroup 1 Gio et swap0 vérifiés avant compilation, unité
`cx-hotfixs-quarantaine-build-9aa2ecdc9`, borne globale240s, compilation120s,
lien60s, un seul compilateur, groupes de processus supervisés. Résultat :
compilation/lien/contrôles ELF code0 ; contrôleur3,793s, service3,820s,
pic affiché540,8M et swap0. Ces durées ne mesurent pas le rollback.
Logs plafonnés2Mio par commande, aucun dépassement.

Rehash avant compilation des830fichiers du paquet,17archives tierces,
archive moteur/manifeste, références de flags et en-têtes de fixture/générés ;
source de tiers0511 propre vérifiée. Rehash final du paquet et des entrées
principales sans écart. Aucun objet moteur ne vient du build historique ni
ne se recompile dans cette opération. La compilation utilise exclusivement
les en-têtes assortis du paquet privé.

## Empreintes et ELF

- Source : `d2db4d049f92dc5d197eb34e4fea758259bf4af7419537e1042b8a3c88a38530`.
- Objet118488octets : `df8bce14ffecfa1cf0a93b412262fa70fdb427bad746e7d651cfd1ff27dcb951`.
- Binaire25630312octets : `d3b6bb5dbe520e2209c607e34c18bf62fe689c5c0943d5fac30b0e34a3c8530b`.
- Rapport de build : `cef19a0f4731a81ee1d4a52b21501c9ab3fbd03e6e31fb46d33584e213f45427`.
- Manifeste moteur : `0049111df769914dbeea66fd0d2d07c6add58a670eb80e797589604d48c80df7`.
- Archive moteur privée : `3030fe674e33e2ea0b5b01ed7bdf1a919e538c02e5dfc0b68fff7b50e1d06758`.

L’ELF déclare exactement `libstdc++.so.6`, `libm.so.6`, `libgcc_s.so.1`,
`libc.so.6`, `ld-linux-x86-64.so.2`. Aucun NEEDED de lib projet, aucun
RPATH/RUNPATH. Lecture par readelf, pas par exécution du binaire.
Les noms de dépendances ne sont pas présentés comme une nouvelle validation
fonctionnelle des bibliothèques système.

## Lanceur prêt pour revue, pas pour lancement

[Source du lanceur](outils/04-lancer-temoin.py),
[plan et argv](outils/04-plan-runtime.json),
[modèle d’accord désactivé](outils/04-accord-runtime-modele.json).
Le modèle est maintenant lié aux empreintes réelles du source, binaire et
rapport, mais garde `approved_to_run=false`. Aucun accord runtime activé
ou copié sur le poste, aucune invocation du lanceur, pas même pour smoke test.

La copie du lanceur se trouve dans le staging02 sous `runtime.py` ; empreinte
à relire avant toute exécution future. Les argv proposés lancent une unité
utilisateur unique : MemoryMax1G, swap0, RuntimeMax20s, KillModecontrol-group,
arrêt forcé borné1s et Restartno. Le contrôleur intérieur revalide le cgroup et
l’accord lié aux empreintes, crée un dossier exclusif, puis lance un seul témoin
dans son groupe avec délai15s ; le témoin garde son alarme12s. Tout groupe restant
est arrêté par identité de PGID et le cgroup, jamais par nom de processus.
Sortie contrôleur1Mio maximum conservé ; statut/erreurs restent diagnostiques.

Fichiers de base/CSV/steps et parent conservés, sans suppression ni réouverture.
La sentinelle s’arrête avant toute construction réelle de Checkpointer et le
code sort sans destructeurs Database/Connection. Le code0 runtime éventuel
signifierait observation du chemin ou refus nommé, pas innocuité ni corruption.
Durabilité, lecteurs, auto, verrous gardés et attentes concurrentes restent différés.

Relecture ELF/lanceur puis autorisation runtime distincte requises avant suite.
Ticket ouvert ; aucun tour master donné.
