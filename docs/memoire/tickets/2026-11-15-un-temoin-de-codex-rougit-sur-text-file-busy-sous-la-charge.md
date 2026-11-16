# Un témoin de codex_threads rougit sur « Text file busy » sous la charge

- **État** : ouvert, le 15 novembre 2026.
- **Famille** : produit. Le défaut est dans un témoin de la lib du crate ; le code de
  `codex_threads` n'est pas en cause.
- **Gravité** : un rouge intermittent de la lib entière, qui fait rejouer une passe. Il est vert
  seul.
- **Atteignable en service** : non.
- **Touche rag3weaver** : oui, `src/codex_threads.rs`, le test
  `un_codex_qui_ne_rend_pas_la_main_est_coupe_au_delai`.
- **Pour** : la session qui tient le pont vers Codex (pont entre agents de code), ou qui le veut.
- **Signalé par** : la session mémoire (96), dans la passe du lot `segment` du 15 novembre 2026 au
  portable.

## Ce qu'on voit

```
codex_threads::tests::un_codex_qui_ne_rend_pas_la_main_est_coupe_au_delai ... FAILED
/tmp/.tmpbxAo6B/lent queue : ne démarre pas : Text file busy (os error 26)
```

Il est rouge dans la lib entière (2472 passés, 1 rouge, 20 h 03), et vert joué seul juste après
(1/1).

## Pourquoi, très probablement

Le test écrit un script (`std::fs::write`), le rend exécutable, puis le lance aussitôt. Sous Linux,
`execve` refuse avec `ETXTBSY` un fichier encore ouvert en écriture par un processus. Les autres
tests de la lib forkent en parallèle : un enfant forké pendant l'écriture hérite du descripteur
ouvert, le garde jusqu'à son propre `exec`, et le lancement du script tombe dans cette fenêtre.
C'est la course classique d'un exécutable écrit puis lancé dans un programme multi-fil. Seule la
charge de la lib entière l'ouvre.

## Ce qui serait juste

- Écrire le script dans un fichier temporaire, le fermer, puis le **renommer** à son nom final avant
  de le lancer : un `rename` ne porte pas le descripteur hérité.
- Ou réessayer le lancement sur `ETXTBSY`, quelques fois avec un court délai, dans le test ou dans
  `livrer`. Un vrai binaire de codex n'est jamais écrit par nous, donc le réessai ne servirait
  qu'aux tests.
- Ou lancer par `/bin/sh <script>` plutôt que le script lui-même : le fichier n'est alors qu'ouvert
  en lecture.

## Reprise du 16 novembre : preuve avant correction

Lot `cx-memoire-script-fixtures-r1`, base `927941a67a5805450d49f1e04b39fdd542a6059b`.
Périmètre réservé : fixtures de `src/codex_threads.rs`, présent ticket et preuves.
Aucune modification de `livrer` ni de la logique de production autorisée.

Le renommage n'isole pas l'inode : un enfant qui garde le descripteur d'écriture
hérité bloque aussi l'exécution au nouveau nom. Le témoin Linux autonome
`docs/memoire/16-11-2026/script_fixture_probe.py` garde ce descripteur jusqu'à
un acquittement explicite, sans sommeil ni course probabiliste. `--legacy`
exige le lancement malgré ce writer et doit rougir avec errno26. Le second
cas vérifie que rename ne corrige pas cette cause. C'est une preuve système,
pas encore l'exécution des témoins Rust du crate.

Correction proposée : faire écrire les deux scripts par un processus enfant
attendu, de sorte qu'aucun writer du fichier exécutable ne soit ouvert dans
le parent multi-thread qui crée les autres enfants. Le témoin vert utilisera
la commande shell exacte du helper Rust de test, extraite du source ; pas un
réessai de livrer, pas une relaxation de délai. Lib/F8/NAV NON JOUÉES, attendent
FINI G et une recette compatible avec la lib nouvellement autorisée59ac.

Rouge système vu sur `bc07b0cb85` : 3 cas en0,005s, deux contrôles de cause
passent et le lancement attendu échoue exactement avec errno26. Journal horsW
`~/.cache/rag3weaver-build/journaux-cx-memoire-script-fixtures-16nov/rouge-bc07.log`.
Le correctif des deux fixtures passe par `/bin/sh -c` avec chemin et corps en
arguments séparés ; printf/chmod s'exécutent dans l'enfant seulement. Attente
bornée à5s ; aucun réessai et aucun changement du pont de production.
Un témoin Rust de chemin/corps littéraux est ajouté, NON JOUÉ avant FINI G.

Vert système sur40e7 : 3/3 en0,007s, commande du helper extraite du source.
[Rapport du lot](../16-11-2026/02-script-fixtures.md). Qualification Rust toujours
NON JOUÉE ; conservation du premier rouge et des limites de cette preuve.

Préparation magasin explicite sous XDG privé : contrôle résolveur seul portable/
PC conforme, pin59ac et so/vectorSHA dans le rapport. La future recette fixe
RAG3DB_LIBS_DIR réel sans copie SDK ; ce contrôle ne déclenche aucune passe.

## Union neuve après masterbef, avant pilote et exécution

C7 demande candidatNEUF avec codeEmbar entré après734. Branche
cx-memoire-script-fixtures-union-r1 depuisbef0 ; sixpatcheségaux, seulRust
codex_threads.rs mode/OID734 exact et préfixeproductioncfgtest inchangé.
Tousmasterhorschevauchementpréservés, audit/range dansrapport09.
[RecetteDISTINCTE3600proposée](../16-11-2026/09-union-master-et-recette-distincte.md),
F8/lib/NAV+deuxièmeELFmême, nouvellefenêtre pasresetancien7200. Pilote et
transitionGit archive734physique/mêmeC SAMEOWNER àpréparer ; zéro Cargo/target
ou sourcePC734 touchés, pasGOmaster/retrait. Preuves2a068 restent acquises734,
aucun report automatique àl'union avec 5Rust Embar entrants.
