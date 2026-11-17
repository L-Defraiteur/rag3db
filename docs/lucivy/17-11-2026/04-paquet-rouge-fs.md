# Paquet exact RED FS — préparation seule

**HOLD C7 avant GO : borne physique locale manquante. Voir [06](06-hold-borne-physique-portable.md). Ne pas lancer la commande ci-dessous.**

[Ticket porteur ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Source rouge inchangée **8b9435363dbd8ec0e3973f2368b641189f0726eb**.
RO Recherche favorable bornée source/recette c987957426. C7 autorise la
préparation du couple pilote/paquet, **pas son exécution**.

## Paquets figés

PC : `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1`,
UID1000/dev55/inode168823153, nonce **b4e4acbf19074499d160a96eca799b46**.
Worktree PC propre `/home/lucied/git_workspaces/cx-lucivy-fs-admission-17nov`,
branche propre `cx-lucivy-fs-admission-17nov`. Même unique target3552 ; aucune
création/purge d'un target supplémentaire. Ancien Common1021297/start119982634
terminé0/restes[]/nettoyé relu comme métadonnée, sans scan de processus.

Préflight exact **79adf115e7096644c32cf6be31286d89e03ce54c160fe5da538c933afeac15db**.
Neuf copies : pilote/admission/exporteur, deux helpers historiques inchangés,
Common/poste inchangés, source-pins et lock seed. [Préflight et préparation](preuves/fs-red-preparation/SHA256SUMS).

| Copie nouvelle | SHA256 |
|---|---|
| qualify-fs-admission-red.py | a5c74663cb2b783041fe4c76e80d2b75dcac1773521f99629b4d1e63d101c735 |
| admit-fs-admission-red.py | 166d3ff6ec640539eadeb0fe82d867fa3a6d539c5e321d39108681f2889a8b03 |
| export-fs-red-ledger.py | 0471118fe427d8e5a52685d02c6f30c51ae5ffa2b91937f78e2caa86adf63f6f |

Portable : `~/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v1`,
répertoire privé de preuve, **pas un target Cargo**. Exporteur et préflight
copiés exactement ; identité et empreintes dans
[son marqueur de préparation](preuves/fs-red-preparation/portable-export-preparation.json).
ELF ledger, launch-ssh.log et exported-final.json y sont encore absents.

Contrôles réellement faits : source/propreté Git, neuf SHA de copies, markerT,
lock seed, syntaxe Python par AST seulement aux deux postes ; scope/logs/arm/ACK/
sauvegardes ELF absents, scratch/scans vides. Aucun import des pilotes, aucune
admission exécutée. **Aucun Cargo/rustfmt/test/ELF/proc/corpus réel contacté**.
Les trois sauvegardes ELF sont prévues seulement après le GO, avant compilation.

## Choix de budget et export autonome

Une seule phase proposée, **300s physiques**. Pas de phase compile séparée puis
reset de budget. L'alternative compile300 puis RED60 demanderait deux GO et
deux portées terminées ; elle n'est pas retenue ici.

Le lanceur/exporteur portable est un seul processus : avant de lancer SSH/poste,
il pose un armement PC source/nonce, `launch_by=armed_at+15` et
`physical_deadline=armed_at+300`. Le code d'admission refuse si le vrai poste
n'est pas acquis avant launch_by. Un timeout physique extérieur calculé depuis physical_deadline inclut
aussi acquisition et retour de poste (TERM au reste−5, KILL au plus tard300).
Le timeout intérieur TERM295/KILL300 reste dans poste. L’ancien préflight
non exécuté a été remplacé avant publication ; la révision est tracée dans
les preuves de préparation. START admission est après poste, END = min(START+285,
physical_deadline−5). Préflight et toutes les vérifications y sont inclus.
Common reçoit floor(END−now)−12, donc réserve explicite pour verdict final,
scan owned et copies. Aucune nouvelle échéance après compilation. Plafonds
32G/max/swap0, quota4 et jobs<=4/affinitémoins2 inchangés.

300s est proposé d'après les passes précédentes sur ce même T : compilation
trois targets source81b en32,20s, puis source1ee en3,37s ; les sept nouveaux
juges sont de petits FS temporaires, pas des builds d'index. Le changement de
chemin de worktree et la dépendance directe peuvent invalider du cache : aucun
succès ou temps futur n'est garanti. Un dépassement reste une erreur de portée,
pas un rouge d'admission ni une raison de prolonger ou relancer automatiquement.

Après build **release offline** des deux targets, delta lock contrôlé et archivé
(seul ajout de sha2 à lucivy-core ; aucune version/checksum modifiée), ledger
physique deux ELF écrit/fsync puis marqueur ready. Avant tout inventaire, le
lanceur déjà actif copie ce ledger sur le portable, fsync fichier/parent et
**rehash de la copie**, puis ACK atomique PC avec source/nonce/SHA exacts.
Le pilote n'invoque aucun ELF sans cet ACK. Aucune autre session/personne ne
doit répondre en temps réel ; un message humain n'ouvre pas cette barrière.
Cette attente consomme END et s'arrête avec lui.

Inventaires des deux ELF, puis ledger complet d'inventaire, puis six négatifs
exactement nommés et le positif. Tous les négatifs exigent code101 **et** nom,
compteurs0/1/0/6filtered, assertion précise `missing precise REFUSED before
builder`, résultat Ok(None) ou Ok(Some(1)) et traces associées. Pas de déduction
à partir de six rc seuls. Le positif exige1/0/0/6filtered ; ses assertions
internes contrôlent panel15, builder1/read1 et hash des mêmes octets.

Fin : rehash2ELF/source/copies/préflight/anciensELFsRO, Common effectivement
revenu, verdict terminé/restes[]/nettoyé, nouveau scan owned/attribué avec
partiel publié avant assert. UNKNOWN nonattribués conservés, aucune quiétude
universelle. Rehash2ELF externe après Common puis final-accepted ; le lanceur
attend le retour réel poste/SSH et exporte final-evidence/accepted sous END
(contrôle du temps restant, pas de comparaison d'horloges interpostes).

## Arrêt explicite

Seuls les six échecs **sémantiques attribués** permettent de continuer vers le
juge suivant. Mauvais nom/cause/trace, code inattendu, positif rouge, skip,
compilation/résolution offline, delta lock inattendu, provenance, publication,
copie/rehash/ACK ou délai : STOP, jamais transformé en rouge attendu. Les preuves
partielles restent sur Q. Si l'exporteur échoue, aucun ACK humain de secours :
Common/timeout bornent la passe, attendre son verdict et constater la libération
réelle avant toute suite. Pas de nouvelle tentative sans arbitrage explicite.

Même quand le pilote rend0, `result.json` dit **six tests échoués et un passé**,
pas sept verts. Ce0 signifie seulement observation attendue des rouges, puis
nettoyage ; chaque trace/log et la preuve de fin restent nécessaires.

## Commande candidate — NE PAS LANCER sans nouveau GO

Le code et les copies sont prêts pour RO du couple exact. Une seule invocation
portable future déclenchera elle-même le lanceur/exporteur autonome :

```bash
python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v1/export-fs-red-ledger.py \
  --preflight /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v1/preflight.json \
  --output /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v1/elf-ledger.json \
  --launch-log /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v1/launch-ssh.log
```

Le shell distant est explicitement bash, cwdQ hors source/T, set-euo/noclobber.
Tous les V3_*, ABSENT_OK, flags rust et wrappers de compilateur hérités sont
refusés. Le lock seed audité est copié sans résolution ; toute adaptation est
faite uniquement par le futur Cargo offline et contrôlée après. Aucun corps
`v3_ground_truth_contains` ou autre natif exécuté. A/B, FS corrigée/read-once/
sink/flags, core complet, neuf HOLD, Linux7.2/ULP, PR42 et publication restent
non qualifiés. RO puis **GO C7 distinct** requis avant cette commande.
