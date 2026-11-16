# Le profil TMP privé dépasse la longueur des sockets des témoins du pont

- **État** : corrigé par profil pilote court qualifié f7ef/f812 puis union01f3 ; preuves465d68a678, proposé à intégration avec parent.
- **Famille** : produit.
- **Gravité** : blocage de qualification, trois témoins de lib rouges sous profil privé long.
- **Atteignable en service** : non établi ; observé dans les fixtures seulement.
- **Touche rag3weaver** : oui, tests pont_navigateur/temoins_relai.rs et profil du pilote mémoire.
- **Pour** : mémoire pour diagnostic/proposition de profil, C7 pour arbitrage ; Archi pour le pont si défaut produit établi.
- **Signalé par** : mémoire, [preuve rouge04](https://github.com/L-Defraiteur/rag3db/blob/2a068b49c404008e3523efa335c44cf0c8a680e2/docs/memoire/16-11-2026/04-preuves-rouge-script-fixtures.md), après GO C7 exact734087/383042.

## Constat et liens

Fils du [ticket de qualification codex](2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md).
Les sept tickets instables listés dans passation ED2.8 ont été relus : aucun ne
vise ces trois témoins sockets/profil long ; le ticket générique du pont Archi
2026-11-12 est une capacité MCP distincte. Pas de doublon ouvert identifié.

Témoincodex6/0/0/F8 8/8 acquis, lib entière STOP sur FAILED relai écoute,
secondhôte et nouvelleconnexion. Le buffer ne livre pas le détail de panic avant
le STOP : errno exact NON OBSERVÉ. Aucun lien causal ETXTBSY ni défaut du pont
production déduit. NAV/seconde lib NON JOUÉES, ticketparent reste ouvert.

TMPDIR84octets ; vrais dossiers pont-986542-N conservés ; chemins sockets
108octets, ou115 avec ecoute/. dossier() utilise temp_dir, malgré son commentaire
« court : un chemin de socket Unix tient en108octets ». Géométrie incompatible
avec un chemin socket Unix terminé par NUL dans sun_path[108] : cause de profil
plausible, à confirmer par une preuve ciblée autorisée ; pas de message errno inventé.

## Suite à arbitrer avant toute édition du pilote

C7 demande uniquement diagnosticRO et snapshots : profilHOME/XDG/TMP/DBUS,
UID/modes/inodes des dossiers existants, source/pilote/limites et versionNode.
Ne pas rejouer les témoins ni adapter le helper avant retour de RO C7.
Proposition éventuelle : TMP privé court propre au reçu, possédé et nettoyé par
le lot, mêmeENDoriginal et resteaprèsposte, cache porté avec provenance stricte.
Ce serait nouveaupilote/RO/GOdistincts, pas un reset7200 ni un contournement du rouge.

Critères de fin : cause qualifiée sans mélange avec ETXTBSY ; profil ou fixture
corrigé au périmètre autorisé ; rouge/vert ciblé puis qualification globale
requise. Conservation du premier rouge et des26preuves, aucun retrait de cache.

## Accord C7 de préparation seulement, avant code

C7 autorise un témoin AF_UNIX privé distinct court/107/108/115octets et
UTF8, résultats/errno archivés sans attribuer au vieux panic perdu. Aucun Rust
rejoué. Préparer ensuite TMPcourt exclusif0700/UIDneuf sousR surdisque,
P/Q/Sv2neufs, mêmeTsource734 avec reçuTERMINE+SHAELF/natif+marqueur/identité,
sans purge/repriseFP. END1186775.678574093 et INITIAL1179575.678574093
originaux conservés ; reste après poste, aucunreset7200 niGO de reprise.
Deux étatsXDG/RAG restentfrères. Publication du nouveaupilote avanttémoins
purs ; ROcouple et nouveauGOC7 restent requis avant tout Cargo/runtime.

Prépa publiée eb2cb15691 : sonde AF_UNIX sur disque confirme107octets acceptés,
108/115refusés (PythonerrnoNone, rawbindNUL22/EINVAL), UTF8 mesuré en octets ;
aucunerrno attribué au vieux Rust.22fonctions Pythonhermétiques vertes,
préflightcapture readonlyPC OK/8647FP hashées, T734inchangé.
[Recette concrète05](https://github.com/L-Defraiteur/rag3db/blob/2a068b49c404008e3523efa335c44cf0c8a680e2/docs/memoire/16-11-2026/05-recette-tmp-court-reste-original.md).
P/Q/Sv2/TMPexécution ABSENTS ; nouveauGO exact C7 NON reçu.

## Réserve ED de publication avant correction

Pilote13a093 retiré, jamais lancé. ED2519763ef8 a consigné avanttransmission
que finalize vérifie END avant save mais save ouvrex puis dump sans nouvelle
comparaison : une ouverture retardée pourrait publier GREENaprèsEND.
Correction : comparaison APRÈSopen et immédiatementavantdump, résultat RED
si tardif ; témoin réel finalize/save avec ouverture artificiellement retardée
prépublié puis rouge avantfix. Aucun budget/PQSv2 consommé, autrecode inchangé.

Premier essai4c45976c38 nonqualifiant pour ce rouge : harnais finalize({},0)
oublie la clépilot, donc retourne REDsurKeyError avantpublication. Aucunfix du
pilote effectué ni vert de correction revendiqué. Corriger d'abord ce harnais
et exiger le motif deadline, puis observer rouge réel avantfixsave.

Profilcourt qualifié par première libcanonf7ef puis second-onlyf812 : trois
ancienssocketFAILEDv1 passent ; deuxlibs2596/0/72, sameELF, TARGET/MODELPREUVES
rapport08. Aucunfixproduitdu pont ; ticketfils reste ouvert jusqu'àRO/intégration
pilotes avecparent, aucunGO/retrait implicitementdéduit.

## Clôture proposée après qualification d'union

ProfilTMPcourt0700surdisque+identity+UTF8<=107, statesfrères ; sondeAF_UNIX
privée établit107OK/108-115refus, sans prêter un errno à l'ancienpanic. Trois
ancienssocketsrougesv1 passent dans deuxlibs de734 puis deuxlibs d'unione272
2605/0/72, mêmesnoms/ELFc397. [Rapport11](../../memoire/16-11-2026/11-preuves-union-deux-libs.md).
Aucunfixproduit pont nécessaire ; montageCARGOMANIFEST distinct déjàconnu ticket
ancres est corrigé danspilote. Parent clos dansle même candidat, pas de ticket
ancresferméabusivement, pasGOmaster/retrait déduit.
