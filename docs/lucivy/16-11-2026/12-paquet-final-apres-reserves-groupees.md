# Paquet final du témoin rouge — réserves groupées, pas de GO Rust

Même ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Cette version remplace le paquetv1 de la recette11 pour un futur GO éventuel.

## Relectures et correctif du pilote

RO Recherche SOURCE81b0 favorable bornée, sans réserve source bloquante :
consommateur réel raccordé avant scratch/build, onze QueryConfig/modes/ordre
identiques, témoin mémoire50return0 discriminant statiquement. Couverture
volontairement absente en baseline, code noncompilé/nonjoué. Cœurs et offsets
cohérents ; fuzzy partage sa définition avec le moteur. Enveloppes FS/lossy et
loader réel historiques inchangés, PAS une admission physique déjà qualifiée.

RO Banc publiée7f6f1d2407, `docs/banc/16-11-2026/13-ro-paquet-lucivy-admission-rouge.md` :
cohérence du paquet précédent, deux réserves groupées avant GO. Les deux sont
traitées dans le même diff final du pilote :

- Les trois ELF réels sont rehashés après le témoin et comparés au ledger initial.
  HEAD/propreté, copies figées et préflight sont réattestés sous la même échéance.
  Le SHA du résultat vient de ce rehash physique final. La copie RO106fa est
  préservée sousQ ; son ancien chemin sousT n’est pas exigé intact après compile.
- Chaque publication vérifie le budget avant ouverture, **après ouverture avant
  json.dump**, puis après flush/fsync. Un dépassement refuse le résultat ; le
  verdict reste de toute façon pendingCommon avant fin externe.

Le pilote final unique est `qualify-derived-admission-red.py`, SHA
**7833a6f4bc42c137c1766aba889f529b006a27ee72402d6227c22b9defdad0fc**.
Admission7649a9b9d34ec3cdfc1333f33bc216f260fbda9a6d52f3c70fe979d7aabff5e2
inchangée. APP **81b0e74f0254c8216864f90a460f8969842432ed**, branche distante
cx-lucivy-corpus-admission, inchangée après sa RO. Aucune query ou donnée permutée.

## Rouge puis vert des gardes Python seules

Autorisation C7 limitée aux témoins Python AST hermétiques timeout60/failfast,
sans Rust/Cargo/ELF, processus réels du code testé, proc, corpus ou builder.
L’entrée Rust et son témoin mémoire restent entièrement NON JOUÉS.

[Preuves des gardes](preuves/derived-admission-final-guards/SHA256SUMS).
Avant correction, pilote99a8 : **un échec réellement vu**, copie modifiée non
refusée avant le résultat final (AssertionError not raised). Cinq cas nonjoués
par arrêt au premier échec ; ne pas prétendre six rouges observés. Après
publication du pilote correctif272c3fea55 : **six tests/0échec/0erreur**, un seul
passage vert, couvrant copie/ELF/HEAD/propreté, ouverture tardive et cas inchangé.
Les fichiers, horloge et appels processus des fonctions testées sont simulés,
seul le harnais Python est exécuté sous timeout60. Le résultat ne qualifie pas
la compilation ou l’admission corpus du code81b0.

Incident d’outil conservé : la première application du patch avait retiré
`docs/` du chemin par le strip1 de git apply. Elle n’a rien modifié ; commit13b0b802e3
contient les preuves rouges seules malgré son message. Diagnostic, contrôle
`git apply --check -p0`, puis application-p0 vérifiée. Le vrai correctif est
272c3fea55, publié avant le seul vert. Aucun test relancé après un rouge inattendu,
aucun amend/force, ancien paquetv1 conservé. Les empreintes réelles font foi.

## Paquet PCv2 et provenance

Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v2`,
UID1000/dev55/inode168509414, nonce neuf
**d4f2d8cfd6d6705b2c17423ba66d91a9**.
WPC `/home/lucied/git_workspaces/cx-lucivy-corpus-admission-16nov` propre81b0.
[Préflight/copies et relevé partiel](preuves/derived-admission-source-preparation-v2/SHA256SUMS).
Préflight SHA **0c90301ed5324f8a07d80641283e581ed3f427bc6355dcf1304afa14763f7325**.
Neuf copies figées incluent les deux pilotes, helperscore2fa59/rouge0a269,
Common6439/poste79e3 mode755, ELFancien106fa RO, source-ledger et transcription11.
Le ledger de sources décrit les six fichiers au moment de la préparation ;
source_compiled=false est l’observation historique, pas un verdict futur.

Même T unique3552, UID1000/dev55/inode164536077, marqueur5a1671 intact,
ancien ELFcore106fa physiquement vérifié avant copie RO et avant toute compile.
Ancien Common3528029/start119072861 TERMINE101/restes[]/nettoyé. Précontrôle
metadata seul du paquetv2 : **12UNKNOWN nonattribués** conservés, aucun
owned/attribué actif ; pas de quiescence universelle revendiquée.
Scope/logs nouveaux absents, scratch vide, aucun Cargo/ELF/source Rust exécuté.
Les originaux et le paquetv1 ne sont ni purgés ni réutilisés comme nouveaux logs.

## Commande proposée600 — RO delta puis GO distinct requis

Le budget et la sélection restent ceux de la recette11 : vrai poste,
32G/HIGHmax/swap0/quotaCPU4/j<=4, START unique après poste/END=START+600, confineur
sur le reste moins2s. Compiler **sans exécution** les trois targets du raccord
et de l’extraction, écrire durablement le ledger3ELF avant tout inventaire,
inventorier tous les trois puis écrire le ledger d’inventaire complet avant
l’unique `--exact missing_panel_is_rejected_before_any_builder` mémoire.
Attendu :101/0passé1échec0ignoré, compteurleft1/right0. Les huit autres témoins
Rust et les deux targets natives ne seront pas joués. Tout autre écart STOP,
pas de retry aveugle, pas de sourcefix avant observation du rouge discriminant.

Parent et shell cwdQv2 obligatoires, bash explicite et noclobber :

```bash
ssh -o BatchMode=yes luciepc 'cd /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v2
exec bash -s' <<'BASH'
set -euo pipefail
set -C
test "$PWD" = /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v2
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 POSTE_CPU_QUOTA=400% RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v2/support/poste lourd timeout --signal=TERM --kill-after=5s 610 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v2/admit-derived-admission-red.py > launch.log 2>&1
BASH
```

NON lancée. RO delta demandée à Banc sur ce nouveau pilote7833/paquetv2 seulement,
source81b0 déjà relue favorablement. Après retour, C7 garde le GO exact600.
Le résultat attendu resterait provisoire jusqu’au Common terminé/nettoyé et au
scan final owned, tous contrôles sousEND. Rien ne qualifie le core entier,
neufHOLD/ignorés, les corpus réels,7.2/ULP, les anciens trois warnings ou PR42.
Pas de réactivation de deadline, de nouvelle sélection, builder natif, main/tag.
