# Préparation source de la gate FS — aucune exécution

[Ticket ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Source **0668432241989dc9fac88e6df0e54e80c0f3e368**, branche
`cx-lucivy-fs-gate-fix-17nov`, dépôt lucivy, parent0f. Quatre fichiers,
339 ajouts/96 retraits. [Pins complets](preuves/fs-gate-source-0668432/source-pins.json).
Nouvelle branche dans le worktree propre dédié, aucun changement de l’arbre de Lucie.
Aucun Cargo/rustc/rustfmt/test/ELF/corpus/service/target ni contact PC dans
cette préparation. Contrôles réellement faits : lecture du diff, SHA des
sources, liste textuelle des dix tests, `git diff --check` sans défaut.
**Compilation et dix résultats non observés.**

## Prérequis conservés avant le diff

Les six rouges sémantiques et le positif de source0f sont archivés dans
bb5707c6a5 ; sources/outils/deuxELF conservés en RO sur les deux postes.
Recherche favorable aux preuves (80 entrées publiques et16 copies locales),
Banc favorable d32884b667 (mêmes entrées et16 copies privées PC ET portable).
Rapport24, consignations ab9e84d09b/e7979ba585. Aucun ancien brut réécrit,
aucun coupon utilisé pour ce diff ou réutilisable pour le futur vert.

## Contrat exprimé par le diff, à relire

Le raccord `with_v3_contains` réserve le reçu par create_new avant la capture.
Un reçu déjà présent refuse sans réénumérer, même si le premier verdict était
REFUSED. Le résultat est `Result<R, Refusal>` : plus de succès vide optionnel.
Les refus portent Empty, opération/path/kind IO, erreur UTF8/offset ou lignes
non couvertes du panel15 inchangé. Les erreurs d’entrées read_dir ne sont plus
flattenées. Un préfixe partiel éventuellement capturé est conservé dans le
reçu REFUSED, jamais transmis au consommateur.

La capture reste unique et non triée, cap5000 et exclusions fixés avant FS.
La politique **v2** explicite les changements : `.vault` et `.env*` exclus avant
metadata/read, symlinks non suivis à metadata, racine répertoire non liée,
identités de chemins UTF8 sans perte. Les règles antérieures restent taille
1..100000, exclusion NUL/binaire et texte blanc. Lecture bornée100001 pour
refuser une croissance au-delà du plafond ; sous Unix ouverture O_NOFOLLOW et
O_NONBLOCK, puis contrôle fichier régulier sur le FD. Cela ne revendique pas
une résistance à une mutation hostile des répertoires ancêtres.

Le reçu schema1 inclut plan/pins ordonnés/tailles/SHA/panel/verdict/refus ;
enveloppe SHA du body, sync fichier puis parent avant Builder. Une impossibilité
de réserver/écrire/syncer refuse la consommation. Les erreurs de configuration
qui empêchent un reçu sûr (sortie dans la racine candidate, racine sans identité
UTF8) refusent sans reçu ni consommation. La séparation sortie/entrée est
lexicale ; le parent de sortie doit être un emplacement de preuve connu,
existant et non aliasé sur le corpus. Aucun répertoire créé par le sink.

Le vrai `v3_ground_truth_contains` n’appelle plus `exists`/usable/ABSENT_OK,
max_docs/query_spec ni le panel qui relit le FS. Il refuse par présence les
modes V3, même vides/0, et LUCIVY_CORPUS_ABSENT_OK. Seuls V3_CORPUS et
V3_ADMISSION_MANIFEST sont des entrées d’invocation admises ; une nouvelle
invocation exige un nouveau chemin de reçu explicitement épinglé, pas la
suppression du précédent ni une relance pour chercher un meilleur préfixe.

`Snapshot::literal_spans` compose le noyau oracle existant sur les mêmes
buffers que le builder. Le panel natif est fixed15, distance0, sans ancrage,
plafond résultats=taille snapshot, comparaisons exactes docs/spans et doublons.
Les lignes du rapport sont écrites/syncées avant assertion ; le panel n’entre
pas dans les overrides diagnostiques.

`IndexBuildPlan` rassemble les paramètres du builder partagé. Le constructeur
correctness fixe schema3/positions, RAM, commits500, NoMergePolicy, sans merge
ni executor configurable et **sans chemin de persistance**. Les sites réels
reuse/delete/persist sont gardés par cette capacité absente, pas seulement par
les traces. Les autres diagnostics gardent leur constructeur à variables.
Le coût est un plan local de construction et un panel de correction explicite ;
aucune nouvelle logique moteur ni nouvel algorithme d’appariement.

## Témoins préparés

Les six noms et leurs critères de refus restent identiques, les trois I/O
restent distincts. Le positif ajoute reçu ADMITTED présent dans le callback,
SHA des octets originaux, mutation du fichier après capture sans changement
builder/oracle, read1/builder1. Le reçu complet et son SHA externe sont imprimés
avant retrait du petit artefact ; son hash interne est vérifié séparément.

Trois témoins complémentaires sont préparés : présence flags vides/0 avec
zéro opération corpus ; absence racine avec NotFound metadata sans skip ;
réutilisation du reçu après refus sans recapture ni réécriture. Ils n’ont pas
été joués sur la baseline ou le fix : aucun rouge/vert indépendant ne leur est
attribué. Les six rouges acquis restent la preuve red-first de la correction.
Leur éventuelle exigence de rouge préalable distinct doit être tranchée avant
un GO, jamais masquée dans le bilan du futur vert.

## Recette suivante proposée, pas un paquet exécutable admis

D’abord RO TOTALE source/contrat par Recherche et Banc. Préparer ensuite le
nouveau pilote et son paquet exact : le pilote RED ne convient pas, puisqu’il
exige encore six FAIL. Aucun lancement sous ses anciens GOs.

Même T unique PC inode164536077/marker5a1671, même lock effectif/seed DBDB,
release/offline/locked, deux ELF issus du JSON compilateur. Préserver les deux
ELF0f et les trois anciens1ee avant compilation. Nouveau Q/nonce/préflight,
export autonome ledger copié/fsync/rehash/ACK avant tout ELF, garde finale du
lock effectif après juges ET au dernier verdict externe, mêmes watchdog/budget
physique300/CommonMIN/32G/HIGHmax/swap0/quota4/jobs4/affinité moins2. Le GO doit
inclure acquisition, compilation, transferts, juges, nettoyage et vrais retours.
Pas de nouveau target, purge/import, contact corpus, service ou nettoyage ici.

Le futur paquet léger vise dix noms exacts de test_fs_fixture_admission :
chacun1PASS/0FAIL/0ignored/9filtered, rc0, aucune erreurinfra ou retry ; les six
anciens refus restent distingués par cause/trace/reçu, pas « ça rend Err ».
Conserver les reçus JSON+SHA dans les journaux et leurs copies avant suppression.
Second ELF test_sfx_v3_ground_truth seulement compilé/inventorié ; son corps
et tout index/corpus réel restent exclus de cette première proposition.
Un vert de ces dix ne qualifiera pas seul le builder moteur réel ni le corpus.

Le **replay depuis manifeste** (mêmes paths/ordinals/SHA, sans sélection) reste
à implémenter et prouver sous le ticket : aucun lecteur de replay n’est ajouté
par ce diff. Le reçu initial et son exclusivité ne sont pas renommés replay.
La parité native, les frontières contre alias de sorties, les nouveaux témoins
et tout autre constat de RO restent à résoudre avant de revendiquer l’ensemble
du contrat FS. Parent core/default-profile reste ouvert : l’absence devient un
refus de ce test, pas un vert CI ou une qualification des neuf HOLD.

Porteur Lucivy ; actions RO source demandées sous ce même ticket, puis paquet
exact/RO runtime/nouveau GO distinct. Aucun main/tag/publication/PR42/A-B/ULP7.2.
