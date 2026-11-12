# M8 : adoption, fusion et reprise — relecture du 12 novembre 2026

**État : conception relue, aucun lot de code ouvert.** Porteur : mémoire ; relecture :
everything declarative. Le [ticket M8](../tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md)
reste ouvert. Cette note révise la [conception du 9 novembre](../../../extension/rag3weaver/docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md) ; elle ne livre ni adoption, ni montée, ni migration.

## Périmètre et preuves

Relecture sans compilation, test, base réelle ou activation. Le code de référence lu est
`da69b842c3d73c97b07f52124a6852219188894e` ; le worktree documentaire part de
`7004350fb6b0117a9b675d65da2121709fd7a146`. Le précontrôle 1b relu à
`85ef4921128b445e74766c1e628809b206957485` est un diagnostic : il n'autorise pas
l'application ou la publication d'une migration. Le lot équipier figé et le rouge Gemini
restent inchangés ; aucun nouvel essai.

Les références ci-dessous désignent les fonctions de cette révision, pas une preuve d'exécution :

- [builtin_templates.rs](../../../extension/rag3weaver/src/builtin_templates.rs) : `Livre`
  (244), `state_root` (276), `record` (323), `local_copies_avec` (167), `refresh` (397).
- [lecteur.rs](../../../extension/rag3weaver/src/lecteur.rs) : `Recouvrement` (87), avec
  [backend.rs](../../../extension/rag3weaver/src/backend.rs), `PreparedBackend::load_avec` (1075).
- [daemon_host.rs](../../../extension/rag3weaver/src/daemon_host.rs) : `open_slot` (558),
  `close_slot` (625), `close` (749).
- [migrations.rs](../../../extension/rag3weaver/src/dataflow/migrations.rs) : `pending`
  (363), `apply` (382), `dry_run_migration` (434), `apply_migration` (491), `rollback` (555).
- [journal_des_runs.rs](../../../extension/rag3weaver/src/daemon_host/journal_des_runs.rs) :
  `avec_bornes` (196), `runs_de` (232), `lire` (249).

## Constats

`Livre` conserve le nom livré, le JSON de base du manifeste et l'empreinte du manifeste écrit.
Il ne conserve pas les bases des autres fichiers. `local_copies_avec` reconnaît des chemins par
suffixe parmi les valeurs du manifeste ; cela ne prouve ni leur origine historique ni la couverture
transitive des dépendances de graphes et scripts. `link` ne remplace que les copies identiques au
livré courant. `Recouvrement` et `load_avec` permettent déjà de préparer et charger une proposition
sans réécrire les déclarations sur disque.

`refresh` refuse un manifeste retouché ou suivi par Git, mais écrit les manifestes éligibles à
l'ouverture : `open_slot` l'appelle, et `refresh` écrit le manifeste puis le livre (465–467).
Sa fusion réapplique les différences locales sur le neuf (419–432), sans détecter les changements
concurrents divergents. Elle ne constitue donc pas la fusion avec conflits promise par M8.
`builtin:` désigne le contenu du binaire courant : ce lien seul n'épingle pas son contenu.

`close` ferme le slot ; le prochain appel peut le rouvrir. La
[page de migration du 25 octobre](../../../extension/rag3weaver/docs/25-octobre-2026-memoire-longue/05-la-migration-de-la-fiche-de-lucie.md)
le dit expressément. Cette page décrit une opération historique et des scripts de scratchpad,
pas une primitive atomique de maintenance et de restauration livrée.

Le runner fournit fichiers de migrations, variables, registre de nœuds, validation de graphes,
checkpoints et undo. À la révision de référence, `apply` calcule les migrations avant le verrou
(389/399), l'identifiant d'exécution est fondé sur version/nom (510), et le journal de succès ou
d'échec est écrit après le graphe (513–535). Ces mécanismes ne prouvent pas une publication atomique
entre base, fichiers, déclarations et livre. Le diagnostic 1b ajoute une qualification prudente,
pas un droit d'exécuter M8.

Le journal des runs est borné : les appelants observés ne sont pas tous les clients possibles.
Un graphe identique sous un autre nom suggère un renommage ; il ne prouve pas une identité d'outil.
Enfin, la protection des tests contre le dossier d'état réel existe désormais dans `state_root`
et les fixtures e2e : le signal des 400 livres est historique, sans mesure nouvelle ici.

## Recommandations

### Adoption et base récupérable

Étendre le livre existant avec un inventaire explicite des fichiers possédés et de leurs bases.
Résoudre `--from` en commit exact, chemin livré et empreintes vérifiées ; « master du 25 au soir »
n'est pas une base identifiée. Les différences locales deviennent des modifications à conserver,
pas une preuve que la copie historique était exacte. Base absente, ambiguë ou indisponible : refus
nommé sans écriture. Inclure les dépendances transitives, fichiers renommés, supprimés ou non livrés.

Préférer des contenus de base adressés par empreinte dans le même état que le livre, qui les référence.
Git peut fournir ces contenus lors de l'adoption ; l'installation du binaire n'a alors pas besoin
d'un clone complet pour chaque montée. Cette option reste à trancher. Un hash sans contenu
récupérable ne suffit pas pour fusionner.

### Fusion pure et validation

Une primitive pure base/local/cible doit exprimer : local égal base prend cible ; cible égale base
prend local ; local égal cible garde la valeur commune ; sinon conflit. Distinguer absence et null,
traiter les suppressions contre modifications et les changements objet/scalaires avant la descente.
Les tableaux restent atomiques sans contrat de fusion supplémentaire. Pour les textes, `merge-file`
est une possibilité ; conflit et erreur doivent être distingués et tous deux interdire la montée.
Valider le candidat complet en recouvrement, sans marqueurs de conflit servis comme déclarations.

Complément mémoire, constaté par lecture à `da69b842c3` et `7004350fb` :
`load_avec` peut encore appeler `builtin_templates::resolve`, puis `root`, pour un `builtin:`.
Si le cache manque, cette résolution peut matérialiser les gabarits sur disque. Le recouvrement
ne rend donc pas à lui seul toute la validation sans effet ; il faut préciser la frontière
du plan et prévoir le cas de cache vide, sans confondre absence d’écriture des déclarations
et absence de toute écriture. Aucun essai de ce chemin ni correction n’est livré ici.

### Identités et compatibilité

Distinguer identité du paquet de gabarits (inventaire canonique des chemins et contenus), provenance
Git, état local et cible. Ni cette identité ni son commit ne sont une version de schéma interne,
un format moteur/applicatif/de fichiers ou une génération de données. Un compte de commits de retard
n'est pas universel entre branches ou binaires distribués sans Git.

Comparer aussi identité des entités, relations, index/signaux, formats de fichiers et valeurs par
défaut : « ajout facultatif » n'est pas en soi une preuve de compatibilité. Refuser les transitions
sans chemin réellement pris en charge avant toute ouverture du backend cible susceptible d'écrire.
Les champs propres au backend ne se réduisent pas à une liste figée de quatre clés.

### Maintenance et reprise communes

M8 doit composer les contrats communs d'admission, d'exclusion de la ressource physique,
de migration et de récupération, notamment ceux du
[ticket de migration de l'ancre](../../arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).
Il ne doit pas créer un deuxième moteur de migration. Ces garanties restent à fournir et prouver
sur les chemins concernés ; les citer ne les rend pas livrées.

L'exclusion doit couvrir appels, réouverture automatique et écrivains durant toute la maintenance,
avec revalidation des empreintes après acquisition. L'instantané vérifié doit couvrir base et
fichiers moteur, déclarations, livre/objets et fichiers sous `files_root` si transformés, même hors du
dossier de configuration. Un journal durable doit permettre de reprendre après interruption,
distinguer publication et vérification, contrôler la restauration et maintenir le backend fermé
si la récupération échoue. L'undo d'un graphe ne remplace pas cette preuve multi-ressources.

## Choix non tranchés et première tranche proposée

Mémoire et orchestration doivent préciser la coexistence du suivi vivant déjà demandé par Lucie
et de la montée explicitement épinglée. La formule « l'ouverture dit seulement » ne décrit pas
`refresh` aujourd'hui. Ne pas désactiver implicitement le suivi vivant ni annoncer une épingle
qui laisserait `builtin:` évoluer sous elle. La forme persistée des bases, l'identité des outils
renommés et le contrat de maintenance/reprise restent également à trancher.

Proposition de première tranche : un plan d'adoption/montée **à blanc**, à partir de bases fournies,
avec fusion pure du manifeste et des fichiers, validation en recouvrement, diff des outils,
appelants observés et refus structurés. Aucun apply, aucun nouveau livre écrit, aucune activation.
Les témoins à prévoir couvrent base absente/ambiguë, retouche préservée, ajout divergent,
suppression contre modification, conflit de forme, fichier manquant, local égal cible,
ordre déterministe, absence d'écriture, changement de schéma non pris en charge et historique
ancien non qualifiable. Ils ne sont pas exécutés par cette relecture.

Une tranche d'adoption persistée pourra suivre ; l'application attend le contrat commun de
maintenance/instantané/reprise et ses preuves sur copie privée. Toute montée de la mémoire réelle
reste une décision explicite distincte. Une conception ne ferme pas M8.

L'identité Git applicable est `Lucie Defraiteur <luciedefraiteur@gmail.com>` ; les anciennes mentions
d'identité neutre sont retirées de la conception courante. Elles ne justifient aucun commit anonyme.
