# Publication composite sur un catalogue neuf possédé

Suite bornée du [ticket migration](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md),
après A et RO, autorisée par le [plan de nuit](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Ce document prépare la première primitive ; il ne décrit pas encore une capacité livrée.

## Objet effectivement publié

Un **instantané déclaratif complet** produit par A : EntityConfig effectives,
relations et politiques, fournisseur, octets des recettes résolues et modèles
déclarés. Le producteur A valide la complétude et les dépendances ; son mécanisme
JSON canonique est réutilisé, sans second compose ou hash ad hoc. Le catalogue
neuf de cette tranche est dédié à ces instantanés : il ne contient pas les données
ou vecteurs d'un index existant, ni des références à des couches parentes externes.

L'empreinte H porte sur ce contenu déclaratif complet, avec un domaine versionné
explicite. Elle ne devient ni empreinte des données physiques ni génération de
stockage. Le registre conserve **le contenu**, ses publications immuables, sa tête
et les reçus d'opérations : publier ne consiste pas à écrire seulement un statut.

| Événement | Publication | Contenu déclaratif | Empreinte |
|---|---|---|---|
| Création explicite de l'origine O | P1 | A | H-A |
| Nouvelle opération sur P1 | P2 | B | H-B |
| Retour logique sur P2 | P3 | A | H-A |
| Même opération déjà validée, rejouée | P inchangée | contenu déjà enregistré | H inchangée |

Les identifiants O/P sont des identités d'événements persistées, indépendantes du
contenu et du chemin. Réutiliser le générateur d'identifiants de corrélation
existant pour ces occurrences, pas HASHSAFE qui représente une identité de contenu.
L'identifiant d'opération est explicite et existe avant la demande, afin de
résoudre une réponse perdue. Ce n'est pas un secret ou une autorisation.

## Frontière avec le stockage et les migrations

Le registre publie des déclarations, pas leur application au catalogue d'index.
Aucun `storage_descriptor:v1`, PlanJournalState::Applied, transfert, bootstrap de
legacy ou validation d'un format cible n'est produit. Le journal `_DataflowMigration`
reste celui des fichiers Mermaid et garde ses refus sur les identités manquantes.
Il ne peut pas certifier cette nouvelle opération par son seul ancien statut.

La primitive manquante est un **intent de publication figé**, pas un second moteur
Mermaid : origine attendue, publication et empreinte source attendues, identifiant
d'opération, publication cible préallouée et instantané cible complet. Son identité
est calculée par le mécanisme canonique commun et un domaine propre versionné.
Il n'est jamais modifié pour suivre la tête courante. Le rapprochement avec un
plan de migration et des données physiques reste une suite distincte.

Après P1→P2→P3, un intent non exécuté préparé sur P1 refuse P3 même si H-A est
retrouvé. Réutiliser son instantané cible déjà validé permet de préparer un
**nouvel intent sur P3**, avec un identifiant propre ; l'ancien reste identique.
Cette réutilisation concerne les déclarations fermées et leurs validations,
pas des calculs externes dont les dépendances ne seraient pas requalifiées.

## Création, ouverture et transaction

Entrée native publique, gardant sa connexion d'écrivain privée :

```rust
// Surface de la primitive proposée, noms ajustables avant le diff.
let origin = NewPublicationOrigin::fresh();
let expected_origin = origin.identifier().to_owned();
let initial = DeclarationSnapshot::from_complete_input(input_a)?;
let mut store = OwnedPublicationCatalog::create_new(path, origin, operation_1, initial)?;
let plan = store.prepare(operation_2, snapshot_b)?;
let receipt = store.publish(&plan)?;
let current = store.observe()?;
```

- `create_new` crée atomiquement un répertoire absent, puis un catalogue natif
  dédié sous ce répertoire. Toute cible déjà existante est refusée avant ouverture
  en écriture. Aucun chemin ne fournit une identité et aucun répertoire étranger
  n'est effacé sur un échec. L'origine attendue est générée/conservée par l'appelant.
- Avant toute identité H de publication et tout effet, refuser une entrée qui
  change au roundtrip du codec : comparaison canonique complète et égalité
  typée des déclarations ModelSource (incluant AddressesVec). Le constructeur
  ne normalise pas silencieusement une entrée déjà fournie.
- Réutiliser `_catalog_meta`, le dialecte et les décodeurs stricts communs. Une clé
  versionnée distincte contient le registre de déclarations. Extraire seulement
  l'écriture KV commune actuellement dans Catalog::persist_meta_key si nécessaire.
  Pas de Catalog::initialize ou de modèle pour publier des déclarations.
- `open_owned` n'initialise rien : présence du catalogue et protocole existant
  exigées, validation RO de l'origine attendue et du registre avant l'écrivain,
  puis revalidation sous sa vraie exclusivité. Catalogue vide/legacy/protocole
  absent, table, colonne ou clé étrangère, type/défaut/clé primaire hors contrat, contrat inconnu, record incomplet/corrompu : refus.
- Le catalogue dédié ne contient que le registre et sa table de métadonnées ;
  l'ajout d'une donnée hors contrat refuse au lieu d'être couvert par une empreinte
  déclarative. Un futur élargissement à un catalogue d'index sera un autre contrat.
- Sous la connexion exclusive et une vraie transaction : relire/valider le registre,
  vérifier O/P/H attendus, refuser une opération homonyme avec un autre intent,
  ajouter la valeur et sa publication/reçu immuables, avancer la tête, COMMIT.
  Avant COMMIT, tout échec entraîne tentative de ROLLBACK ; ses erreurs sont gardées.
- Une erreur de COMMIT ne prétend pas établir l'issue : le handle devient inutilisable
  jusqu'à réouverture et lecture du reçu. Pas de second effet automatique.
- Rejouer exactement un intent déjà enregistré rend son reçu et ne republie rien.
  Si la tête a ensuite avancé, le reçu indique ce caractère historique et la tête
  courante ; il ne qualifie pas celle-ci comme cible de l'ancien intent.

Un document de registre unique permet l'atomicité de contenu/tête/reçus, avec
validation complète de la chaîne et des empreintes à la lecture. Coût assumé pour
cette première tranche : lecture/validation et réécriture O(nombre de publications).
Il n'est pas présenté comme un index scalable ni comme un journal d'exécution.
Les instances sont privées, non clonées, avec méthodes nécessitant l'accès mutable ;
le moteur natif assure l'exclusivité de l'écrivain entre ouvertures.

## Primitives réutilisées et fichiers

- A : `DeclaredSchemaFacts::from_complete_input`, `facts::canonical_json` et refus
  des valeurs non finies ; ajouter Deserialize aux **entrées** seulement pour
  reconstituer/valider les instantanés stockés, jamais aux faits ou reçus scellés.
- `DbConnection::{begin,commit,rollback,begin_read_only}`, Rag3dbConnection avec
  tampon explicite borné ; le read-only avant writer suit l'ordre établi par RO.
- `catalog::metadata::{decode_rows,...}` et le dialecte `_catalog_meta` ; une seule
  fonction d'écriture KV partagée si l'extraction est nécessaire.
- Nouveau module `migrations/publication` (protocole pur et stockage natif gardé
  par feature), petit export dans migrations.rs. Pas de Cargo/lib.rs/modèles/agent/IDE.
  Catalog::persist_meta_key délègue au helper commun ; aucun autre chemin de Catalog
  ni son initialisation n'est modifié. Mémoire relit sans éditer ces fichiers.

## Témoins et qualification prévue

Fixtures synthétiques neuves uniquement, identités explicites et tampon64MiB.
Aucune base de Lucie, aucune donnée ou conversation réelle, aucune carte/modèle.

1. Création O/P1 et vraie réouverture : valeur/fingerprint/reçu persistés, aucun
   storage_descriptor ni application de schéma physique déduits.
2. P1(A)→P2(B)→P3(A), P distincts/H-A retrouvé, instantanés précédents immuables.
3. Même opération/intent rejoué : même reçu, aucune nouvelle publication ou tête ;
   opération homonyme avec cible ou source différente refusée.
4. Origine étrangère et publication étrangère (même H compris) refusées ; nouvel
   intent après requalification distinct, ancien identique octet pour octet.
5. Catalogue déjà existant/legacy, état incomplet, contrat/fingerprint corrompus,
   clé/table étrangère : refus avant effets et aucun bootstrap.
6. Interruption après mise en transaction avant COMMIT puis réouverture : état
   précédent intact. COMMIT réussi/réponse perdue : reprise par même opération,
   reçu retrouvé sans répéter l'effet. Témoins enfants natifs et erreurs transactionnelles.
7. Source/recette publiées avant ciblé natif sous commonconfiner/j8/Max32/target32/
   lib29/CPU ; premier rouge inattendu arrêté/diagnostiqué, pas de retry inchangé.
   Passe adaptée et tour C7 avant master ; nettoyage réglementaire après preuves.

Les nouveautés/tests ne seront annoncés comme livrés qu'après leur exécution et
l'intégration. Le ticket migration reste ouvert pour la pyramide et l'application.
