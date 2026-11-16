# Protocole A/B corrigé : qualification CPU seulement

Le 16 novembre 2026, le lot CPU hermétique corrigé est passé : **11 tests,
0 échec, 0 erreur, 0 skip**. Aucun Cargo, test natif, lecture du /proc réel,
autorité, bus, service ou mutation du target n'a été lancé par ce lot.
La sonde historique et la passe complète restent **non jouées** ; REGLES reste7.
La relecture totale Banc et un GO natif distinct restent nécessaires.

## Sources et refus conservé

Réserves groupées enregistrées avant diff dans le ticket au commit770f0ffd2c.
Paquet complet publié1916b3553f (pilote, scanner, tests et recette), puis son
unique lot CPU autorisé a refusé : rc2 en0,065137s, zéro test joué. La commande
passait -f alors que unittest.main(failfast=True) désactivait cette optionCLI.
Ce refus du harnais n'est ni une qualification ni onze échecs de tests.
Preuves sur **les deux hôtes**, sans écrasement :
`~/.cache/codeparsers-notes/rust-probe-protocol-1916b3553f/`.
SHA result.json :611962687c56af984f31bb144f29f1025a4244f8b54edda58ddf5f5b21aa5acc.

Correction d'appel et diagnostic publiés avant relance au commit exact
**c1cd48750f8178e68d24a3570bc320b458cad7d1** : unittest.main() laisse l'option
CLI-f activer failfast. GO renouvelé explicite **C7-PARSERS-PROTOCOL-CPU2-16nov60**,
une seule exécution du même lot ; aucune répétition supplémentaire.

Le présent rapport est publié sur une branche documentaire distincte,
cx-codeparsers-protocol-report, pour garder le HEAD candidat c1cd et la
préparation pair-02 épinglés, propres et cohérents avec le jeu observé.

## Commande et preuves CPU2

Depuis le worktree privé cx-codeparsers-rust-fix, source exacte ci-dessus :

```sh
timeout --signal=TERM --kill-after=2s 58s python3 -B docs/codeparsers/16-11-2026/rust-probe/test_protocol.py -f -v
```

sourceSHA,argv,started et allocation ont été enregistrés dans command.json avant
l'appel. stdout/stderr bruts ont été ouverts avant le processus ; résultat et
elapsed enregistrés ensuite. Retour0, **0,071854s** mesuré extérieurement,
11tests en0,003s selon unittest, tous vus exécutés. Les tests manipulent des
JSON Cargo synthétiques, fichiers /proc factices, horloges simulées et fichiers
temporaires ; ils ne lancent aucun processus testé ni programme natif.

Répertoire identique aux deux hôtes, copié puis SHA comparés :
`~/.cache/codeparsers-notes/rust-probe-protocol-c1cd48750f/`.

| Fichier | SHA256 |
|---|---|
| command.json | 6ea88698df706ddbab13cdeaa25ec6465aae6450e6f729b5425c03e11dee8b02 |
| stdout, vide | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| stderr, 11 noms et OK | 44bae5a68a0039a35343c791d705585770dee3a18998070c6c039b4adf7d8640 |
| result.json | d87aea28824cd6a9159558a2aa0816a7e8cae3e4879bca7eb4066960c161bcdb |

## Delta total à relire après RO052d8a5377

1. **Nouvel ELF** : compilation --no-run et JSON Cargo ; artefact unique de la
   bonne source/test/features ; copie hors target et hash ; ledger de build et
   copie ELF publiés/recontrôlés sur deux hôtes avant --list. Inventaires all et
   ignored exacts puis ledger runtime/inventaires publiés avant le test. Deux
   ACK distincts sous le délai courant ; rehash des deux copies jusqu'à la fin.
   Les ELF118 acquis sont conservés, mais ne remplacent pas ce nouvel ELF.
2. **Borne complète** : adaptation recherche af11, vrai poste puis timeout1798s
   et kill2s, Common1750 à l'intérieur. Contrôles corpus/sorties/sources/ELF,
   contrôles physiques et classification sont composés dans le même enfant
   borné ; MIN(ENDCommon,ENDtimeout). Publication TTY256 après open puis fsync,
   un vert tardif réécrit ROUGE/phase_completed=false. Maximum proposé pour les
   deux phases natives :3600s. Rapport comparatifCPU distinct et explicitement
   alloué. Aucun verdict ni scan après le retour de la commande extérieure.
3. **Identités et accès physiques** : contrôleurs copiés hors W/T, entrée exec
   avant parent persistant ; aucune exemption de chemin/FD d'ancêtre. Scans
   frais avant Common, après-poste et finaux, attribution incrémentale
   exe/cwd/fd/argv/cgroup avec PID/start/uid avant/après. UNKNOWN attribué et
   disparition après indice protégés conservés. Seuls compose courant et
   parent timeout à la signature exacte peuvent correspondre aux contrôleurs
   du cgroup, jamais par une simple parenté. Lecture finale des ownedPID/start,
   superviseur terminé, bus et répertoire privés absents ; nouveau scan après
   rehash et analyse des sorties. Tout écart refuse.

Les 11 tests couvrent unicité/source/features d'artefact ; construction exacte
de la borne ; publication déjà expirée ; fsync franchissant END ; attribution
argv/CG sans exe/cwd ; absence d'exemption FD poste ; correspondance stricte
PID/start/argv des contrôleurs ; permission partielle après attribution ;
disparition après indice ; ACK absent ; premier rouge immuable.
Ils ne constituent pas une exécution de la composition Linux réelle.

## Préparation concrète à relire, jamais armée

Pair-02 préparée seulement sur PC, puis copiée sur portable, même SHA :
`~/.cache/codeparsers-notes/rust-probe-pair-02/`.

- prepared.json :f919bfacb4980cee40cfa966b3f46492c0356f3bcc32a61356b823fbd883eb1f.
- runtime-profile.json :81969f85c1cc6384ef67283fb6486cff22bbbcc8c5a795026339b7dbb13c4192.
- Candidat c1cd, production118/parser34 ; référence fec351, productiond395/parserbd.
- Pilote :66b8194131423f869f506c55a5fe62740f7cafc67fc1cd69a9144fabfe66297e.
- Scanner :c7ef7b43b99922cd0e51f2c269443909044a84775f91f2369c00f2a2a52367c7.
- Tests :3d62e07556ceb6c9f29e9feefb0920d0f8f848383414b51af4e173af2ccdde01.

Préparation sans allocation/autorité ni compilation ; pair-01 préservée.
Le targetPC garde le propriétaire acquis fix/extension/rag3weaver après END118 ;
le target portable31GiB n'a pas été touché. SDK59, corpus physique1840 et sonde
instrumentée sont inchangés. Les SHA de configuration ne sont pas des résultats
natifs. Ticket principal et suite de qualification restent ouverts.
