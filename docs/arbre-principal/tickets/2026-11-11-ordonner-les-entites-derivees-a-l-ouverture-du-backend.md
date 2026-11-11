# Ordonner les entités dérivées à l'ouverture du backend

- **État** : ouvert, initialisation corrigée sur `bffbe5033` ; reste le raccord public search_index/manifeste du lot 1a.
- **Famille** : produit
- **Gravité** : refus de chargement et d’ouverture reproduits puis corrigés ; preuve du raccord public 1a encore manquante.
- **Atteignable en service** : oui sur les baselines décrites ; correctif d’ouverture éprouvé, association publique 1a non livrée.
- **Touche rag3weaver** : oui, src/backend.rs, enregistrement du schéma ; DerivedConfig et Catalog::verifier_la_derivation.
- **Pour** : arbre-principal (raccord public/manifeste 1a) ; ED conserve les preuves du correctif d’initialisation.
- **Signalé par** : arbre-principal, [ticket du contenu partagé](../../arbre-principal/tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).

## Éléments relus

PreparedBackend conserve les configurations dans un HashMap<String, EntityConfig>. Son ouverture
initialise un catalogue vide, puis appelle register_entity dans l'ordre de cette HashMap ; les
relations déclarées sont enregistrées après les entités. Catalog::verifier_la_derivation exige que
la racine soit déjà enregistrée et que les relations de gather existent. Ces contraintes doivent
être composées à l'ouverture, indépendamment de l'ordre des noms ou de la HashMap.

Repères de lecture sur la base du lot arbre-principal 49caf64c6 : backend.rs vers 961 et 2248–2308,
catalog.rs vers 1935. Les primitives natives du contenu partagé sont éprouvées par API Catalog,
mais cela ne prouve pas encore l'accessibilité depuis un manifeste de backend.

## Travail attendu et critères de fin

Poser un e2e minimal de chargement/ouverture de manifeste avec racine, index dérivé et association
publique search_index, via le chemin normal PreparedBackend. Reproduire ou invalider le risque
d'ordre ; réutiliser l'ordonnancement existant s'il couvre déjà ces contraintes, sinon composer
racines, relations puis dérivées selon leurs dépendances, avec refus explicite d'un cycle ou manque.
Prouver écriture sur la racine et recherche sur sa cible publique sans dépendre des noms ni des
semences de HashMap. Les dérivées avec gather doivent également avoir leurs relations disponibles.

Aucun cas spécial Scope, aucune activation des bases de service. Ce ticket est distinct des preuves
natives du lot 1a et n'autorise pas un nouveau lot de code sans coordination avec l'orchestration.
Le porteur peut invalider le risque avec sa preuve ; il ne doit pas être présenté comme reproduit avant ce témoin.

## Départage ED et choix sémantique

Sur master `fc3ca92aa`, témoins seuls `02948aebb`, trois rouges indépendants
(`0/3/0`, 0,11 s ; `I-luciepc/ouverture-20261111/derivees-rouge.log`) :

- Le chargement refuse l'identité héritée de la dérivée : exigence hashsafe
  alors que la validation locale interdit hashsafe sur une dérivée.
- Après isolation de ce verrou de chargement, l'ouverture refuse la racine
  absente quand la dérivée occupe réellement la première position de la map.
- Avec les racines effectivement premières, l'ouverture refuse la relation
  gather qui n'est enregistrée qu'après les entités.

Les témoins contrôlent l'ordre observé sans supposer une graine et sans répétition
aléatoire. Les deux derniers isolent la phase open en appliquant la déclaration
dérivée à une version préparée de racines ; le premier et le nouveau parcours
complet couvrent le vrai chargement de manifeste.

Plan privé d'initialisation, choix relu et validé par orchestration : entité dérivée
dépend de sa racine et de ses relations gather ; une relation dépend de ses
extrémités. Validation du plan complet au chargement et avant le premier appel
base à l'ouverture, tri stable et refus nommé des dépendances absentes/cycliques.
Le corps pur de Catalog.verifier_la_derivation est partagé dans
DerivedConfig.validate_context (résolveurs de configurations/relations), le
catalogue conserve sa vérification individuelle ; direction, champs et refus de
racine elle-même dérivée restent identiques. Pas de nouvelle chaîne ou cas Scope.
Seules les dérivées sont exemptées de l'exigence hashsafe (identité de la racine).

Source `be6646344` sur master `2e910930d` : **six témoins ouverture verts**
(0,60 s), dont chargement/ouverture réels, écriture racine puis recherche sur
la dérivée rassemblant un voisin, trois permutations des noms et zéro appel base
sur refus après préparation. L'essai précédent 5/6 lisait à tort un UUID ParLot
avant drain (Pending), corrigé dans la fixture, pas dans le produit.
Preuve : `cibles-2.log`. La passe avec/sans code sur `a166474a0` reste en cours.
Lecture croisée avec arbre principal demandée pour l'identité root/content ;
aucune source 1a importée. L'alias public search_index est absent de cette base :
son raccordement et sa preuve finale relèvent de la combinaison avec 1a et restent
à faire avant clôture de ce ticket.

Premier résultat `a166474a0` sans code : ouverture6/0/0 (0,58 s), module
builtin_templates9/0/0 (0,01 s). Lib native entière et ciblés avec code encore
en cours ; preuve `cibles-3.log`, aucun verdict global anticipé.

Ciblés terminés sur `a166474a0` : sans code, ouverture6/0/0 (0,58s),
gabarits9/0/0 (0,01s), lib native entière1575/0/15 ignorés (20,44s).
Avec code, ouverture6/0/0 (0,60s), gabarits9/0/0 (0,01s), aucun ignoré dans
les ciblés. `cibles-3.log`, `lib-native-3.log`, `verdict-cibles-3`.
Ce vert ciblé ne remplace pas la passe globale ni la lecture croisée avec 1a.

## Complément de revue : schéma nommé du workspace

Orchestration demande de vérifier que le plan compose aussi le fournisseur de
schéma nommé `workspace.index=code` : entités et relations fournies par le moteur,
absentes des entités explicites du manifeste. `code::plan_schema()` existe pour
les faits des plans ; le montage réel utilise `code::register_code_schema`.
Le plan candidat ne voit encore que les entités/relations explicites.

Après la passe08, trois témoins comparent baseline et candidat : relation du
manifeste vers une entité nommée, dérivée depuis cette entité, gather par une
relation du schéma nommé. L'hypothèse de régression de la relation n'est pas encore
reproduite : le chargement baseline vérifie déjà les extrémités contre ses seules
entités explicites (backend.rs1397), donc la comparaison est nécessaire.
Aucune duplication de schéma Scope ou pseudo-configuration ne sera utilisée ;
les faits doivent venir du fournisseur existant. Source08 et sa passe restent
inchangées. Ce point de revue peut empêcher l'intégration même si la passe08 est verte.

Comparaison terminée après fin de la passe08 : baseline `6cf2bc8b5` (base2e,
tests seuls) et candidat `db79fd2a1` donnent chacun **0/3/0**, 0,01s.
La relation custom vers Scope échoue identiquement (`ANNOTATES: unknown relation
endpoint`) : **pas de régression prouvée de ce chemin**, il était déjà refusé.
Les deux dérivées sont refusées sur baseline par hashsafe, sur le candidat par
la racine Scope absente des entités explicites. Il reste donc une limite de
composition préexistante, pas une réussite du point de revue. Preuves externes
`nomme-baseline.log`, `nomme-candidat.log`, `commit-nomme-*`.

La passe08 s'est achevée verte avant cette comparaison : features5, gardes20,
lib2121/0/64 ignorés (41,73s), navigateur39/0/0 (132,88s), sept suites e2e
sans SAUTÉ : dérivées4, lectures24, PG34, txvecteurs3, txarrêt18, txmerge1,
txligne3. Elle reste étiquetée08, ne valide pas le schéma nommé et n'autorise
pas le push tant que ce point de revue n'est pas départagé/corrigé.

## Sous-travail fournisseur confié à arbre principal

Extraction pure autorisée par orchestration ; porteur arbre principal pour code.rs,
ED pour la composition Backend. Worktree arbre neuf `cx-wt-schema-code-provider`,
base master `c04c62021`, indépendant de la source 1a. Signature stabilisée :
`code::schema_configs(scope_chunking: ChunkingConfig) ->
(BTreeMap<String, EntityConfig>, BTreeMap<String, RelationDef>)`.
La primitive alimente plan_schema et register_code_schema avec les mêmes configs
et propriétés ; ordre de montage existant File/Scope/Library/Symbol puis RELATIONS
conservé. Faits Scope (chunking, hashsafe) préservés, aucune pseudo-configuration
construite dans Backend. SHA de livraison attendu avant compilation combinée.
Ce sous-travail reprend ce même ticket, sans doublon ni clôture par transfert.

Fournisseur livré par arbre `c12f840b4`, repris en `820bfc0ee` (code.rs seul,
aucune source1a) ; test propre du fournisseur ajouté mais pas encore compilé.
Composition Backend : faits réels explicites/fournis, validation commune avantDB,
définitions identiques coalescées, conflits d'entités nommés. Relations : mêmes
extrémités et union additive de propriétés compatibles, conflit de même champ
ou d'extrémités refusé ; None et carte vide équivalents. Choix validé par
orchestration, overlay workspace.index_signals conservé. Nouveaux témoins de
coalescence, propriétés/propriétés incompatibles et compteur zéroappelDB prêts.
Deux cas Catalog additif/conflit d'extrémités compareront la baseline existante.

Correction de fixture avant le vert nommé : `HAS_SCOPE` n'est pas une relation
du fournisseur. La vraie relation est `DEFINED_IN` (Scope→File, direction out,
champ path). L'ancien troisième rouge, arrêté avant gather par hashsafe/racine,
prouvait seulement le verrou amont ; aucune validité de cette ancienne règle
n'est revendiquée. Le cas corrigé sera rejoué sur baseline et candidat.

Source combinée `11ea6ec87` : sans code ouverture6/0/0 et gabarits9/0/0 ;
lib native entière1575/0/15 ignorés (20,60s). Avec code ouverture16/0/0 (0,70s),
gabarits9/0/0 (0,01s) et test fournisseur1/0/0. Les trois cas nommés corrigés
passent, dont DEFINED_IN/out/path ; propriétés de MENTIONS étendues sans perdre
kind, conflits de type/endpoints refusés, compteur zéroappelDB. Overlay déclaré
Scope en BM25 vérifié après ouverture, pas traité comme conflit du fournisseur.
Preuve `cibles-4.log`, `lib-native-4.log`, `verdict-cibles-4`.

Baseline corrigée `292b61081` (2e+tests seuls) : les trois cas nommés restent
0/3/0 (0,01s), tandis que deux cas du contrat Catalog existant sont verts2/0/0
(0,16s) : propriétés ajoutées à MENTIONS en gardant les précédentes ; refus de
changer ses extrémités. Preuves `baseline-opening_named_schema_.log`,
`baseline-named_catalog_existing_.log`, `commit-baseline-contrats`.
Le nouveau refus de conflits de type est prouvé dans la composition Backend,
pas présenté comme une nouvelle garantie de l'API Catalog individuelle.
Reste la passe globale de la composition, tour et intégration ; alias1a toujours
à prouver après raccordement. Ce ticket reste ouvert jusque-là.

## Validation finale et passage de lot

Source Rust/UI/e2e `bffbe5033` : features5/5, gardes31/0/0, lib2132/0/64 ignorés
(43,16s), navigateur39/0/0 (132,98s). Sept suites e2e sans SAUTÉ : dérivées4,
lectures24, PG34, txvecteurs3, txarrêt18, txmerge1, txligne3. La suite txarrêt
laisse un CSV connu (12K), suivi par
`docs/recherche/tickets/2026-11-11-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md` :
ce vert ne signifie pas absence de résidu. Les ignorés de la lib sont ses marques
existantes ; les tests navigateur sont joués séparément.

Native sans code sur11ea : ouverture6 et gabarits9 verts, lib1575/0/15 ignorés
(20,60s). Avec code : ouverture16 et gabarits9, fournisseur1 verts. Les propriétés
compatibles s'ajoutent, les identités et configurations incompatibles ne
s'écrasent pas ; refus nommés avant le premier appel base. La baseline292 prouve
le contrat Catalog additif et le refus d'extrémités, deux verts. Les anciens
rouges de fixtures et les vrais rouges produit restent séparés dans les logs.

Après rebase1365b872b, diff depuis bff strictement deux Markdown : Rust/UI/Cargo/
gabarits et helper inchangés. Helper65aa SHA dca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628,
19 tests verts aux deux postes ; nouvelle exécution sur SHA documentaire final
avant push. Tour « à toi » accordé par orchestration après lecture du verdict.
Preuves sauvegardées aux deux postes : `~/.cache/rag3weaver-build/I-luciepc/ouverture-20261111/`
(`verdict-final.json`, `*-passe-2.log`, `cibles-4.log`, logs baseline et garde).
Push sous poste push/garde VERT, trois contrôles puis archives administratives
fidèles hors cible et garde fraîche avant chaque retrait ; aucune suppression
si refus. Worktrees/target/PG du reste-équipier restent hors de ce lot.

## Reprendre après transfert ouvert à arbre principal

Accord orchestration : ce même ticket passe à arbre principal avec le correctif
ED validé ; aucun doublon, aucune clôture par transfert. Restent : charger/ouvrir
le manifeste combiné1a, valider son association publique search_index avant toute
écriture et prouver écriture racine/recherche par la cible publique. L'association
est de lecture et ne crée pas d'arête de création Root→Index (Index.from dépend
Root, l'inverse fabriquerait un faux cycle). Conserver les modes root/content,
leurs règles d'identité et le refus des chaînes from→dérivée. Relier la preuve
combinée et le commit1a avant git mv de ce ticket vers closed et État final.

## Reprise arbre après intégration ED

Rebase 1a sur c693496a8, branche neuve cx-arbre-scope-version-4 (premier checkpoint b68cebd46).
Aucun conflit runtime, aucune preuve 1a attribuée à la passe ED. Les copies historiques
recréées dans everything-declarative par le rejeu sont retirées au profit de ce ticket
transféré et du ticket de feature déjà fermé ; leurs preuves initiales sont conservées ici.
Le WT fournisseur c12 a été retiré au portable sans force après archive administrative
fidèle et helper65aa exact VERT : cx-provider-nettoyage-c12/{administration,garde.json}
sous ~/.cache/rag3weaver-build. Aucun target ni sous-module initialisé dans ce WT.
Restant : validation partagée de l'association, sans nouvelle arête de création,
vrai chargement/ouverture/écriture/recherche par cible publique dans le manifeste combiné.

Deux témoins combinés sont posés sur le socle ED : vrai manifeste public root/content avec
permutation des noms, deux racines/une version et trois modes de lecture ; refus des
associations invalides à la préparation et après mutation du préparé avant le premier appel
base. La validation d'association n'est pas encore raccordée : rouge attendu, pas affirmé.
Le ticket reste ouvert, ED et 1a gardent leurs preuves distinctes.

## Rouge combiné 1a et frontière partagée

Source **490e0acf6951e7fcade74a5d4febf4571089ddca**, après rebase sur ED c693 et mise
au commit a730 du sous-module. Log `cx-manifeste-public-1a-rouge.log` sur luciepc :
**1 réussi, 1 échoué**. Le vrai chargement/ouverture/écriture/recherche public est déjà
vert : trois permutations de noms, une version pour deux racines, trois modes de résultat.
Le rouge est réel et distinct : `searchIndex: Missing` est accepté à la préparation.
Les autres refus et le compteur zéroappelDB ne sont pas déduits de ce premier échec.

Correctif candidat : `EntityConfig::validate_search_index_context` vérifie les faits réels,
appelé par la Forme de recherche et par SchemaFacts à la préparation/avant open. Même règle
pour index présent, identité content/from correct, absence de récursion, contenu cherchable
et champs indexés (titre compris) existant sur l'occurrence. Aucun doublon de validation
métier, aucune pseudo-config fournie, aucune arête Root→Index dans l'ordre de création.
La validation de dérivation root/content et le refus des chaînes restent ceux partagés par ED.

Source combinée **eb02a9546**, `cx-manifeste-public-1a-vert.log` sur luciepc : **2/0/0**,
0,60 s. Vrai manifeste public chargé et ouvert, trois permutations, deux racines/une version,
lecture des occurrences dans les trois modes ; fermeture de lecture comprenant l'index.
Les cinq refus d'association passent (inconnu, mode root, mauvaise origine, soi-même,
chaîne) ainsi que la mutation du préparé avant open avec compteur **zéro appel base**.
Ce sont les preuves combinées 1a, distinctes des 16 ouvertures ED et du fournisseur c12.
La vraie passe réglementaire combinée reste à jouer ; ticket conservé ouvert jusqu'à son
verdict et au commit final, puis git mv vers closed selon la procédure.
