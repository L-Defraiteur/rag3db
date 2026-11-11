# Deux occurrences identiques réembarquent le même contenu

- **État** : ouvert (11 novembre 2026), lot des versions de contenu 1a.
- **Famille** : produit
- **Gravité** : coût inutile, un contenu identique est embarqué une fois par occurrence.
- **Atteignable en service** : oui (deux fichiers ou branches portent le même scope).
- **Touche rag3weaver** : oui (déclaration du gabarit code, identité de contenu, ingestion/recherche).

- **Pour** : session arbre-principal, porteuse du lot 1a ; coordination recherche/ED pour les contrats communs, orchestration pour les créneaux et le tour de push.
- **Signalé par** : reprise arbre-principal demandée par Lucie via [la prise de contact orchestration](../../orchestration/11-11-2026/04-contacter-orchestration-codex.md) ; [contrats et lots](../11-11-2026/03-contrats-et-lots-des-vues.md).

## Preuve

Le témoin deux_occurrences_partagent_un_embarquement_et_gardent_leurs_positions,
suite e2e_scope_versions, branche cx-arbre-scope-version-2 (c4bce011c), est joué sur
luciepc avec la lib29a7bc52a : deux occurrences a.rs/b.rs, positions distinctes correctes,
mais le compteur reçoit deux fois le même texte de fonction au lieu d'une.
Journal hors target : ~/.cache/rag3weaver-build/cx-scope-version-etat.log ; résultat de
la suite : 2 réussis, 1 échoué. La projection générique corrigée ne partage pas à elle seule
le stockage ; aucun format de base vivante n'a été changé.

## Avancement générique du lot 1a

Checkpoints 7748fa53c et d63157d8c, poussés sur origin/cx-arbre-scope-version-3 : versions dérivées par
contenu, références indépendantes, L3 composé et compteur générique sensible 2→1. Les tests
ciblés prouvent aussi déplacement sans nouvel embarquement, retour à une ancienne version,
recette et modèles 4/6 distincts, rollback des origines/liens/états/FTS. Le schéma Scope et ses
gabarits ne sont pas basculés : son témoin initial reste rouge. Projection publique, masque avant
top-k, passe globale et migration restent ouverts ; ce ticket n'est donc pas fermé.

## Preuves du checkpoint générique

Worktree cx-wt-scope-version aux deux postes, lib 29a7bc52a, target unique
cx-target-contenu-derive sur luciepc. Diagnostic natif, sans passe globale :

- 3 unitaires derive_nodes et 7 config/schéma : cx-contenu-invariants.log.
- 8 e2e_entites_derivees : cx-contenu-derivees-vert-v4.log.
- Compteur sensible 2 au lieu de 1 sous mutation : cx-contenu-compteur-rouge.log ;
  restauration, déplacement/réutilisation et modèles 4/6 : cx-contenu-compteur-modeles.log.
- Retrait de L3 : perte du lien A ; restauration : racines, versions, hashes, file indépendante
  et FTS préservés après échec d'insertion ou de lien. cx-contenu-atomicite-rouge-v2.log,
  cx-contenu-derivees-vert-v2.log puis les suites suivantes.

Ces journaux sont dans ~/.cache/rag3weaver-build/ sur luciepc. Le témoin Scope initial reste
rouge et aucun format de base vivante n'a été changé. PostgreSQL et les autres features ne sont
pas encore rejoués sur cette implémentation.

## Association publique et première projection native

Depuis 5a068a404, une association unique EntityConfig.search_index désigne la dérivée qui sert
la cible publique. Le plan de projection est calculé depuis son from et DERIVED_FROM ; il ne
constitue pas une deuxième déclaration. Le masque passe par le filtre relationnel commun avec
les UUIDs des racines admises. La projection se fait dans finish_signal, avant fusion et pagination.
Les coordonnées ne sont réutilisées que si les champs de contenu existent sur les occurrences et
si leurs octets correspondent ; sinon la recherche refuse un remappage non déclaré.

Premier témoin natif index_derive_rend_les_occurrences_publiques_avant_pagination : 1 réussi,
0 échec, cx-projection-publique-premier.log sur luciepc. Il couvre dense/plein texte × trois modes,
deux occurrences distinctes pour une seule version et une page offset=1. Pas encore de preuve
sparse ni de saturation par candidats invisibles sur ce diff ; pas de passe globale. Scope et ses
gabarits restent inchangés. Le texte natif qui ne garantit pas ce masque est refusé explicitement.

## Renforcement en cours après 49caf64c6

- Projection neutralisée : 1 version au lieu de 2 occurrences, rouge produit dans
  cx-projection-publique-rouge.log. Restauration : les 9 cas dense/texte/creux × 3 modes et
  pagination passent leurs assertions, mais le lanceur refuse un résidu de cache de la fixture
  (cx-projection-publique-vert-v2.log, code 101). Ce lancement n'est pas un vert global.
  Ordre de destruction corrigé : Catalog et ses handles avant le TempDir. Rejeu ensuite accepté
  par le lanceur, 13/0, cx-projection-masques-vert.log, sans résidu.
- Le masque suit maintenant la relation DERIVED_FROM canonique pour calculer les UUIDs de versions,
  puis emploie les filtres d'index existants. Une recherche de relation par seuls noms d'entités
  serait ambiguë si d'autres liens existent ; un témoin ajoute précisément un AUTRE_LIEN trompeur.
- Rouge sans masque avant top-k : les trois signaux perdent le résultat visible (0 au lieu de 1),
  cx-projection-masque-rouge.log. Restauration : 13/0, incluant les 12 orphelins toujours indexés et mieux classés, budget 2, filtres A/B partageant
  une version, pour dense/creux/texte, ordre de pagination stable et refus d'une transformation d'octets sans remappage déclaré.

## Reprise de l'ingestion directe par lot

Rouge observé : ingest_entities écrit sa source par un graphe direct avant le drain des dérivées.
Un échec de lien laissait alors A.body=après, version=avant, B inchangée :
cx-projection-lot-reprise-rouge.log sur luciepc. Envelopper uniquement le second drain était insuffisant.
Le mécanisme L3 est maintenant composé par ecrire_un_lot_atomique pour les deux chemins,
drain et ingestion directe, avec mise à part des ressources indépendantes.

Vert ciblé : un_echec_du_rendu_en_lot_preserve_sources_et_references_apres_reouverture, 1/0,
cx-projection-lot-reprise-vert.log. Base privée sur fichier : échec injecté de lien, source/référence/hash
antérieurs conservés ; réouverture sans réenregistrer le schéma, mêmes valeurs ; nouvelle tentative
réussie qui change seulement A. La régression native large est en cours, pas encore un verdict global.

## Travail restant et responsabilités

La session arbre-principal garde ce même ticket pour la suite du lot, sans duplication :

1. Déclarer une seule association cible publique → index dérivé ; composer from/DERIVED_FROM,
   filter_indirection, Forme et réadmission dans le même L6. Recherche a relu ces primitives :
   ResolveParent après pagination ne suffit pas. Redéployer les occurrences avant pagination.
2. Appliquer le masque d'occurrences avant le top-k dense, creux et plein texte ; refuser un
   signal incapable de le garantir. Prouver absence de fuite, couverture et positions/arêtes.
3. Raccorder le schéma/gabarit Scope en test privé et rendre vert son compteur initial.
4. Passer les vérifications du lot sur son hash final, puis demander un tour à l'orchestration.
5. Préparer la migration déclarée 1b et son refus d'un ancien format, sur copies privées seulement,
   avec couverture, recherches, arêtes dans les deux sens et zéro embarquement.
6. Pour M3–M5, arbre-principal garde la recette et les résultats attendus ; coordination avec
   l'optimiseur pour copies et fenêtre exclusive, autorisée par l'orchestration. Aucune mesure
   lancée, aucun chiffre disponible. L'attente de créneau ne ferme pas ce travail.

Recevoir ce ticket ou relayer une demande n'autorise ni activation en service ni push master.
Les transferts éventuels déplacent ce fichier et ne ferment pas l'action.

## Ses filets

- generic_search_nodes.rs, BM25SearchNode : le texte natif qui ne garantit pas le masque
  d'occurrences refuse cette projection ; à lever uniquement avec sa preuve avant top-k.

- src/read_view.rs, Forme::resolve_search_target : refuse la recherche directe des versions
  internes tant que projection publique et masque ne sont pas déclarés. À réévaluer avec leur
  implémentation ; le refus ne constitue pas une livraison de recherche sur les occurrences.
- Forme::search_target_names ne propose pas ces versions internes comme cibles publiques.
- Catalog::verifier_ecriture_de_version et verifier_ecriture_de_reference préservent l'immuabilité
  et les rattachements gérés. Ces garanties doivent rester après résolution ; ce ne sont pas
  des replis temporaires à retirer avec le ticket.

## Pour le fermer

Mettre en œuvre le partage de contenu/chunks/vecteurs déclaré du lot1a, avec empreinte sur
l'entrée effectivement chunkée et version de découpage, modèle/dimension distingués. Conserver
les occurrences, positions, relations et visibilité. Prouver un seul embarquement pour le même
contenu et des contenus distincts pour docstrings/titres pertinents ; migration explicite1b sur
copies avant toute base partagée. Contrat : docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md.
