# Contrat de nettoyage Vertex : préparation seulement

Le lot commencé16nov continue17nov ; aucuneancienneadmission n'est réactivée.
Master99fef92ce6ee9a0007eaf99c7c3a6a3789061c57 intégré et3contrôles lus2postes,
[landing26](26-landing-vertex-et-controles-deux-postes.md). C7 clôt le tour et
n'autorise ici QUE la préparation : zéro retrait/quarantaine/signal/force.

[Liste typée exhaustive](vertex-cleanup-contract.json), SHA
6f6f4f84559024c202f5e6828954f957bab042b2e5695ef9b6a52aa116d98742 :
14WTportable,7WTPC, leurs21adminsprivées, unTuniquePC. Chaqueentrée a
chemincanonique/HEAD/branche/UID/dev/inode et copies fidèles source+admin.
Toutes branches locales etdistantes restent conservées. Le WT de ce rapport
cx-ed-vertex-landing-docs-r1 resteACTIF/CONSERVÉ et peut évoluer ; sonHEAD
n'est pas une ressource à retirer. AucunautreWT ni sousmodule de pair inclus.

## Copies fidèles deux postes, preuves conservées

Root privé ~/.cache/rag3weaver-build/cx-ed-vertex-cleanup-contract-v1.
Copie cp-a/reflink sans déréférencer les symlinks, inventaire source AVANT=
source APRÈS=copie(mode/UID/GID/mtime_ns/SHA ou readlink), puis TARposix.
Toutes les TAR ci-dessous existent et sont réempreintées deuxpostes égales :

| Archive | Octets | SHA256 |
| --- | ---: | --- |
| portable-archives.tar.zst | 2218369564 | e25dfdd3f2eda8de9ac01f58507c55f30871d6ed07591d43b918ac0382e859dc |
| pc-archives.tar.zst | 1117789441 | 8a08edbba998ae6006b5c6c035f297471ea80cf60f95bdebb0a84f8733707f1c |
| G-deps-b472-v1.tar.zst | 1270521826 | c47fcbeca06d1ef8705ebca151f7c80fa7ab6374844c875873e9e75e35a300e0 |
| target-fixtures.tar.zst | 9802168 | 130039eae1f0b96a515b151f87b87e5e9c8ed1ff2b884c5b007b66cf4f56ff3c |

Les deux premières contiennent les sources/admins et rapports privés,
lesinventaires et les21rapports de garde, sans token de portée.
ELF/logs/ledgers CPUrouge39/CPUvert59/canonique54 réempreintés intégralement
2posts sansécart (141a…524b/7a73…0a6/2754…ddde). Toutesarchivesoriginales,
P/Q/S, rougesintermédiaires, paquets22/34, PUSH10 et corpus restent gardés.
Aucune suppression d'archives ou de branches n'est proposée.

## Garde privée et quatre refus actuels

Source privée6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c
identique sur deuxpostes, jamais modifiée. GardeRO sur21WT :17VERT/4ROUGE,
cesquatre refus sont réels et ne sont pas blanchis par les copies :

- portable union-pilot-r1/docs/everything-declarative/16-11-2026/vertex-token-union/__pycache__ ;
- PC green-pilot-r1/docs/everything-declarative/16-11-2026/vertex-token-green/__pycache__ ;
- PC union-pilot-r1/docs/everything-declarative/16-11-2026/vertex-token-union/__pycache__ ;
- PC union-r1/extension/rag3weaver/target (journauxduWT, DISTINCT duTcache).

Les4sous-arbres sont déjà fidèlement archivés ; empreintes, UID/dev/inode
et listesexactes sont dansJSON. Proposition à C7 : après GOEXPLICITE, les
RENOMMER versquarantaineprivéeneuve horsW/admin/T, conserver cettequarantaine,
puis refairestatus et garde6f3 VRAIMENTVERTE. Aucune neutralisation de garde,
aucun rm d'ignorés préalable implicite. Si drift/UNKNOWNattribué/refus :STOP.

TroissousmodulesPC initialisés, sourcefix-r1/refreshr3/union-r1 : lesadmins
sont sousLEURadminprivé et ne serventqueLEURsourcecodeparsers, aucunpair.
Retraitnormal gitworktree remove proposé d'abord. Un refus structurel de Git
pour sousmodule initialisé est un risque connu, PAS un refus observé ici.
--force resteNONAUTORISÉ ici : C7 doit nommer explicitement ces3chemins et
la condition(GitnormalrefusUNIQUEMENTstructurel+archives2posts+sourceintacte+
quiescence+garde6f3fraîcheVERTE). ToutautrerefusSTOP, jamaisforcéparanalogie
avec l'ancienlotTIES. Aucun gitworktreeprune, branchdelete ou rmglob.

## Target unique, Git de fixtures et relais G

PC ~/.cache/rag3weaver-build/target-cx-ed-stable-tie-identity,
UID1000/dev55/inode166664078, propriétairemarker sourceunionb472. Portable :
TABSENT. Mesure du-sb8851604066 compte une fois les hardlinks ; somme des
fichiers parnom10140571541 compte leurs multiples noms. Ce ne sont pas deux
mesures interchangeables, et aucune promesse d'octets physiques libérés.

6symlinks listés, dont crateW et registreCargo : ne PAS suivre leurs cibles.
Pas de zoneprotégée, entrée spéciale ou propriétaireétranger au scan.
10petitsdépôtsGit de fixtures code-tree-tests (5HEADunborn), issus des juges
code_tree.rs. Tous leurscommon-dir ET worktrees sont dansT, aucunutilisateur
extérieur : preuve par worktree_state/inventory de LA6f3inchangée. Le corpus
completcode-tree-tests etsesGit/admins sont copiésfidèles avant/après puis
TAR2posts. Ne pas traiter cesdossierscomme un simplecacheaveugle.

PropositionT à C7 : après nouveauGO et rechecks typés/quiescence/fidélité/
10Git sanspair, renommer T vers retraiteprivéeneuve puis retirer SEULEMENT
cette racine capturée, sans suivre lesliens. Rien d'exécutable ou d'autorisant
n'est livré avec cecontrat ; pas de retrait fait. Toute nouvelleGitdépendance,
occupation, changementmarker/UID/inode ou archive invalide ⇒STOP.

G a demandé puis audité une copie HORS_T complète et figée,
R/cx-ed-vertex-deps-b472-v1 :14428fichiers5526823298octets,
manifestde797438448c79ae54a1bc313003b0fdead4e37b70b180298ce21690b320c945.
Copieav/aprèssource/destSHAégaux, inodesdifférents/nonliésT ; zéroCargo/
rustc/SDK/SQL. CompilerfichierOBSERVÉ1.98.1 SHA859254…1e74, PASattestprébuild.
GAUDITaa80f31a1e50550bcae6fd7126c0b3bd267f8360dbebedf43c3d6e63696ad394 :
sélection74artefacts/aliases résolus0manquant ; PASLINK/SQLqualifiés,
nativebuildouts exclus. G confirme aucunlecteurT/hold ; lacopieETsaTAR sont
CONSERVÉES horscleanup. Ses5.527Gologiques restent vivants enCOW : nepas
annoncer8.85Gophysiques libérés par retraitT. AucuneclosureclientPG/native
ou nouveauGO outillage déduit de cettepréservation.

## Quiescence, bornes et décision attendue

Scans exe/cwd/fd/argv aprèscopies :0attribution vivante surlesdeuxpostes ;
PC7088couplesPID/start des3anciensruns vérifiés0vivant/0inconnuowned.
UNKNOWNglobaux496portable/541PC conservés neutres, parcequescanTOUSUID ;
pas de preuve négative GPU/device/maps. Les scans sont observations de
préparation : relecturefraîche requise AVANTchaqueretrait et T.

Bornes PROPOSÉES600s parhôte après GO, sans build/test/probe ni exécution
SDK. Avantchaqueaction : HEAD/status/identitécanonique/archives2hosts,
quiescence, symlinks/Git/privateadmins et nouvellegarde6f3. Aucuneracine
partagée (main/.git/modules/registry/cachecommun/SDK/service/baseLIVE) n'est
supprimable. Les scripts cachecommun restentFREEZE, aucune recopie/pose
SCP/install n'a eu lieu ; seuls nos artefacts/archives privés sont copiés.

C7 doit arbitrer cecontratexact et les4quarantaines/3éventuelsforces AVANT
un GOcleanupdistinct. Sanscela : toutresteenplace. AucunGO900/6000/ancien
5400/ÀTOImaster, CIrerun, déploiement, OAuth/F300 ni retraittiers déduit.
