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

## Pilote concret demandé par C7, avant implémentation

Le16nov C7 autorise la préparation/petits CPU, pas Cargo avant couple publié
et GO exact. Source r3 `ff37eb82e52aa733503f3bf60fbf7a4d052ead85`, base
`f83c6c4dd6a40200883c6325326f093ecdba3e23` : six patches égaux à r2.
Pilote séparé : portée unique7200 après vrai poste32G/HIGHmax/swap0/quota
par place/j8 maximum ; P/Q/S neufs horsW, target unique neuf, pin59ac/magasin
réel. Canonique SUITES=- et codex_threads, puis seconde lib entière DIRECTEMENT
sur le même ELF large identifié dans lib.log, SHA avant/après ; aucune seconde
F8/NAV. Résumé canonique intact, deuxième journal et verdict composé séparés.
Arrêt premier rouge, cleanup par confineur, contrôles frais après hashes et
avant publication, autorité terminée/bus absent/quiescence. Aucun manager
7970/base de Lucie requis. Préparation ne vaut pas GO ni qualification Rust.

Gel corrigé après entrant ties C7 : branche NEUVE r4, source734087a4e99d54331aac2f6321df9ef6da3b6d9c
base d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6, six patches égaux ;
pilote NEUF r2 porte même qualification7200. R1 jamais lancé, aucun reçu.
Huit fonctions Python hermétiques vertes0,002s, pas qualification Rust.
[Recette concrète03](../16-11-2026/03-recette-script-fixtures.md).

## Réserve ED avant correction du watcher

Pilote1e3a3d005d RETIRÉ, jamais lancé. ED a identifié que P.rglob('*.log')
parcourt aussi XDG_CACHE_HOME=P/cache : des journaux historiques de gabarits
(llm-gemini.log et test result FAILED) peuvent y être copiés pendant les tests.
Un P neuf ne rend pas ces contenus contemporains. Réserve ED consignée aussi
par ED3d2d6cfffd. Correction attendue : liste EXPLICITE des seules sorties
actives canonical-pass/official témoins,F8,preuve,lib,NAV,résumé/second-lib ;
aucun scan de cache/state. Témoins sur vrai run avec fauxPopen : historique
FAILED ignoré, FAILED actif arrête avant phase suivante, should_panic préservé.
Publication avant essais, nouveau SHA soumis RO puis GO distinct C7.

## Rouge de qualification native, avant diagnostic correctif

Couple APP734087/PILOTE383042 exécuté au GO exact C7. Témoincodex_threads6/0/0,
F8 8/8 et preuve acquis. Lib entière arrêtée au premier FAILED du vrai journal
actif : pont_navigateur::temoins_relai::l_ecoute_range_un_pont_par_navigateur_et_le_socket_est_a_soi.
Deux autres FAILED relai apparaissent déjà dans le même buffer : secondhôte et
remplacementconnexion. ScopeROUGEcode1/restes[]/nettoyé, autoritéTERMINE ;
NAV et seconde lib NON JOUÉES. Aucun retry ni reset. Détails rapport04 à publier.

Diagnostic RO à établir avant toute modification : dossier() du témoin relai
compose std::env::temp_dir()/pont-PID-N/[ecoute/]pont.sock, limite108octets ;
TMPDIR de notre pilote sousP long pourrait dépasser cette limite. Le STOP
précoce a conservé la première ligne FAILED mais pas les détails capturés du
panic imprimés seulement en fin de lib : ne pas inventer son message exact.
Pas de changement produit autorisé par ce constat ; prochaine proposition
profilTMP court/privé serait un nouveau pilote et GO distinct sous ENDoriginal
si C7 le décide. Ressources et premier rouge conservés sans nettoyage.
