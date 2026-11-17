# Passation Embarquement vers Claude — arrêt du 17 novembre 2026

## Priorité absolue : STOP Lucie pour quota

Ce document du **17 novembre prime les consignes et GO historiques**.
C7 transmet l'arrêt prioritaire de Lucie : travail arrêté ; tous coupons non
consommés révoqués, aucun END réactivable. Aucun calcul, test même CPU, health,
provider, slot, tunnel, RUN, master, tag ou nettoyage à partir de cet arrêt.
Seuls sauvegarde/commit-push WIP sans force, passation et métadonnées RO
nécessaires sont autorisés. **Ne pas reprendre sur rappel automatique.**
Un futur travail effectif demande une reprise explicite et un nouveau GO exact.

Cette passation publie seulement des documents, dans son propre worktree :
`/home/lucied/git_workspaces/cx-embar-passation-20261117`, branche
`cx-embar-passation-20261117-r1`, base ec5e675cc2870e5987baee226e17ff43e4175b7d.
Le SHA du commit portant ce rapport est communiqué à C7 après push ; il ne
peut pas être inclus dans ses propres octets. Vérifier ce SHA et la branche
origin via les commandes de lecture ci-dessous. Aucun WIP non poussé connu.

## Livré et composants préparés : frontières de qualification

**Atterri avant STOP** : rate-key uniquement, master
`bef0b27cb33b9143dabee019fccdf7453da553a3`. Cela ne clôt pas le ticket de
mesure débit/table/par passe. La validation antérieure avait lib2604 réussis,
72 ignorés NON JUGÉS, NAV45 et trois e2e. Aucun de ces tests n'est rejoué à
la passation. Les propres worktrees cx-rate-key et target-activation avaient
été retirés après archives et garde courante6f3 fraîche ; branches distantes
conservées. Ne pas généraliser ce retrait à d'autres chemins rate-key.

**Préparations publiées, pas nouveau déploiement produit** :

| Composant | Source complète et preuve antérieure |
|---|---|
| Helper grant exclusif | 3aa76a7a6009931bae5f41e5cc30961f6f3ef230 ; ancien protocole borné, réutilisé sans changement par l'owner |
| Observation R→S | c5306b42663d2c001578dcc64d621cac826bfad5 ; identité stable distincte de deux états vivants |
| Transport commun et owner global | 443b5a039cc2a3ff11705b0ebaa5bfa6577d6b0e ; 17CPU transport,39CPU global+transport antérieurs |
| Owner cycle raccordé au transport commun | bcfd1934ac44153b26ea5832f2bd7415ea83ded2 ;57CPU owner+helper+contrat+transport antérieurs |
| Acteur commun, samplers watcher/contrôleur/driver | 4f54eef8af693c17e5401d1adc9d4c9062b315d3 ;20CPU hermétiques0.008s antérieurs |
| Contrat providers TTY INACTIF | ec5e675cc2870e5987baee226e17ff43e4175b7d ; document seul, aucune admission runtime |

SHA fichiers à conserver :

- `provider_health_transport.py` :
  edf45f73bf97d21f2a0caab4d6e07a7d01f807fba8310c9006f442159e2bc2a6.
- `global_services_owner.py` :
  9d308b5bd2d1eb44dca02da8d02f8cce4a2a7717e57a246301b468c142bf6b8d.
- `lifecycle_provider_observation.py` cycle :
  83139ebb329a59f19aa881f7f9678a7e6bf87c39abd3ff0ab7d20ecc15c0ba00.
- `grant_watch_actor.py` :
  7d7db5a0c7d9ac762eed74ade2d0b9ee256b8499d0029ae970feac916de76612 ; test
  9a7bbe30d040ece0c3f016fd429beca1a1e6e28b17f6d5c68ef4204588121899.

Transport = GET /sante PC7878/7879 uniquement, worker whole5/kill-reap bornés,
statut200 et neuf identités strictement typées, usage entiers0, preuves brutes
bornées avant refus même timeout/nonzero/partiels. Aucune déduction zéro RPC
à partir d'usage0. Modèles attendus externes figés, jamais actual-as-expected.
Owner cycle/noembedding et owner global/embeddings sont des politiques distinctes.

Acteur = ARM dans première boucle après contrôles, preuve OS indépendante +
poll parent requis, heartbeat10s, common END original, arrêt immédiat si
firstred/disparition, ACK grant distinct de résultat terminal. Après grant :
attente RELEASE du contrôleur sans GET supplémentaire. Samplers acceptent
R/S si identité stable et états vivants ; script rôle attendu avant hash ;
.env/.vault et cibles liées refusés avant lecture de contenu. Aucun Env entier
ni jeton d'autorité dans les preuves. Tests acteur utilisaient des faux proc,
clock, politique et autorité ; aucun véritable acteur lancé dans ces20CPU.

Rouges de développement préservés dans l'histoire :5a0cbc5997 admission non
implémentée ;1dddf58545 sampler public absent ;ae75f55139 absence du refus
.env. La première fixture c1b622cab3 avait un faux exe incomplet, donc rouge
FileNotFoundError non probant, corrigé avant le rouge pertinent ae75. Source
et tests ont été poussés avant leur exécution. Comptes verts = exécutés à ces
étapes, pas rejoués après STOP. Aucun skip transformé en vert.

## Cycles réels d'Arbre : résultats rapportés et limites

Les ressources et contrôleurs de ces cycles appartiennent à Arbre, pas à cette
session. Consulter sa passation du17 pour les derniers arbitrages/RO. Embar
n'a pas rejoué leurs juges ni relu toutes les preuves Q9.

- V4/3c2 : archives immuables
  e65cbd34a582c3bbb82773bfd9fd6f9b8abff695, rapport
  `docs/arbre-principal/17-11-2026/01-cycle-3c2-rouge-avant-armement-producteur.md`.
  Watcher armé1247046.197 après dernière entrée owner1247017.830 : refus
  avant helper/GET/grant. Notre annonce « veille active » était incorrecte
  car poll1 observé ; erreur reconnue, pas attribuée à une latence réseau.
  Pas de SHA préexec ni startOS/UID/CG/argv préexec capturés : NON CAPTURÉS,
  ne jamais les reconstruire. Scope ROUGE/TERMINE/restes[]/nettoyé rapporté.
- Q8 : grant autonome consommé, puis premierrouge1266055.091886152
  `read-only copy changed existing bytes or modes`, rc1/acteurREFUSED,
  scopeROUGE1/restes[]/nettoyé/TERMINE rapportés. Ancien grant reste consommé.
- Q9 correctif : source e9f037be2aa25da2c7f6c76f79e2eb57b6c676c9,
  prepared96ebc22e7fae474f812273e0bf2f8d41b3f24b5205c6e4f21d4594fbbd0f3099,
  preuve6e97b2ff8a6a8f4e4ca095ce3033fce10ceb8549,
  `docs/arbre-principal/17-11-2026/09-cycle-q9-observe-preuves.md`.
  Arbre rapporte rc0/OBSERVED/acteurRELEASED/CommonVERT0/TERMINE/restes[]/nettoyé,
  shutdown mêmePID et santé avant/après usage0 ;444 fichiers manifestés sur
  deux hôtes, deuxRO demandées dans son message. Copie seulement,
  pas qualification LIVE ni durabilité. Pas de nouvelle admission par ce succès.
  Le groupe préparatoire antérieur2dccc avait **124** fonctions CPU vertes,
  pas84 ;84 était un compte ancien, et84 fichiers e65 est une autre nature.

Anciennes preuves ne servent ni à rafraîchir des baux ni à réactiver un GO.
Aucune réservation maintenue par Embar, aucun GET manuel ajouté à Q8/Q9.
Les acteurs autonomes ont porté les relevés frais dans leurs GO propres.

## En cours désormais gelé : B, TTY, global et dette

**B copie privée sans embedding** : parent
`docs/arbre-principal/tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md`,
cadrage206cb4f93707d141489df8c829ad90927dd05537. Arbre préparait source99 /
backendprivé687de930...58f78 / SDK59 sur NOUVELLE copie FIN, sans calcul.
Ces pins partiels ne constituent pas un couple final. Dernière demande =
référence d'empreinte provider ATTENDUE, fournie depuis Git sans contact.
GO B/coordination distincts étaient encore requis ; maintenant tous non
consommés révoqués. Aucun cloneDATA/health/service lancé par cette préparation.

Profil publié `docs/arbre-principal/16-11-2026/debt-copy-a/lifecycle-provider-expectations.json`,
source initiale84a61e69341fd5cecfd750bc31298dee8ad5e91f, inchangéQ9 ;
blob24e7706091168d10f34309293e3a438776f78323, SHA256
0ce4548bcc0ccbc44357f6c9f71cbd8301afce7dd9740bdeb5680178947a5e8c.
Kind EXPECTED_PROVIDER_ONLY_NOT_FRESH_HEALTH. Provenance historique
`~/.cache/embarquement-notes/vertex-union-968/fresh-window-2.json`, SHA
f1bfcab2664b82692c5b608552824604138168b529584e3a895c425f0b06c4f1.
Attendus historiques PC7878 PID3906704/UID1000/start111727101,
exe `/home/lucied/.local/bin/rag3weaver-embeddings`, SHA
 a70f1e462bd9df9516b30b052e90f60d064bafacadcd6adb39e3b1f44be31028 ;
health executable suffixé@1794771361 ; granite-278m/768/Flex32 réel dense,
max_chars12000. **Pas des faits frais ni un PID à signaler/arrêter.**
Mismatch futur = refus, aucune promotion d'observation en nouvel attendu.

**TTY INACTIF** :
`docs/orchestration/17-11-2026/coordination/tty-provider-contract-preparation.md`,
sourceec5e675cc2870e5987baee226e17ff43e4175b7d, SHA
3f3342ae49695484b68cdc79619580f9c863e152a43e6c31a88915f27cec5fc3.
TTY rapporte sourcec003eaa86f5fef5f3870916df91461e5ce67128a,
Qlocalcx-tty-union99-controller-v2/prepared
6ea06438b4a5ba83578d7c4f52875787313c33626b1ef81546829133e4bcd8c9,
execution_ready=false/refus AVANTslot. Horloges PC/LOCAL, autorités,
politiqueTTY/payloads/release HOLD ; ne pas remplacer acteurPC par ACKfichierLOCAL.
LOCAL7979/7980→PC7878/7879 mapping nominal, pas disponibilité fraîche.
G référence85c96a59be, `docs/optimiseur/17-11-2026/01-tty-local-resource-contract.md`
et tty-local-resources.inactive.json : PG LOCAL5433 distinct de PC,
outil PG à qualifier, pas PG_BASE_FIXE inventée. Canon proposé
MIN2700/END−650 sous3600 n'est pas GO ; lease doit couvrir phase entière,
pas ACK90s/leasecycle230s. Le noembeddingcycle ne s'applique pas au global.
Les versions TTY plus récentes se reprennent dans sa passation17, pas depuis
cette référence préparatoire plus ancienne.

**Global Recherche/Archi** : composants owner443b/commonedf45 préparés et RO
favorables bornées antérieures. Pilotes complets, physique PG et clock/topologie
restent des qualifications séparées. PGv1 pur8ddb87233a22eda7122c04f2b4ba8b1656577550
n'est pas un transport PG réel qualifié. G avait un nouveau pilote Rust/paquet
préparé ; reprendre sa passation17. Aucun hostpsql/dockexec de secours,
aucun faux OID ni troisième acteur pour contourner HOLD. Plus de GO existant.

**Dette469** : replay publié1760fa03a5b0fd3d05ef98b104aa4a9bf4420221 par Arbre,
51parents/469nouveaux-only attribués au recut pour cette cohorte, pas preuve de
paiement LIVE ni de74réutilisations. Q9 copie FIN déjàdette0 ne paie rien sur
original. Garder computed_count/reused_count UNKNOWN historiques.

## Tickets à reprendre sans les déclarer clos

- `docs/orchestration/tickets/2026-11-17-le-cycle-demarre-avant-l-armement-effectif-du-producteur-de-grant.md`
  : mécanisme acteur porté, intégration/cycle/RO chez Arbre, STOP prioritaire.
- `docs/orchestration/tickets/2026-11-16-un-grant-est-publie-apres-la-pause-ready-faute-de-borne-explicite.md`
  : gardes d'admission/lease distinctes, vieux B5/late refusés conservés.
- Parent B troisbases ci-dessus et ticket dette469
  `docs/arbre-principal/tickets/2026-11-16-la-migration-de-scopes-laisse-469-morceaux-sans-vecteur-transfere.md`.
- `docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`
  : contrat inactif seulement côtéEmbar, produit porté par TTY.
- `docs/embarquement/tickets/2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md`
  : parent encore ouvert après key-only ; mesures parservice/parpasse manquantes.
- Autres réserves héritées, non résolues par ces owners : anciens morceaux
  géants à réécrire ; parité sans tolérance mesurée ; mêmes textes/bits selon
  lot ; réglage déterministe CubeCL ; plafond exact jetons ; activation/résidence
  modèles ; refus service sans Vulkan ; preuve saturation carte réelle ;
  visibilité des causes d'arrêt service. Fichiers sous
  `docs/embarquement/tickets/`, noms explicites ; consulter état actuel et
  passation13 en historique, ne pas inférer une clôture du présent rapport.

## Git et worktrees : constat métadonnées lors du STOP

Six propres W locaux propres, zéro diff/untracked, têtes vérifiées via
`git ls-remote --heads origin 'cx-embar*'` égales aux branches distantes :

| Worktree local | Branche | HEAD complet = origin lors du constat |
|---|---|---|
| ~/.cache/rag3weaver-build/cx-wt-embarquement-passation-week-end | cx-embarquement-passation-week-end-20261113 | f4d791a0956b571c5f5a4d8cc23f71db6e2b0b51 |
| ~/git_workspaces/cx-embar-lifecycle-grant | cx-embar-lifecycle-grant-20261116-r1 | 3aa76a7a6009931bae5f41e5cc30961f6f3ef230 |
| ~/git_workspaces/cx-embar-lifecycle-observation | cx-embar-lifecycle-observation-20261116-r1 | c5306b42663d2c001578dcc64d621cac826bfad5 |
| ~/git_workspaces/cx-embar-global-services-owner | cx-embar-global-services-owner-20261116-r1 | c11d511344350efbd11d606b3ae937020d725996 |
| ~/git_workspaces/cx-embar-cycle-shared-health | cx-embar-cycle-shared-health-20261116-r1 | bcfd1934ac44153b26ea5832f2bd7415ea83ded2 |
| ~/git_workspaces/cx-embar-grant-watch-actor | cx-embar-grant-watch-actor-20261117-r1 | ec5e675cc2870e5987baee226e17ff43e4175b7d |

Septième W = cette passation ; documentation seule, commit/push puis comparaison
origin effectués avant signalementC7. Aucun forcepush ni suppressionbranche.

Sur PC, inventaire worktree filtré embar/rate-key ne trouve pas les six W
locaux ci-dessus ; trouve `/home/lucied/git_workspaces/rag3db-rate-key`,
branche rate-key, HEAD0aecc6b70b116c08ff8f266bf86370090b32fc77 propre.
**Ce chemin n'est pas revendiqué comme notre ancien cx-rate-key** : aucune
mutation/retrait ni arbitrage de propriété fait. Autres W étrangers non audités.
L'arbre principal de Lucie reste HEAD927941a67a5805450d49f1e04b39fdd542a6059b,
avec sousmodule codeparsers modifié et docs/lucie-tmp non suivis : étrangers,
pas sauvegardés/commités/modifiés par nous.

## Deux hôtes, archives et targets : conservation sans nettoyage

Répertoires `~/.cache/embarquement-notes/` observés présents LOCAL et PC par
métadonnées seulement au STOP :

| Répertoire | Manifeste historiquement vérifié sur deux postes avant STOP |
|---|---|
| global-services-owner | 81b18acca528ed4102be575ae358896a1ce2404e32b2f2848a29857208be13a9,36fichiers |
| health-transport | 071faa4c1b09d45533565d9420b032c6380c3dd9550b1129fc9953bd6fbd2efc,40fichiers |
| cycle-shared-owner | ffa9f266b75e5d7118ff45715d392dc6d39614c13e7efd9900b96376a48605d8,12fichiers |
| lifecycle-3c2-watch-v4 | 06e5980bd9273aa35b50e73f26acabfe35819a7825f56cefbeb239b9f968fcd5,11fichiers+manifest |
| lifecycle-3c2-watch-v4-accounting | accounting.json bc47bfe04eb09147013b22032bf311103ff8265096f66c90df793cec1bc5cfeb |
| rate-key-16nov | archives rate-key/ELF/admin/rouges conservées, voir rapport antérieur |

Ne pas présenter ces SHA historiques comme un nouveau rehash à la passation.
Archives Arbre e65 :84fichiers/85avecmanifest,16 738 573octets, manifest
8087ab5db59b8f48bdfb98579f0743acecb1d7831197781a981c0fde814e55de.
Les nouveaux20CPU acteur sont décrits dans son ticket et l'historique source ;
aucune prétention de copie PC des nouveaux fichiers actor via cette table.
Le Git poussé est la sauvegarde de référence de ces sources.

Métadonnées PC : `~/.cache/rag3weaver-build/cx-rate-key-59ac-v1` existe avec
logs/reçus/fixtures/journaux, sans target-activation listé au premier niveau.
Il n'a PAS été supprimé. Archives-embarquement-15nov/16nov et pile-embarquement
existent ; leur contenu/runtime n'est pas requalifié. LOCAL conserve aussi
des historiques cache lourds étrangers ; aucun effacement. Aucun nouveau
target cargo créé pour l'acteur, les owners ou la passation ; pas de mesure
récursive de taille ni scan de base dans cette passation.

## Processus, services, verrous et ressources au STOP

Aucune exécution persistante lancée par cette session encore active à notre
connaissance : ses derniers appels étaient lectures Git/métadonnées et
publications terminées. Aucun PID/scope owned à arrêter ni réservation
provider maintenue. Le watcherV4 est terminé depuis son rouge ; Q8/Q9 et
leurs scopes étaient rapportés TERMINE/nettoyés par Arbre. Aucun process
nouveau lancé pour vérifier cela au STOP ; aucun scan /proc/health effectué.
Ce constat ne certifie pas l'absence globale de travaux d'autres sessions.

Aucun verrou poste ou bail modèle détenu par Embar. Pas de restart, warmup,
modèle/SDK/changementGPU/7970/service ni tunnel ; connexions baseline laissées
intactes. Aucun signal sur PID historique provider. GPUcard0 interdit ;
card2 et services restent soumis à future coordination, disponibilité inconnue.
SDK historique59ac complet :
`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
Ce nom n'autorise ni build ni runtime. C7/Arbre/G conservent leurs autorités
et leurs ressources ; ne pas les nettoyer par recherche de noms.

## Trois prochains pas — lecture inactive seulement

1. Après reprise explicite, lire synthèseC7 et passations17 Arbre/G/TTY/R/ED,
   puis vérifier les têtes et les tickets. Commandes inactives :
   `git -C /home/lucied/git_workspaces/cx-embar-passation-20261117 status --short`,
   `git -C /home/lucied/git_workspaces/cx-embar-passation-20261117 rev-parse HEAD`,
   `git ls-remote --heads origin cx-embar-passation-20261117-r1`.
   Aucun test/fetch/checkout de force ni reprise de coupon.
2. Lire les preuves Q9 publiées et les verdicts indépendants AVANT choisir la
   suite B/original. Commande inactive :
   `git show 6e97b2ff8a6a8f4e4ca095ce3033fce10ceb8549:docs/arbre-principal/17-11-2026/09-cycle-q9-observe-preuves.md`.
   Reprendre le parent B et compléter pins/source/profil/qualifications sans
   copier DATA ni contacter un provider. Tout runtime exigera un nouveau
   GO exact, scope neuf et garde fraîche ; les END anciens sont morts.
3. Pour TTY/global, lire le contrat inactif et rapprocher les nouvelles
   passations :
   `git show ec5e675cc2870e5987baee226e17ff43e4175b7d:docs/orchestration/17-11-2026/coordination/tty-provider-contract-preparation.md`.
   Avant toute activation : matérialiser horloges/autorités/phases/PGtools,
   payloadcaps/leases/release, couple publié +2RO neuves, puis nouveau GO.
   Ne pas appeler health pour « préparer » une future admission.

Aucune de ces lectures n'est une demande de reprendre maintenant. RetourC7,
puis **STOP**, y compris aux rappels automatiques.
