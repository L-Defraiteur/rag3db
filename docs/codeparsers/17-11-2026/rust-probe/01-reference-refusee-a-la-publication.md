# Référence P02 : arrêt à l'échéance de publication, aucun test natif exécuté

La phase référence autorisée par **C7-PARSERS-HIST-BASE-P02-16nov1800** est
**ROUGE**, sans relance. La compilation seule a réussi ; le premier ACK de
publication n'a pas été fourni dans le délai courant. Le pilote a refusé avant
la première invocation ELF : aucun inventaire, aucun test de sonde, aucun
résultat sémantique. Ce résultat ne remet pas en cause les unités/usages118 ni
les onze contrats CPU déjà prouvés. Il ne qualifie pas le moteur ou la sonde.

Le GO candidat **C7-PARSERS-HIST-CAND-P02-16nov1800** et le GO comparaison pure
**C7-PARSERS-HIST-REPORT-P02-16nov60** étaient conditionnés aux verts précédents :
conditions non satisfaites, aucune de ces deux commandes n'a été lancée.
REGLES7, tickets Rust ouverts, aucune passe globale, aucun master.

## Sources et exécution

Outillage/candidat propre :c1cd48750f8178e68d24a3570bc320b458cad7d1.
Référence propre :fec35198b64b7e9da0496de6b01a7232fa7a8390, parserbd133850f89d6b12a3ca2b2f6ce238785ed37e3a.
Préparation P02 :f919bfacb4980cee40cfa966b3f46492c0356f3bcc32a61356b823fbd883eb1f.
Profil commun :81969f85c1cc6384ef67283fb6486cff22bbbcc8c5a795026339b7dbb13c4192.
Corpus1840, SDK59, HashEmbedder64CPU, sources et copies contrôleurs inchangés.

Entrée SSH bash-s, cwd P02 hors W/T, exec direct du pilote copié Q ; TOOL_ROOT
passé par environnement, aucun parent persistant dont argv contient le worktree.
Vrai poste lourd, limite32GiB/high=max/swap0/quota5CPU, -j8, timeout1798+kill2,
Common1750. Les fichiers allocation.json,outer-supervision.json,controls.json,
envelope-observed.json et les scans préalables sont conservés.

## Chronologie monotone observée

| Événement | Horloge monotone PC |
|---|---:|
| Début compose | 1204294.751507139 |
| Compilation seule terminée, rc0 | 1204329.420694972 |
| Premier rouge : peer publication deadline | 1206030.11595931 |
| Publication classification ROUGE | 1206030.237478167 |
| END Common prévu | 1206045.038331869 |

Cargo rapporte une compilation de33,04s. Après celle-ci, l'attente sans ACK dure
environ1700,7s et expire à END−15, comme prévu. Aucun fichier build-peer-ack,
inventory-all/ignored, runtime-ledger, runtime-start, probe.stdout ni
probe.result n'existe. Le ledger de build et sa copie ELF existaient sur le PC ;
leur transfert sur le portable a été réalisé seulement **après le refus**, pour
conservation. Il ne vaut donc pas publication préalable valide ni ACK rétroactif.

La session SSH89603 a ensuite été vue fermée avec255/Connection reset by peer.
La collecte montre l'expiration interne ci-dessus ; elle ne prouve pas que la
fermeture SSH a causé l'absence d'ACK. Le raccord de publication dépendait encore
de l'intervention de la session ; aucun relais autonome n'avait été démarré.
Le défaut de coordination est consigné dans le ticket dédié du17novembre,
sans correction ni nouvelle tentative implicite.

## Fermeture et état à reprendre

Common validation-rvm1cxyv, superviseurPID1813013/start120429499 : autorité
**termine**, verdictROUGE/code1/erreur null/restes[]/nettoyé true. Le répertoire
privé /tmp/r3v-eg3lhyvm et son socket de bus sont absents. L'autorité brute et
son jeton restent sur PC ; seule authority-status.json filtrée a été transférée.

Le pilote rouge n'a pas atteint ses contrôles finaux verts. Une collecte
**postmortem distincte**, lecture seule et hors ancienne borne, a relevé les
identités owned et les accès exe/cwd/fd/argv/cgroup aux deux worktrees, au target,
aux cgroups ancien/nouveau et au privé : owned_alive[],owned_unknown[],found[],
UNKNOWN attribué[]. Cet audit n'est pas présenté comme qualification bornée et
ne change pas le rouge publié.

TargetPC propriétaire actuel :
`/home/lucied/git_workspaces/cx-codeparsers-rust-probe-baseline/extension/rag3weaver`.
Ce changement vient du vrai garde_target appelé pendant la compilation. La
préparation P02 attendait initialement fix ; elle reste une archive consommée,
pas une recette à réarmer. Target portable31GiB intact. Toute reprise nécessitera
un lot et GO frais, état réel du target pris en compte et dossier de preuves neuf.

## Conservation sur les deux hôtes

Racine commune :`~/.cache/codeparsers-notes/rust-probe-pair-02/baseline/`.
Fichiers bruts de premier niveau, ledgers et copie ELF transférés ; scope public
sur portable sous scope-public/{verdict,enfants,recette}.json. Aucun autorite.json
brut transféré. Les SHA suivants ont été comparés et concordent :

| Preuve | SHA256 |
|---|---|
| bin/probe-executable | 0c565ace3bf6bd0d972447466df2842610a9d76a6f59c5bc2928599bd7f8b63b |
| build-ledger.json | 86edbbaf52bea6181cf238e3671bb7931c9323a7a2bf30a6ceb17654048bb113 |
| first-red.json | 3d563bee4a59084d0aa02ffbeb5a0e4597a8b4d27ddb0e34b5fb94d708f7865d |
| classification.json | 077e33f1dff091c3a583ae1131841354677cc57c6b49f8455a17751410166a71 |
| postmortem-quiet.json | a917d1d894209c56f7be303b3d2028c311597cc65576eacb355cf727f2bd3555 |
| verdict Common | 1098d5edecee55cd3da4a11586dc3abf0471ee4a802f9bcc159721a6ca8c6555 |

Le SHA du nouvel ELF original sous target a aussi été relu sur PC et égale celui
des deux copies. Cet ELF n'a jamais été exécuté dans ce lot. Les ELF118 restent
séparés et ne sont pas employés comme preuve de cette sonde.
