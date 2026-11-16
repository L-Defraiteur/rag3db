# Rust7601 : unités et usages verts, sonde historique encore à jouer

Source exacte jouée118cc71eea57ebdfa4c79ae01d5d5a0caf181d4b ; le produit reste
celui de7601, complété uniquement par la préparation documentaire. Parseur
34c74e41b2e46570cdbe7d3e6d4218852a1abe1d, lib59ac9c86bc du magasin réel,
SOb7e9fc75...2101/vector1669d639...7561, SDK exporté par le vrai resolver.
Préparé SHAfc7b60207b50c93e3771ff17d565d9611a469866d8f023b52539e4b8e6d4c580,
pilote SHA4f438ce314081fb24ab31d2664ddfed15601fd96e7625e16cee59df44c1ff7ae.

## Vérifications réellement exécutées

| Vérification | Résultat |
|---|---|
| Liste/chargement des mots vides Rust | 1passé,0échec,0ignoré ; compilation4,55s |
| code::reference_path_tests | 1passé,0échec,0ignoré ; compilation native40,79s |
| Filets du binaire E2E | 2passés,0échec,0ignoré |
| Toute la suite e2e_usages | 41passés,0échec,0ignoré en132,64s |

Les quatre témoins requis sont exécutés exactement une fois : std seul ne relie
pas son homonyme projet ; un appel Rust nommé comme un mot clé JavaScript reste
une référence ; les deux ordres chemin std/appel projet nu sur une même ligne
conservent l'appel projet. Le helper vérifie également colonnes UTF-8 et frontières
d'identifiants. Le test lib a filtré2118autres tests ; la suite usages a filtré
les637tests des autres suites du binaire commun. Aucune de ces suites non demandées
n'est déclarée jouée. Les tests usages utilisent HashEmbedder64 et des catalogues
en mémoire, dont deux lisent nos src/dataflow ; aucun service ou base de Lucie.

## Portée, adoption et fin

Allocation neuve C7-RUST-FOLLOWUP-118-16nov,900s à luciepc ; rien n'a repris
l'ancien budget du vert ciblé2/2. Scopevalidation-k4rax94q,PID3331048,
échéance monotone1190561.174354277, commande lancée depuis les preuves.
PQS après vrai poste : found=[], puis garde réelle sur le même target PC.
Propriétaire final : cx-codeparsers-rust-fix/extension/rag3weaver, après passage
par son sous-module pour l'unité parser. Pas de deuxième cache, ni target portable.
Enveloppe effective relevée : memory.max34359738368/highmax/cpu.max=500000 100000.
TMP disque et XDG/state privés frères, pas de modèle ni PostgreSQL ; SDK/pool1GiB
issus des primitives existantes. La charge affichée par run_e2e est celle du poste,
pas une attribution GPU/RAM au test ; le profil du test est CPU/Hash64.

Fin canonique : VERT/code0/erreur null/restes[]/nettoyé true, autorité terminée,
privé/tmp/r3v-pkpfl8i1 absent. Classification followup_units_and_usages=true,
historical_probe_run=false/global_pass=false. Aucun rouge ni retry dans ce créneau.
Le premier rouge des deux ordres reste archivé056f ; le vert ciblé7601 et cette
qualification complémentaire ne remplacent pas la sonde ni la passe complète.

## Preuves et exécutables

Dossier `~/.cache/codeparsers-notes/rust-followup-118cc71/` aux deux postes,
logs/commandes/codes/préparé/verdict et statut d'autorité sans token conservés.
Les trois exécutables exacts sont copiés dans bin/ avant toute nouvelle adoption :

- stop-words :2cfa5e879fac8f69a424e3c54827f4d853701ae51915e220c2412524a78125cb ;
- reference-path :f785a0c6b4afa4d5307800df1acdf8c477f881034ccfec7c56f336b31dc326ba ;
- usages :17351ca564127ab8659c382b53826aa9a8e780585910ed3824b92ee8c47362fe.

Classification SHA b6c3da20f98fe5705e1dbfa36d4c0021a1a289b8b3cae21bb9f96941e43c412d ;
stdoutstopwords1ad4beb1adfca43bb6800815f961314330e95de5c5682c2833d0914bfb880d26 ;
stdouthelpercd518fa72fd87ad1b3a780356736eefe6ee67c480e7a20a2a1f59d2658d3bb2b ;
stdoutusagesa713cafc10a3eb40a03098731e3c265f09dddc1a2bfa438e4245d19c03c48a9c ;
verdictcanonique366ebcb645c1add8e9573937a0e760c2c7ec659ca74dd9a667cd4247caf660ad.

## Ce qui reste

Les14sites/18couples historiques et la sonde290rw/1550engine ne sont PAS JOUÉS
dans ce profil. Leurs sources physiques sont figées aux deux postes, manifeste
0a6c9af21e3666ff95427ecb44ea253d10907cd5619f96b0a96e91e6c1297185.
Il faut un couple baseline/candidat frais, une borne et une allocation distinctes ;
les TSV de l'ancienne lib373 ne certifient pas une parité actuelle avec59.
Ensuite seulement REGLES8 et fermeture des tickets selon preuves, puis rebase
sous un nom neuf et passe complète sur le master entrant. Ni global vert, ni GO
master, ni ingestion revendiqués ici ; REGLES_DE_L_ANALYSE reste7.
