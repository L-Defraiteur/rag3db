# Capture physique des exécutables : contrat proposé, aucune capacité livrée

Périmètre C7df5e5ee7e1, même ticket anglais OPEN, complété avant cette note
par e9719bba7d. **Documentation et lectures seulement** : aucune implantation,
API exécutable, fixture lancée, compilation ou capture de processus réel.
Les14témoins verts/ROpreuve4a6cc restent le résultat du validateur hermétique,
pas du producteur physique. ELF_PROVENANCE reste vide ; Qv4 demeure désarmé.
[Références exactes et manifeste inactif](physical-execution-contract.inactive.json).

## Ce que les primitives portent déjà

| Primitive relue | Garantie utile | Limite pour ce raccord |
|---|---|---|
| Common6439fdd8, `Descendants.relever/signaler/arreter` | PID/start, ascendance, cgroup, plafonds, arrêt pidfd/reap et END | Le sondage25ms ne capture pas chaque exec et n'en empreinte pas le contenu |
| Snapshot strict7e2f5bc7 | Types, UID, inode/dev, mode, mtime, SHA des fichiers ; avant/après | Deux états de chemin ne prouvent pas les bytes exécutées entre eux |
| Noyau publication C9c9b84168 | Extraction constante ;36régressions CPU rapportées par CP/Archi | Aucune API physique ELF générique livrée |
| Préparation c2d13ffd, `validate_plan`, `Relay.verify_publication/publish`, `GateBase.stage/ack` | Plan immuable, fichiers exacts, ELFmapN, référence ACK antérieur, copie rehash/fsync/ACK lié | Intermédiaire NONJOUÉ/NONADMISSIBLE ; copie acquittée ≠ copie exécutée |
| Provider Rust c2d13ffd, `RustPublicationProvider.read` | Source/copie/ledger et inventaires confrontés | Pas de PID exécuté associé à chacune des copies |
| Préparation Archi229b, `runner.compile/inventory`, `qualify.check_artifacts`, `observe.identity/runtime_snapshot/complete` | CargoJSON unique, copie/inventaire, observations PID/start/exeSHA/cgroup | Sondage non exhaustif, canon pouvant jouer T plutôt que P ; aucune preuve générale contre ABA |

CP et Archi confirment indépendamment ces limites. Les interfaces proposées
2c8d1df8/00e20846 ne sont pas du runtime. Window/GlobalLifecycle/bridge ne sont
pas livrés par ces références ; aucune capacité quatre publications ou inter-
hôtes n'est empruntée au seul motif que les noms existent dans les sources.
Les copies Q229b et les worktrees de leurs porteurs ne sont pas modifiés.

## Frontière sémantique manquante

Trois choses distinctes :

- **contenu** : SHA des bytes ELF, fermé sur un catalogue et un profil Cargo ;
- **identité du processus** : PID/start/UID/cgroup/exe réellement exécuté ;
- **occurrence** : une invocation précise, son rôle, sa phase, ses argv/filtres,
  ses logs, son code de sortie et ses limites.

Un même contenu peut servir les filets et plusieurs suites, donc plusieurs
occurrences. Un même chemin T peut porter des contenus successifs. Une copie
P peut être correcte sans avoir été jouée. Ni égalité de chemin, ni hash final,
ni observation ponctuelle d'un PID n'unifie ces trois identités.

La capacité commune recherchée doit relier **l'image réellement passée à
l'exécution**, une copie préservée de cette image et son occurrence, avant de
laisser qualifier les résultats. Un sondage plus fréquent ne fournit pas cette
garantie pour un processus bref ; un fichier de recette n'y supplée pas.
Le choix mécaniste appartient au noyau commun : frontière d'exécution fiable,
identité du fichier/image réellement utilisée, copie stable et observation du
PID avant réutilisation. Aucun hook Linux, traceur, wrapper Cargo ou observateur
TTY n'est ajouté ici. Une simple ouverture du chemin avant exec suivie d'un
exec par chemin garderait une fenêtre de remplacement : elle ne suffirait pas.

## Surface conditionnelle producteur → publication → consommateur

Ces obligations sont proposées, **pas des nouvelles fonctions livrées**.
Le portage commun serait à cadrer par C7 avec CP/Archi ; TTY fournit son profil
et consomme les preuves, sans seconde boucle ni client d'observation propre.

1. **Contrat attendu avant observation.** Source APP/pilote/Q, fermeture des
   manifestes/locks/features/defaults/SDK, rôles d'exécutables et invocations,
   plan, nonce/allocation, domaines d'horloge et END sont épinglés. Le SHA du
   contrat attendu reste distinct du SHA des faits observés. Pas de promotion
   d'un PID ou SHA découvert en valeur attendue de sa propre validation.
2. **Catalogue du contenu.** CargoJSON lie le target/src_path/profil/features à
   l'ELF produit ; copie et bytes sont préservées. `--list`/ignored porte sur ce
   contenu exact, sous autorisation native future explicite. Catalogue et liste
   ignorée sont des payloads hashés, pas une regex des résultats a posteriori.
3. **Publication acquittée.** Composer le noyau unique : identité plan/phase/
   séquence/prepared, fichiers déclarés, copie/fsync/rehash, ACK puis référence
   à la publication antérieure. Aucun slot rempli par un faux payload, aucun
   nom de ledger choisi à partir d'un chemin runtime libre.
4. **Occurrence physique.** Le producteur commun relève son propre rôle et le
   processus testé : PID/start/UID/cgroup/exeSHA/argv déclaré, parent effectif,
   référence contenu/catalogue/ACK, copie réellement jouée et temps pré/exec/
   post dans un domaine déclaré. Il doit garantir le lien à l'image exécutée,
   pas déduire « exécutée » du nom du fichier ou d'une observation de fichier.
5. **Fin et preuves.** Code/signal, inventaire et blocs de résultats exacts,
   compteurs/ignored, RAW et références SHA, première cause immuable, arrêt
   attribué/reap/quiet, publication terminale fsync tiennent sous END original.
   Absence d'observation complète, processus disparu trop tôt ou ambiguïté :
   HOLD/refus, jamais une occurrence positive reconstruite depuis les logs.
6. **Consommation.** Vérifier références catalogue→contenu→occurrence→RAW,
   identités avant/après et intégrité des copies. Pas de reçu libre pris pour
   une preuve du noyau. Le producteur doit attester la capture, et le noyau
   attester sa publication ; ces deux garanties ne sont pas interchangeables.

La première surface à faire approuver contient donc : identifiants contenu,
publication/catalogue, occurrence et phase ; contrat source/profil ; identité
producteur et processus ; lien à l'image/copie ; pré/exec/post ; RAW/rc ;
première cause et état terminal. Les champs runtime restent NULL dans le
manifeste joint. Aucun format de reçu consommable n'est fabriqué maintenant.

## Canon inchangé et reconstructions

Le blob canon2150 demeure la référence. Son `touch src/lib.rs`, ses contrôles
F8 et Cargo peuvent reconstruire une lib après le catalogue initial. Il faut
traiter chaque occurrence, sans supposer que le nom du binaire reste son contenu.

- Reconstruction de contenu différent B : A ne permet plus de qualifier B.
  Un nouveau catalogue/publication/ACK B, dans le plan autorisé et AVANT l'action
  dépendante, est nécessaire ; sinon refus. Pas de relisting clandestin ni TTL neuf.
- Reconstruction A→A : l'inode peut changer sans changement de contenu. Le
  contrat source/profil et les identités des occurrences restent à vérifier ;
  pas de confusion entre équivalence des bytes et ancien processus réutilisable.
- Chemin restauré B→A après l'exécution de B : la capture d'occurrence doit
  conserver B, même si le hash final du chemin redevient A.
- Copie P=A, exécution T=B : un ACK valide de P ne qualifie pas l'exécution.

Le post est celui de l'occurrence, AVANT une reconstruction suivante autorisée.
Relire uniquement le chemin original à la fin du canon ne suffit pas ; à
l'inverse, exiger son hash final identique à une ancienne génération peut
refuser à tort une reconstruction autorisée ultérieure. Les copies immuables
et les publications de génération doivent préserver les faits des occurrences.
Le validateur hermétique14 actuel n'est pas encore l'adaptateur de ce modèle
multi-occurrences : évolution de sa surface et nouveaux témoins seront requis.

La projection source des racines de tests du canon proposé est : trois appels
e2e (tty/lectures/code), chacun filets puis suite ; lib large ; navigateur ;
libtty séparée. Ce sont neuf positions de racine à DÉCLARER, pas neuf processus
physiquement observés ni la totalité des descendants. Catalogues, auxiliaires
et phases harnais sont des rôles séparés à inventorier avant admission ; leurs
cardinalités ne se déduisent pas des neuf positions. Aucun nombre de payloads
5+1+1+1 du lot Archi n'est copié pour remplir artificiellement le profil TTY.

Si aucune primitive commune ne peut couvrir ces occurrences tout en conservant
la recette canonique, la condition reste HOLD. Toute extension du canon ou de
ses entrées transitives doit être exposée à C7 et à son porteur, témoins rouges
avant code ; pas de réécriture locale « équivalente » ni de canon simulé.

## END et frontières LOCAL/PC

Capture et tests sont proposés LOCAL, sous leur Common réel. Les sources
publication sont actuellement préparées pour leurs propres autorités : aucun
passage PC→LOCAL ne se fait par substitution d'un nom de variable ou d'hôte.
Le futur raccord doit être relu et qualifié dans le domaine exact de sa portée.

La borne3600 précédemment proposée n'est pas un GO. File poste, montage,
catalogues/capture, canon, TTY, transferts, publications et arrêt/reap/fsync
restent dans son unique budget ; chaque phase reçoit MIN(reste frais,cap),
aucun budget neuf après ACK/reconstruction. Échéances/observations portent
CLOCK_MONOTONIC@hôte ; aucune comparaison numérique directe PC↔LOCAL.
Les échanges doivent relayer un budget borné selon le contrat commun approuvé,
sans prétendre qu'une horloge observée détermine celle de l'autre poste.
Les leases modèles PC et le transport/PG LOCAL restent des contrats séparés,
encore HOLD. Le noyau de copie n'accorde aucun service, SQL, warmup ou démarrage.

## Témoins à prépublier avant implantation

Le tableau est un plan d'assertions sur la future API commune réelle, pas des
tests exécutables ni exécutés. Fixtures de bytes/horloges/processus seulement ;
pas de faux noyau, de classe d'exécution de remplacement ou d'ACK inventé.
API absente : manque nommé/HOLD ; un ERROR d'import ne vaut pas son rouge.

| ID | Témoin proposé | Attendu |
|---|---|---|
| P01 | Catalogue A, ACK/copie A, image réellement exécutée A, occurrence complète | Acceptation bornée |
| P02 | Même chemin/noms, image B ; catalogue/copie attendus A | Refus avant qualification |
| P03 | B exécuté puis chemin restauré A | Refus, capture B conservée |
| P04 | Rebuild B après inventaire A, sans nouvelle publication ACK | Refus avant test B |
| P05 | Rebuild B déclaré, catalogue B et ACK avant occurrence B sous même END | Acceptation conditionnelle, sans reset |
| P06 | Rebuild A→A avec inode nouveau mais contrat et occurrence corrects | Contenu équivalent, identité occurrence neuve |
| P07 | Copie publiée A, processus exécutant T=B | Refus même si copie/ACK intacts |
| P08 | PID réutilisé/start ou exe différent entre les relevés | Refus, aucun signal au PID non attribué |
| P09 | Processus bref absent du sondage, logs positifs et chemin final A | HOLD, aucune preuve déduite |
| P10 | Copie/RAW/catalogue modifié après ACK ou après exécution | Refus et bytes antérieures préservées |
| P11 | Même ELF mais argv/phase/nonce/allocation/inventaire d'une autre occurrence | Refus de mélange |
| P12 | Publication héritée sans ACK, doublon ou séquence hors plan | Refus par noyau réel |
| P13 | Fin capturée trop tard, réserve de reap consommée ou horloge d'autre hôte | Refus sans nouvelle échéance |
| P14 | Première cause entre capture et ACK/release, puis chemin restauré | Rouge immuable, arrêt attribué et RAW/reap sous END |
| P15 | Ordre/noms/filets/ignored ou logs non liés au catalogue de l'occurrence | Refus, pas ANYrésumé |
| P16 | Précontrôles connus puis F8extra/duplicate et fin8/8 | Refus du jeu supplémentaire |

Les positifs conditionnels ne peuvent être verts tant que l'API physique
n'existe pas et que son raccord canon n'est pas couvert. Les14témoins déjà
verts restent inchangés et distincts de ces futurs témoins du producteur.

## Décision de suite demandée

RO documentaire de cette frontière et des témoins ; avis CP/Archi sur capacité
commune et compatibilité canon, puis attribution C7 des capacités manquantes.
Aucun code producteur/consommateur, test CPU, Cargo/no-run/list/native, health,
PG, acteur, slot, mutation target ou activation Q n'est autorisé par cette note.
