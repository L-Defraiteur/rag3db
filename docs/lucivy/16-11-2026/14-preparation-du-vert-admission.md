# Gate corrigé — proposition de neuf juges mémoire, aucun vert Rust encore joué

Ticket existant complété AVANT le diff :
`docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Rouge réellement observé et publié9a64654e7f, ROpreuvesBanc c195a6e5fd favorable
bornée (35SHA conformes) ; C7 autorise ensuite ce correctif de TESTS.

## Source corrective et limites

APP **1ee5b366ca67d7fe6926da232a6650d1b1d81407**, publiée sur
origin/cx-lucivy-corpus-admission, parent81b0. Deux fichiers de tests changés,
aucune API ou dépendance produit. Worktree portable cx-lucivy-corpus-admission.
Aucun Cargo/Rust/ELF/judge joué depuis ce correctif.

with_derived_fixture conserve les octets de la vue chargée, valide UTF8, prépare
les onze requêtes et cherche un témoin nonvide pour chacune via les cœurs
communs, **avant** d’appeler le builder. Les absences sont collectées et nommées
par label+valeur, pas seulement la première. La liste vide de spans est une
absence ; erreur Read, Utf8 ou Query (pattern/oracle) est une cause distincte,
jamais convertie en « aucun résultat ». Regex préparée une fois, RustRegex
insensible à la casse, spans vides exclus ; literal/relax/fuzzy/frontières se
composent avec les primitives du parent, sans nouveau moteur.

PreparedQuery est limité aux modes contains(content) du panel déclaré. Il ne
prétend pas interpréter tout QueryConfig. La définition floue reste partagée
avec le moteur, distincte du chemin index. Les enveloppes FS/lossy de ground_truth
et le loader historique restent inchangés ; **pas d’admission physique FS déjà
qualifiée**. Aucun corpus, ordre, seuil de quantité, common::usable, fallback
ou assertion de référence nonvide ne change dans cette correction.

[Invariants statiques](preuves/derived-admission-fix-preparation/source-invariants.json) :
onze QueryConfig/déclarations identiques (SHA240fd8...), mock négatif50return0
identique (SHA7c06d6...), neuf noms de juges identiques. Le témoin initial garde
son compteur0 attendu, maintenant refus précis avant builder, avec vérification
d’absences multiples et du retour couvert. Un contrôle **positif mémoire distinct**
couvre les onze requêtes et exige le passage au builder une seule fois, pour
écarter un faux correctif « toujours rejeter ». Il ne remplace aucun des50
négatifs et ne sélectionne aucun fichier du corpus réel. Les probes de pattern
invalide et décodage sont renforcés dans les juges existants, hors du panel fixe.

RO Recherche exacte1ee5 favorable bornée, aucun blocage source : modes/raccord,
causes d’erreur, refus négatif et positif séparé cohérents ; pas de compilation,
exécution ou qualification des neuf juges déduite de cette lecture.

## Pilotes proposés avant exécution

`outils/qualify-derived-admission-green.py` SHA
**6c701601a5057285c36420d964288060ed20da75173d359aa2f6ca553f9ae023** ;
admission `outils/admit-derived-admission-green.py` SHA
**22550ba19b772c120892cd8fcdf7fbfc8c60708251728e5419680cc8c08426af**.
SOURCE1ee5, anciens Common588652/start119729720 TERMINE101 et ELF rouge7c6f.
Copie RO de cet ELF requise avant compilation, aucun ancien SHA exigé sousT
après son remplacement ; copies et preuves précédentes sont conservées.

Même target unique3552/marker5a1671, pas un autre target par essai. WPC sera
avancé fast-forward vers1ee5 dans sa branche propre après fetch, jamais l’arbre
principal de Lucie. Q neuf prévu
`~/.cache/rag3weaver-build/cx-lucivy-derived-admission-green-16nov-v1`.
Nonce/scope/logs neufs et préflight à figer/publier après ces scripts ; aucun
ancien GO600/615 ou deadline ne peut être réactivé.

Proposition pour respecter le plafond~300MAX demandé : **enveloppe dure300s**
(TERM295+kill5), validation **285s** START après vrai poste incluant préflight,
Common pour le reste moins2, pas de reset. Mémoire32G/HIGHmax/swap0, quotaCPU4,
jobs<=4/affmoins2/quota hiérarchique, parent bash cwdQ/noclobber.

Le pilote reprend les gardes7833 (AST des fonctions publish/budget statiquement
identiques, aucun test supplémentaire exécuté pour le vérifier). Flux :

1. Contrôles frais source/copies/nonce/UID/inode/marker/oldReceiptTERMINE/ELFRO,
   scans owned/attribués, UNKNOWN nonattribués conservés avec relevé avant assert.
2. Compiler offline release **sans run** les mêmes trois targets mémoire/derived/
   ground_truth ; écrire et fsync le ledger3ELF avant tout inventaire, puis les
   trois inventaires/ignorés et leur ledger complet avant les tests.
3. Neuf appels --exact, neuf noms publiés identiques, un thread chacun, mémoire
   seulement : chaque juge doit donner1pass/0fail/0ignored/8filtered. Le juge
   antérieurement rouge est premier. Ces filtres sont par appel : à la fin les
   neuf fonctions auront été jouées, aucun « ignoré » repeint en vert.
4. Premier processus ou assertion inattendu STOP sans retry. Aucune requête,
   donnée négative, assertion de spans ou de référence n’est assouplie pour
   rattraper un vert. Les deux corps de tests natifs sont seulement compilés et
   inventoriés, jamais joués, sans index/builder natif ni corpus réel.
5. Rehashphysique3ELF/HEAD/propreté/copies/préflight et publication contrôlée
   avant/aprèsopen/avantdump/aprèsflush sous le mêmeEND. Résultat provisoire
   jusqu’à Common terminé/nettoyé et scan final owned après retour.

Aucun juge vert encore observé. Assemblage metadata du paquet puis RO Banc du
pilote/préflight et GO exact distinct C7 requis. NeufHOLD/full_core=false,
physiqueFS/corpus réels, Linux7.2/ULP, anciens trois warnings, PR42/main/tag
conservent leurs limites. Les preuves et ELF rouges sont conservés deux postes.

## Paquet vert préparé, aucune exécution

Q réel `/home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-green-16nov-v1`,
UID1000/dev55/inode168601424, nonce neuf **023317833614261f17091d13c8a4988a**.
WPC avancéFF dans sa branche propre vers1ee5, source propre/noncompilée.
Préflight SHA **277a637e9bebebeef15cd77d81c8d835db5a6c43f9baa06a829086b9dedba369**.
Neuf copies : green6c701/admit22550/core2fa59/rouge0a269/Common6439/poste79e3,
ELF rouge7c6f RO, ledger des six sources et invariants statiques. Même target3552,
ancien Common588652 TERMINE101/restes[]/nettoyé ; contrôlemetadata frais :
11UNKNOWN nonattribués conservés, aucunowned/attribué actif, pas de quiescence
universelle. Scope/logs absents, scratch vide. Aucune sélection de corpus.

[Paquet et relevés](preuves/derived-admission-fix-preparation/SHA256SUMS).
La commande candidate respecte une enveloppeDURE300 (TERM295+kill5), avec
validation285 et sans nouvelle échéance à mi-parcours ; **NON lancée** :

```bash
ssh -o BatchMode=yes luciepc 'cd /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-green-16nov-v1
exec bash -s' <<'BASH'
set -euo pipefail
set -C
test "$PWD" = /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-green-16nov-v1
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 POSTE_CPU_QUOTA=400% RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-green-16nov-v1/support/poste lourd timeout --signal=TERM --kill-after=5s 295 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-green-16nov-v1/admit-derived-admission-green.py > launch.log 2>&1
BASH
```

RO source Recherche exacte1ee5 déjà favorable bornée ; RO Banc NOUVEAU paquet
green6c701/22550/préflight277a sollicitée sur le même ticket avant toute
exécution. Aucun juge vert Rust encore observé ; seul C7 peut donner le GO
distinct. Le rouge7c6f, l’ancien106fa et leurs preuves2postes restent intacts.

## RO paquet verte reçue

Banc014d5c7fb3, `docs/banc/16-11-2026/16-ro-paquet-vert-lucivy-admission.md`,
favorablePRÉPA sans autre bloqueur statique :5SHApreuves/6sourcesGit conformes,
neuf noms du pilote égaux aux neuf tests Rust distincts ; gardes budget/publish/
run AST identiques7833. Plafond TERM295/KILL300, validation285 après poste avec
préflight/Commonreste-2, ledgers avantinventaires/neufexacts cohérents.
Aucun Cargo/mock/ELF/proc/base/service par Banc, pas de vert Rust ou de GO300
déduit. SourceRecherche1ee5 favorable également ; C7 informée pour GO exact
distinct, finCommon/owned à observer après une future passe sous borne.
