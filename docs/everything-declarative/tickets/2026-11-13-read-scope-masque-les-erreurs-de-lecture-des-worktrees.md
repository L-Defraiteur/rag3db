# read_scope masque les erreurs de lecture des worktrees

- **État** : ouvert ; constat statique, correctif produit et témoins à planifier.
- **Famille** : produit
- **Gravité** : réponse fausse ; absence ou unicité annoncée malgré une recherche incomplète.
- **Atteignable en service** : oui ; read_scope de l'atelier interroge les worktrees attachés via usages.
- **Touche rag3weaver** : oui, src/daemon_host/atelier_fichiers.rs, atelier_lire_scope.
- **Pour** : everything-declarative ; coordination C7 et archi pour la suite.
- **Signalé par** : ED, revue RO de la qualification physique ; ticket demandé explicitement par C7.
- **Ouvert le** : 13 novembre 2026.

## Constat vérifié et portée

Recherche préalable dans les tickets ouverts/fermés : aucun ticket existant
trouvé sur ce masquage. Le code est identique sur master b87688188 (bilan
f8cbd6a48) et sur la source physique figée
4e5ae8a324f50313b74f70772eb2e92ee41fad6f.

Dans atelier_lire_scope, src/daemon_host/atelier_fichiers.rs:407-418 :

```rust
for w in &worktrees {
    let args = json!({ "name": name, "path": rel_voulu.clone().unwrap_or_default(), "limit": 1, "format": "json" });
    let Ok(Ok(v)) = self.with_backend(&w.hosted, |b| b.call("usages", args)) else { continue };
    // Les définitions des seules réponses réussies sont accumulées.
}
// 0 résultat : « aucun scope » ; 1 résultat : sélection de ce résultat.
```

Une erreur de with_backend (ouverture/accès backend) ou une erreur de l'outil
usages est ignorée par continue. Le nombre de résultats restants ne prouve
ni l'absence du nom ni son unicité sur tous les worktrees recherchés.
L'appel outline après sélection propage déjà ses erreurs ; cela ne rétablit
pas la complétude des interrogations usages antérieures.

## Reproduction statique minimale

1. Un worktree W répond en erreur à usages(name). Aucune définition n'est
   ajoutée, la boucle finit et la branche 0 annonce « aucun scope » : une
   panne est représentée comme une absence.
2. Deux worktrees A/B : A rend une définition, B échoue. La branche 1 choisit
   A et peut lire son scope ; B pourrait contenir le même nom. L'unicité
   multi-worktree n'est pas établie, aucun choix silencieux n'est justifié.

Ces chemins découlent du contrôle de flux, sans exécution ni fixture modifiée.
Aucune recette Cypher ne reproduit seule ce défaut : la perte d'information
est dans l'agrégation Rust des réponses du backend et de l'outil.

## Frontière avec la qualification physique

Le constat a été relevé pendant le diagnostic du FAILED
`daemon_host::atelier_fichiers::tests::the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree`.
Ce témoin est vert dans le log ED source09303fe39 conservé. La qualification
physique n'a pas fourni la panic complète : ce ticket n'attribue PAS son
FAILED à ce masquage, au catalogue RO, au poison natif ou à un index vide.

Le helper attendre_l_index ne juge que l'absence d'indexation en cours, pas
la réussite de l'indexage ; c'est une limite du diagnostic disponible, pas
une preuve causale ni une autorisation de modifier ses assertions.

Liens de coordination :

- [Parent migration des ancres](../../arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md), qualification physique indépendante.
- [Parcours IDE qualifié](../../tickets/closed/2026-11-12-prouver-le-parcours-agent-de-code-complet-de-l-ide.md), preuve historique conservée ; ce ticket ne la rouvre pas.

## Correction attendue et critères de fermeture

- Conserver l'erreur d'origine des deux niveaux de Result, avec contexte du
  worktree/backend concerné, du nom demandé et de l'opération usages.
- Une recherche incomplète doit refuser explicitement ; ni « aucun scope »
  ni succès prétendant un résultat unique sur les autres seuls worktrees.
  Ne pas ajouter de retry, de fallback ou d'exclusion silencieuse.
- « Aucun scope » reste possible seulement après toutes les interrogations
  nécessaires réussies ; le résultat unique et l'ambiguïté conservent leurs
  contrats actuels quand la recherche est complète.
- Futurs témoins déterministes : panne d'ouverture backend ; panne usages ;
  un résultat dans A et panne de B ; véritable absence, résultat unique et
  ambiguïté après recherches complètes. Vérifier le contexte et l'erreur
  conservés, pas uniquement un message générique différent.
- Revue et qualification de la correction avant livraison, puis clôture
  avec source et preuves. Aucun correctif produit n'est réalisé ici.

## Autorisation de cette publication

Lot documentaire seul dans un worktree cx neuf. Aucun Cargo, runtime,
service, base, modèle ou ancien lot rouvert. Aucune intégration de code pendant
la qualification physique ; la suite produit doit être coordonnée par C7.

## Préparation du correctif isolé — aucun test exécuté

Suite produit explicitement autorisée par C7 après publication du ticket.
WT neuf cx-ed-read-scope-erreurs, branche homonyme, base master4f6f80ef4.
Un seul fichier Rust : atelier_fichiers.rs. Les deux niveaux de Result sont
propagés séparément avec nom demandé, opération usages, racine, identifiant
backend et manifeste ; erreur d'origine conservée en suffixe. Première erreur
refuse la recherche incomplète avant sélection/outline/lecture du fichier.
Pas de retry ni de choix fondé sur les seuls worktrees répondants.

Quatre témoins écrits, NON compilés/qualifiés :

- Un worktree inscrit avec graphe usages manquant : erreur réelle du chargement
  backend transmise par le vrai hôte, pas « aucun scope ».
- Un backend inscrit sans outil usages : ouverture réelle puis erreur réelle
  de l'appel b.call, contexte et cause conservés.
- Un dépôt A réellement attaché/indexé et son scope effectivement lu avant
  inscription de B sans usages. L'ordre A puis B est vérifié : la panne tardive
  interdit de déclarer le résultat accumulé unique.
- Deux vrais dépôts aux noms d'inscription distincts et indexés : ambiguïté
  conservée pour leur nom commun, véritable absence après requêtes complètes.
  Le témoin historique de lecture unique reste inchangé.

Aucun helper de production factice ni mock de sélection ; les cas utilisent
le gabarit atelier livré, les racines privées et les appels d'outil réels. Les
pannes portent sur des déclarations de fixture nouvelles, pas sur les fixtures
de la qualification physique. Aucun fournisseur de modèle requis par ces cas.

Recette envisagée à coordonner AVANT Cargo/lourd : source/argv figés et publiés,
luciepc seulement, lib29/Max32/HIGH0/j8/target de session neuf ; compilation
libtests native+code+daemon puis quatre témoins `read_scope_` et témoin historique
exact (comptes/inventaires à publier avant exécution). Profil d'indexation explicite
conforme au contrat, pas d'accès aux bases de Lucie ou aux services partagés.
Premier rouge arrêté/diagnostiqué, pas de retry inchangé. Aucun créneau acquis ni
build lancé ; revue du diff requise, pas d'intégration produit pendant la globale
physique. Ce ticket reste ouvert : un patch écrit n'est pas une correction prouvée.


## Préparation après landing physique

Patch repris sans modification sur base master b0a5177c, branche neuve
cx-ed-read-scope-qualification-r2, commit produit a919ad9d9. Range-diff
égal à 674f596c1 ; revues statiques Archi/C7 favorables.
[Recette de qualification isolée](../13-11-2026/04-recette-read-scope.md)
publiée avant GO lourd : cinq ciblés puis module inventorié et frontières
features, target privé neuf, aucun partage de ressources Arbre. Aucun
Cargo/test exécuté à ce stade ; ticket reste ouvert.

Pilotes concrets publiés dans read-scope/ : appelants hors WT, CWD crate
explicite pour Cargo/ELF, magasin SDK explicite, ledger et inventaires
avant juges, check_features canonique 8/8, verdict composé sans retrait.
Préparation seulement ; revue et GO C7 exact avant toute exécution lourde.

Revue C7 pilote6a2 : GO suspendu avant runtime ; corrections de préparation
publiées : UNKNOWN externes séparés des owned/scope/contexte, copie ELF hors
target et source embarquée avant tests, environnement par liste blanche,
surveillance active premier rouge/--nocapture. Aucun Cargo/runtime joué.
