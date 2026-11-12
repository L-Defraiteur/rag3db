# Transfert générique de valeurs connues — tranche à relire

Préparation sur master **6e5f9458c**, 12 novembre 2026. Porteur : arbre-principal ;
revue commune : orchestration. [Ticket canonique](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md),
[répartition parallèle](../../orchestration/tickets/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md).
Aucun Rust modifié, Cargo lancé, transfert exécuté, target, base ou service créé.
Les signatures ci-dessous sont proposées, pas disponibles dans le produit.

## Résultat borné et dépendances

La première tranche de code proposée construit en mémoire un **aperçu déterministe
et non exécutable** du transfert : occurrences→versions→chunks et correspondances
de valeurs connues par cellule et modèle. Elle vérifie la couverture complète,
sans ouvrir de base, écrire de ligne ou disposer d'un embedder. Elle ne prétend
pas livrer T3–T6 sur stockage ni T7–T12 de publication/reprise.

Le préflight livré et le socle1a restent les frontières : ResolvedMigrationPlan
fixe source/cible, variables et faits ; DerivedIdentity::Content définit la version ;
MigrationRunner est l'unique orchestrateur futur. Pas de second runner, journal,
mode d'ouverture permissif ou chemin d'ingestion qui recalcule un manque.

**Deux prérequis du transfert effectif restent absents** : identité durable de
l'instance de stockage source et génération établie de ses données. Le descripteur
actuel n'a pas d'identifiant d'instance ; l'inspection peut rendre Unversioned.
Ni chemin, hash de configs, empreinte d'inventaire, checkpoint ou UUID des lignes
ne remplace ces deux preuves. Les fixtures peuvent déclarer explicitement une
identité/génération synthétique ; cela ne crée aucun producteur de métadonnées
pour une ancienne base. Tout passage vers une future écriture refusera leur absence
avant effets. Leur établissement sous exclusivité réelle et journal durable
appartient à une tranche distincte, avant tout transfert ou apply.

## Primitives relues et décisions d'API

| Besoin | Primitive existante et décision |
|---|---|
| Rendu et identité de version | derive_nodes::{render_context, render_fields, render_inputs_hash, derived_content_uuid}, EntityConfig ; réutiliser les fonctions, aucune formule parallèle de Scope. |
| Découpe et identité de chunk | Chunker::chunk, ChunkerConfig et uuid::chunk_uuid ; les conventions de record_nodes::ChunkNode portent champs, offsets et liens. Si une extraction commune est nécessaire, proposer son diff avant de modifier record_nodes.rs partagé. |
| Modèles et stockage | EmbeddingModelEntry, StorageKind, VectorStorage::resolve : nom complet+dimension qualifient le modèle ; le slug ne suffit pas (collisions possibles). Résoudre séparément source et cible, legacy ou suffixed. |
| Réutilisation actuelle | catalog/aside::KnownVectors : un slug et map par ancien UUID. Ne pas élargir silencieusement sa sémantique ni injecter dans EmbedNode pour obtenir un prétendu transfert sans calcul. |
| Porteur de valeurs | records::{EntityRecord, RecordVectors, DenseVector} : une seule valeur dense par RecordVectors actuellement. L'aperçu porte une liaison par modèle ; ne pas lui faire croire qu'un EntityRecord contient tous les modèles. Adaptateur multi-modèle d'écriture à fournir ultérieurement. |
| Recherche/occurrences | Socle1a et catalog::resolve_to_source_entities ; conserver les liens canoniques et la projection avant réduction des résultats. Ne pas recopier cette politique dans le transport. |

Surface proposée dans `src/dataflow/migrations/transfer.rs`, enfant du runner
existant, avec accès `pub(crate)` initial :

```rust
// Esquisse : noms et signatures à arrêter en revue, aucun code livré ici.
fn preview_known_transfer(
    fixed_plan: &ResolvedMigrationPlan,
    inventory: &TransferInventory,
    target: &TransferRecipe,
) -> Result<KnownTransferPreview, Vec<TransferRefusal>>;
```

TransferInventory décrit des faits fournis, sans prétendre les avoir lus d'une
base. TransferRecipe compose les EntityConfig/faits résolus réels ; les fonctions
communes de rendu/UUID/découpe produisent les destinations. KnownTransferPreview
a des champs privés, accesseurs de rapport et correspondances déterministes ;
**aucune méthode execute/apply et aucune conversion en capacité d'écriture**.
Il référence l'identité du plan fixe et les empreintes des inventaires, sans
ajouter d'identité exécutable au journal. Les refus sont typés et localisés
(cellule, entité/chunk, modèle, motif), triés pour un rapport reproductible.

Le coût de cette primitive est la conservation des métadonnées de couverture et
la comparaison des vecteurs candidats. Elle unifie les changements de parent
indexé pour toute entité déclarée ; aucune branche spéciale Scope. La fixture
reste petite et entièrement en mémoire. Streaming, limites de volume des bases
réelles et lecteur RO cohérent sont des prérequis de la tranche de transport,
pas des promesses de cet aperçu.

## Inventaire requis, sans inventer de données

- Origine : identité d'instance et génération explicitement présentes ou absentes,
  descripteurs source/cible, empreinte des configs/faits/recette, identité du plan.
  L'empreinte de l'inventaire couvre ses entrées, pas une génération durable.
- Par cellule : occurrences avec UUID, valeurs rendues, provenance et propriétés
  d'occurrence (fichier, positions), états des liens, masques/visibilité déclarés.
  Les coordonnées d'occurrence restent sur l'occurrence ; les offsets de chunks
  sont relatifs aux champs rendus de leur version. Même contenu dans deux cellules
  ne donne ni liens ni magasin de vecteurs partagés entre elles.
- Relations : inventaire canonique des extrémités, types, propriétés et
  multiplicités, avec sens ; conserver les deux représentations attendues quand
  le moteur les matérialise. Arêtes du code inchangées ; seuls liens déclarés
  version→occurrence et chunk→version sont ajoutés à l'aperçu cible.
- Chunks : parent/champ/rôle/indice, texte et titre exacts, offsets complets/core,
  lignes et content_offset. Vérifier bornes UTF-8, cohérence avec les champs,
  couverture et unicité ; pas seulement l'égalité des comptes ou des hashes.
- Dense : registre complet des modèles, nom/dimension/stockage source/cible,
  colonnes et marqueurs résolus, valeurs brutes par chunk et par modèle,
  disponibilité/dettes. Dimension exacte, valeurs finies ; aucun vecteur absent,
  partiel, périmé ou contradictoire n'est remplacé par un calcul.
- Entrée du modèle : octets effectivement désignés par le chemin d'embarquement
  qualifié, pas une chaîne reconstruite au jugé. EmbedNode utilise text_field
  (`_text` par défaut), pas automatiquement `_title`. Le titre reste à comparer
  comme donnée/recette ; si un chemin a incorporé le titre ou une autre entrée,
  prouver cette recette et cette entrée. Marqueur `_text_hash` seul ne prouve pas
  une autre entrée historique. Source ambiguë : refus InputProvenanceUnavailable.
- Sparse : croiser signaux persistés, colonnes/marqueurs, handles/magasins et
  dette. Tout sparse/dual à transférer ou contradiction est refusé avant effets.
  Une déclaration models.sparse servant seulement au surlignage ne suffit pas à
  classer le profil sparse ; ne pas faire un refus sur ce seul champ.

## Correspondances et invariants

1. Regrouper les occurrences par **identité1a calculée**, dans chaque cellule,
   sur tous les rendus/rôles et la recette ; ne pas dédupliquer au seul corps.
   Garder huit UUID d'occurrence dans la fixture canonique, six versions attendues.
2. Rejouer uniquement le rendu et la découpe déterministes. Construire les nouveaux
   UUID de chunks avec chunk_uuid(nouveau_parent, champ, indice). Une liaison
   source→destination exige mêmes octets d'entrée modèle, recette, champ/rôle,
   offsets/indice, modèle complet et dimension ; pas une copie de map UUID.
3. Plusieurs sources compatibles peuvent couvrir une destination, mais leurs
   valeurs pour un modèle doivent être identiques **bit à bit** (`f32::to_bits`),
   sans tolérance numérique ni « premier trouvé ». Même slug avec noms différents,
   doublon divergent, marqueur périmé ou absence de couverture : refus nommé.
   Le représentant de rapport est choisi par ordre canonique, tous les témoins
   de couverture sont conservés. L'ordre d'entrée n'influence pas le résultat.
4. Garder une liaison distincte par modèle. Les dimensions4 et6 ne sont jamais
   mélangées ; les valeurs et marqueurs ne deviennent disponibles qu'après future
   écriture et validation réelles, pas parce que l'aperçu les énumère.
5. Texte BM25 exact préservé. Les offsets physiques et structures FTS/HNSW ne se
   copient pas ; leur reconstruction future doit utiliser texte/vecteurs connus.
   La déduplication change le corpus BM25 : scores/rangs globaux identiques ne sont
   pas un invariant ; couverture, filtres, provenance, positions et égalités le sont.
6. Aucun état persistant ni index déclaré prêt ne sort de cette tranche. Les
   lectures masquées doivent rester une politique commune ; l'inventaire/aperçu
   conserve les occurrences et masques nécessaires sans prétendre exercer T7.

## Témoins proposés avant correction et budget

Fixture canonique du plan1b : Alpha A/B identiques, C corps différent, D docstring
différente, E nom différent, F sans docstring ; Beta deux rendus identiques à A.
Huit occurrences/six versions, graphes de code avec propriétés et multiplicités,
deux modèles synthétiques4/6 ; vecteurs littéraux, aucun serveur ni modèle.
Un deuxième nom d'entité déclaré autre que Scope vérifie la généricité.

| Cas | Verdict à exiger |
|---|---|
| Nominal, permutations, deux cellules | Mêmes aperçus/liaisons/empreintes, huit occurrences/six versions, aucun lien cross-cellule, octets/positions/provenance préservés. |
| Texte, titre, rôle, recette ou entrée modèle distincte | Classes distinctes ou refus de compatibilité, jamais réutilisation par seul hash de corps. |
| Deux modèles, legacy/suffixed, collision de slug | Résolution source/cible correcte, couverture indépendante ; collision de noms refusée. |
| Vecteur absent/dimension fausse/NaN/infini/marqueur périmé | Refus localisé, aucun succès partiel ni disponibilité inventée. |
| Deux sources d'une destination, valeurs divergentes | Refus bit à bit ; égalité exacte acceptée, ordre d'entrée sans effet. |
| Identité source ou génération absente/étrangère | Refus préalable à toute future opération ; Unversioned reste explicite. |
| Sparse/dual/dette/contradiction ; lecteur de surlignage seul | Refus du transfert sparse ; le lecteur seul n'est pas un faux positif. |
| Masques, relations et couverture manquante | Conservation exacte de l'inventaire valide ; incohérence nommée, pas de remplacement par un compte global. |

Proposition de premier diff : module pur `migrations/transfer.rs`, déclaration
`mod transfer` dans migrations.rs et tests unitaires voisins. Aucun backend.rs,
CLI, gabarit, stockage, runner supplémentaire ou intégration de service. Les
factories d'aperçu ne reçoivent ni DbConnection ni Embedder ; ce fait limite
structurellement les effets, mais ne remplace pas les futurs témoins d'exécution.

Si les conventions de ChunkNode ne sont pas réutilisables sans extraction,
signaler d'abord le petit contrat commun à orchestration ; ne pas réserver ni
modifier record_nodes.rs en parallèle d'ED sans accord sur le propriétaire.
Préparation actuelle : zéro lourd. Après revue de périmètre, proposition d'un
seul ciblé sur luciepc, target unique par session plafonné32Go, j8/Mem32, borne600s
après poste lourd ; jeu minimal à choisir d'après les dépendances effectives.
Premier rouge : diagnostic et arrêt, aucun rejeu inchangé. Passe complète et
budget d'intégration à convenir après le diff et les compteurs réellement lus.

La tranche suivante devra fournir inventaire RO lié au stockage sous exclusivité,
identité/génération durables, journal running réel, adaptateur multi-modèle,
écritures/reconstruction/checkpoints/validation et publication atomique avant
apply. L'aperçu pur ne permet de sauter aucun de ces prérequis.
