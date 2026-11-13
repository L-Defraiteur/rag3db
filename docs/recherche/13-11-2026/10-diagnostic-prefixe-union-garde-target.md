# Préfixe union garde : refus avant reprise du cache

APP exact `9ec421069320b2f0cca4099fa27f83059e944de9`, pilote exact
`4d38e11409b3c8a9c110a2b1b6addfe619267d78`, sources propres avant et après.
GO C7 distinct : préfixe compilation seulement, ledger publié puis PAUSE avant
runtime lib/NAV/UI. Aucun GO runtime déduit.

## Premier rouge et cause

Commande owner réellement lancée : `python3 -B /home/lucied/.cache/rag3weaver-build/cx-wt-recherche-garde-target-union-pilote-r2/docs/recherche/13-11-2026/garde-target-union/lancer.py 4d38e11409b3c8a9c110a2b1b6addfe619267d78`.
Le `-B` ajouté par Recherche ne fait pas partie du argv exact autorisé par la
politique du lanceur. Celle-ci attend après argv0 seulement le chemin script
et le SHA pilote. Ce défaut de commande déclenche le refus strict, pas une
preuve d'un défaut produit. Aucun élargissement de politique ni retry effectué.

Phase verifier VERT, puis fresh_cache_guard ROUGE :
`exact launcher/poste ancestry not proved`. La preuve cache-after-poste archive
le poste reconnu PID1085829/start91459383, son FD script255 autorisé et le
lanceur PID1085766/start91459356 non reconnu (attributed-live).
Aucun Popen cache-reprise n'a été atteint : aucun Cargo, F8, build large/NAV,
inventaire de nouvel ELF, ledger runtime ou juge produit dans cette passe.

## Budget et fin des ressources

Reçu `validation-aya_mzq1`, START914593.915525602,
END919393.915525602 ; driver elapsed1.0736107600387186s.
Autorité PID1085860/start91459387 terminée ; scope ROUGE/code1,
restes[], nettoye=true. Bus privé `/tmp/r3v-1vqhp3xs` absent.
PID1085766/1085829/1085860/1085861 tous absents au contrôle frais.
Marqueur du target toujours sur source698 :
`cx-wt-recherche-garde-target/extension/rag3weaver`.
Le poste est libéré ; aucun service partagé/PG/modèle touché.
Le finaliseur composé ROUGE avec AssertionError est conservé lui aussi ;
il ne remplace ni le premier rouge archivé ni le verdict du superviseur.

## Preuves conservées

37 fichiers size/SHA comparés deux postes, zéro écart ; manifest SHA256
`6e1f7b6a2889cbc555fb0c62863a54fdff4203f9aa3786e41e756ca8277bf666`.
Copie locale horsW : `~/.cache/rag3weaver-build/cx-preuves-garde-target-union-v1-local/`.
Sources distantes P/Q/S union-v1 conservées intégralement ; journal et manifest
sous `journaux-cx-recherche-garde-target-13nov/union-4d38-{prefixe.log,manifest.json}`.
Les preuves antérieures698/v1/v2 et anciens rouges sont conservées.

Aucune reprise autorisée par ce diagnostic. Une éventuelle correction d'entrée
et recette de reprise doivent être publiées, relues et faire l'objet d'un GO
exact distinct avec P/Q/S neufs, sans reset implicite de l'échéance initiale.
