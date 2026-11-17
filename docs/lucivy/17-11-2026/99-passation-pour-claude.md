# Passation Lucivy pour Claude — STOP prioritaire du 17 novembre 2026

**Le 17 novembre prime toutes les reprises historiques. STOP demandé par Lucie
pour quota, relayé par C7. Ne pas continuer ce chantier depuis un rappel
automatique. Tous les coupons non consommés sont révoqués ; les anciens END
ne sont jamais réactivables.** Seuls cette sauvegarde/passation et ses contrôles
Git/métadonnées ont été effectués après le STOP. Aucune nouvelle exécution,
compilation, santé/provider, corpus, service, main/tag ou opération de nettoyage.

Session lucivy `01a1cb5e-7665-7a23-9980-ac3f98b0d97b` ; orchestration C7
`01a1cb5c-62b9-7ea0-8b7e-cf38398ece09`. Recherche
`01a1cb5c-d8c7-7182-9712-a78a9c43d24d`, Banc
`01a1cb5c-f4f3-7312-a80a-a1eabeef932f`.

## État utile immédiatement

Le dernier livrable est une **préparation SOURCE**, pas un correctif validé :
Lucivy **13298f7db3443956ea01e088b0c014fd8c60c2ff**, branche
`cx-lucivy-fs-output-fix-17nov`, parent
`bdf83ad76cf73a74cdde8358affb364b857e27e2`. Ni compilé, ni testé, ni exécuté.
Quatre fichiers,140 ajouts/49 retraits,16 noms de tests conservés. Aucun vert
FS global, corpus, replay ou livraison main/tag à en déduire.

Le contrat et les pins sont dans
[29-correctif-groupe-sorties-identite-source.md](29-correctif-groupe-sorties-identite-source.md)
et [source-pins.json](preuves/fs-output-fix-13298f7/source-pins.json).
RO Banc favorable PRÉPA publiée `c7e2e5d381`, consignée dans les docs à
**e70053a03cb278e9cf1db750525d734a4fe987be**. **La RO Recherche de cette
source est encore attendue au STOP** ; un avis tardif ne donne pas le droit
à cette session de reprendre les travaux. Aucun paquet GREEN n’a été préparé
ni aucun GO vert obtenu. Les seize PASS sont seulement attendus.

Le code prépare :

- `InvocationPaths` résout ensemble input/reçu/reports, impose sorties distinctes,
  hors entrée, parents existants et feuilles absentes avant toute réservation.
  Le callback reçoit snapshot et slots validés ; le vrai rapport emploie ce slot.
- Rapport `create_new`, sans mkdir, avant le builder réel ; pas de troncature.
  Le natif déclare `V3_CORRECTNESS_REPORT`, default targettmp JSONL direct.
- `document_ordinal(None,...)` devient `OrdinalError::Missing` ; conversion et
  borne explicites, aucune identité manquante transformée en doc0.
- Flags invalides refusés avant résolution/metadata/output et **sans reçu**,
  arbitrage C7 `30561f8229`. Le témoin historique flags/reçu a été explicitement
  aligné après acquisition du nouveau rouge causal. Refus des données
  Empty/IO/UTF8/panel conservent un reçu REFUSED sur sorties prévalidées.

Limites : namespace observé, pas de FD d’ancêtres ancrés ni preuve contre
renommage/montage hostile entre validation et usage. **Replay depuis manifeste
non implémenté**. Construction moteur réelle, parité corpus, corps du second
ELF et core/neufHOLD/A-B/ULP7.2/PR42 restent non qualifiés.

## Branches, worktrees et sauvegarde effective

Contrôles RO après STOP : trois worktrees propres, pas de diff/WIP non poussé.
Les égalités HEAD/branche distante ci-dessous ont été lues avec `ls-remote`.
La branche docs avancera uniquement du commit de cette passation ; son SHA
final est fourni dans le message de restitution et doit égaler origin.

| Dépôt / poste | Worktree | Branche / SHA distant vérifié |
|---|---|---|
| rag3db / portable | `/home/lucied/git_workspaces/cx-lucivy-plan-shutdown` | `cx-lucivy-reprise-16nov`, avant passation **e70053a03cb278e9cf1db750525d734a4fe987be** |
| lucivy / portable | `/home/lucied/git_workspaces/cx-lucivy-fs-admission` | `cx-lucivy-fs-output-fix-17nov` = **13298f7db3443956ea01e088b0c014fd8c60c2ff** |
| lucivy / luciepc | `/home/lucied/git_workspaces/cx-lucivy-fs-admission-17nov` | `cx-lucivy-fs-output-red-17nov` = **bdf83ad76cf73a74cdde8358affb364b857e27e2** |

Autre branche conservée et distante relue : `cx-lucivy-fs-gate-fix-17nov` =
**0668432241989dc9fac88e6df0e54e80c0f3e368** (préparation initiale, revue
réservée puis remplacée par le raccord RED et le groupé). Aucun rebase, force
push, suppression de branche ou switch de l’arbre de Lucie. Git identité
personnelle Lucie Defraiteur, luciedefraiteur@gmail.com. Le dépôt lucivy est
séparé de rag3db ; ne pas chercher les sources Rust dans le worktree docs.

**Attention : PC n’est pas sur le correctif13298.** Ses deux ELF actuels sont
ceux de sourcebdf, encore défectueuse. Ne pas présenter leur compilation ou
leurs observations comme validation de13298. Aucun commit local en attente.
Les scripts de préparation/copie dans `/tmp/lucivy-*.py` sont des aides de
session, pas des recettes autorisées à relancer ; sources/pins/bruts utiles
sont déjà publiés ou archivés. Ne rien nettoyer.

## Target, Q et ressources sur les deux postes

T Cargo **unique, sur luciepc** :
`/home/lucied/.cache/rag3weaver-build/cx-lucivy-target-16nov`, UID1000/dev55/
inode164536077. Marker `validation-provenance.json` SHA
`5a1671d22dc784bec78f60cf25c8b3003acc5f21e28290f05d0dfc22e0c394df`,
créateur0c795f99e5f49926a510971c521eb056a4880adc,
nonce3552903efab590b44f44cd8002f7b028. Aucun nouveau target/purge/import.
Le target homonyme **n’existe pas sur portable**, vérification metadata au STOP.

Lock effectif PC et seed identiques :
`dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6`.
Rehash RO après STOP conforme. Le worktree portable n’a pas de Cargo.lock
local ; ne pas inventer un lock portable ou restaurer l’ancien ce719.

Dernier paquet RED, **consommé et NON réutilisable** :

- PC `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-output-red-17nov-v1`,
  UID1000/dev55/inode169037256.
- Portable `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-output-red-export-17nov-v1`,
  UID1000/dev57/inode97253752.
- nonce075f5217e494870ab9515663beb234db ; préflight
  `54ebfdf06bd12071287cb211e787923ea708de2bcfc4f0fb498cc78296a97a14`.
- Pilot336deb03b25b8c7469bd98ff46a8693c8acebc9b8f56f6f28c793b78c08ff704,
  admitd1d52774f7a1d53d93ee6f9236cd6b6f07f5b95308c53d04881ff44d1cffe3be,
  export56e99ad94c46b927d4e935750257c797cc14415b3ba54c9f2f4dc821a011b40f.

Ne jamais exporter le token d’autorite.json. Toutes les copies publiques
utilisent seulement l’autorité publique PID/start/state/deadline/scriptSHA.
Tous les Q, scopes, traces et archives restent en place au STOP.

## Résultats réellement acquis, distincts des attentes

[24](24-six-rouges-rust-fs-observes.md), preuves
**bb5707c6a5c74825e894595d53ba80351d27d4ad** : source0f compilée, six FAIL
sémantiques initiaux (Empty, IO enumerate/metadata/read, UTF8, couverture15)
et positif. Sept juges **Rust natifs réellement exécutés** ; aucun corps
index-building/groundtruth/corpus. CPU final-lock2PASS et journal3PASS sont
des preuves réelles de raccord avec dépendances modélisées, pas killOS ou
absence distante universelle. Ne pas rejouer leurs GOs consommés.

[28](28-cinq-nouveaux-rouges-fs-observes.md), preuves
**5094f2b29eaafd22159ecebd01fbb9372af5ab7b** : GO
`C7-LUCIVY-FS-OUTPUT-RED-E75-17nov300`, publié `acfd30b407`, UNE commande27
exacte, session19943 rc0. Sourcebdf compilée release/offline/locked en3.22s,
inventaire16/ignored0, six nouveaux juges réels : **5FAIL sémantiques101 +
1PASS0**, chacun15filtered/0ignored, aucune infrastructure. Les dix anciens
ne sont pas sélectionnés. Le contrôleur rend0 parce que les défauts ont été
reproduits, **pas parce que la FS est réparée**.

Les causes observées : reçu ADMITTED réellement tronqué en rapport avec
égalité des octets ; rapport dans input ; reçu dans input via parent aliasé ;
NoneID→Ok(0) ; Flags précis mais EventManifest/reçu REFUSED créés. Le contrôle
sorties distinctes/Some0 passe. Aucun moteur/index/corpus réel construit ou
lu, second ELF seulement compilé/inventorié.

Nouveaux ELF bdf :

| Nom de cible | SHA256 |
|---|---|
| test_fs_fixture_admission-9128a97001eece0d | 542924c1e30da42ea95f4d208ee34fa300020910107d31549130552a191fa056 |
| test_sfx_v3_ground_truth-71572a69e410ce61 | a50adc2473feebc94e52a9335686e2c5241485a3801b39ab9515dc43c013c470 |

Ledger `b42ec21d4edfa1c2da29b4089764cfc6878852220ac8818af0c05b466b4af95a`
copié/fsync/rehash/ACK nonce/SHA avant toute invocation ELF (listes comprises).
Double contrôle lock effectif DBDB après juges puis juste avant finalaccepted.
Cinq anciens ELF sauvegardés en444 avant build et rehashés sur les COPIES.
Les deux chemins de cibles0f ont été remplacés par bdf dans T ; leurs anciens
contenus demeurent dans backups/archives, ne pas les chercher au même original.

## Processus, verrous et services

**Aucune exécution outil encore active dans ce fil au STOP, aucun verrou lourd
retenu ; aucun service démarré par cette phase.** Pas de scanproc postSTOP ni
de revendication de quiétude universelle. Les métadonnées ci-dessous sont
les observations de fin archivées, pas un nouveau contrôle des PIDs actuels.

Dernier Common186154/start128760537, scope `validation-n5ozz9b7`, TERMINE/
VERT0/erreur null/restes[]/nettoyé/own absent ; scan final0bloqué/11UNKNOWN
non attribués. Fin externe1287609.509280635 < CommonEND1287864.412135928 <
admissionEND1287877.043666828. Retour réel poste/SSH vu, restitution annoncée
à C7 et Recherche AVANT archivage. Session19943 terminée0, aucun enfant à
arrêter depuis notre contrôleur. Ne pas tuer un PID depuis ce vieux numéro.

Portable parenttimeout3083866/start941309.28, worker3083881 ; fin observée
941314.237170273, soit4.957170273s depuis parent ; SSH3083906 réapé0,
cleanup_error null. ENDtravail941594.28/TERM941604.28/KILL941609.28 étaient
les bornes de cette seule invocation, jamais une allocation future. Aucun
killOS/expiration réellement mesuré par cette passe. Les pending bruts sont
complétés par les vrais reçus ultérieurs, sans réécrire l’histoire.

## Preuves conservées avant correctif et réserves de lecture

[57 SHA PC](preuves/fs-output-red-bdf83ad-observed/SHA256SUMS),
[6 SHA portables](preuves/fs-output-red-bdf83ad-observed/PORTABLE_SHA256SUMS).
Archive privée **sur les deux postes**, même chemin :
`/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-output-red-bdf83ad-17nov-v1`.
Dix-huit copies en444/17 108 104octets : neuf outils/seed/pins, sept fichiers
README/Rust sélectionnés, deux ELF. Ce n’est ni le target ni une copie intégrale
du dépôt ; les commits complets sont sur origin. Bruts/source/pins/temps,
reçus portables et reçu d’archive aussi conservés deuxhôtes.

RO preuves Banc **d6c49ffecf**, public et privé rehachés PC+portable ; Recherche
favorable Git57+6 et18privés LOCAUX, pas de rehashPC privé intégral revendiqué.
Causes/bytes/ledgerACK/locks/retours recoupés. Archive metadata1287720.809376657
après retourcommande MAIS avant deadline nominale : outside_validation_window
signifie hors invocation, pas deadline expirée. Premier export public a refusé
FileExistsError sur un nom déjà utilisé par la préparation ; aucune écriture
par-dessus. Reprise des COPIES sous `-observed` seulement, jamais des tests.
Ce premier refus est le constat du porteur, pas un événement directement
observé par les relecteurs. Intégrité finale des copies confirmée par eux.

## Tickets à garder ouverts

- [Admission corpus FS / vue consommée](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md) :
  porteur Lucivy, source13298 nonvalidée, replay manquant. Pas de clôture.
- [Profil core et diagnostics non isolés](../tickets/2026-11-16-le-profil-core-par-defaut-inclut-des-diagnostics-non-isoles.md) :
  aucune qualification core/default-profile déduite des fixtures ; ABSENT_OK
  ne doit pas masquer le refus de l’entrée correctness.
- [indexed:false](../tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md) :
  parent lot/suivi ; ne pas le clore sur ces seuls tests FS.

Linux7.2 absent et ULP en HOLD ; copie publique rag3db/vue5000 non qualifiée,
A fixé300 Linux2.6.0 nonadmissible pour son panel, ne pas resélectionner pour
obtenir des hits ;9HOLD/PR42 demeurent leurs refus. Aucun contact corpus par
les phases reprises ici, ne pas appeler leur historique un corpus livré.

## Trois prochains pas, uniquement après reprise explicitement autorisée

Ces commandes sont **INACTIVES**, fournies pour une future reprise par Claude.
Elles ne donnent aucun droit de réactiver ce fil, un coupon ou une deadline.

1. Reprendre le contrat depuis ce document, rapport29 et ticket FS ; recueillir
   l’avis Recherche13298 encore manquant, garder les réserves explicites.
   Relecture Git possible après reprise, sans exécution Rust :

   ```bash
   git -C /home/lucied/git_workspaces/cx-lucivy-fs-admission show --stat 13298f7db3443956ea01e088b0c014fd8c60c2ff
   git -C /home/lucied/git_workspaces/cx-lucivy-plan-shutdown show e70053a03cb278e9cf1db750525d734a4fe987be:docs/lucivy/17-11-2026/29-correctif-groupe-sorties-identite-source.md
   ```

2. Après deux RO SOURCE et autorisation de reprise, préparer un **nouveau** paquet
   GREEN concret : source13298 (ou successeur relu),16 exacts attendus1PASS/
   0FAIL/0ignored/15filtered chacun, deuxELFCargoJSON, ledger/copiefsync/rehash/
   ACK avant TOUT ELF, sauvegarde bdf+historiques, lockDBDB effectif deuxguards,
   mêmeT/32G/HIGHmax/swap0/quota4/jobs4/affinitémoins2/Common/budgetphysique et
   retours réels. Paquet Q/nonce/pins nouveaux et ROcouple complète obligatoires.
   Ne pas transformer l’ancien pilote5FAIL en un run vert improvisé ; aucun
   paquet GREEN ou argv exécutable courant n’existe à la passation.

3. Après ROcouple, obtenir un **nouveau GO exact** C7/Lucie avec bornes physiques
   neuves puis seulement exécuter UNE entrée. Le préfixe suivant est un gabarit
   INACTIF à compléter depuis le futur paquet relu, pas une commande actuelle :

   ```text
   PYTHONDONTWRITEBYTECODE=1 timeout --signal=TERM --kill-after=5s <borne-neuve> python3 <nouvel-exporteur-relu> <argv-neufs-épinglés>
   ```

   Premier imprévu STOP sans retry ; archiver réels bruts/ELF/source/temps/
   Common/retours et rehash deuxhôtes avant une suite. Seize verts fixtures
   ne qualifieront pas à eux seuls moteur/corpus/replay/globalFS : maintenir
   ces suites dans leur ticket, publier source/recette/pins/RO et GO séparés.

**STOP après publication de cette passation. Aucune reprise automatique.**
