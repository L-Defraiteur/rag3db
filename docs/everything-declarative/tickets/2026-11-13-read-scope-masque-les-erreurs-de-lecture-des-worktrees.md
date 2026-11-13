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

## RO Archi des pilotes exacts6a2d1c8 — avant GO lourd

Revue statique uniquement, aucun Cargo/test/runtime lancé. Produit674f reste
hors de ces réserves ; elles concernent lancer/driver/finaliser de la recette R2.
Le porteur reste ED, GO lourd C7 encore requis.

1. Arrêt premier rouge : driver.run ne surveille que délai/maps pendant la vie
   du processus, puis son rc à la fin. Un FAILED précoce du module à8threads
   laisse donc les autres témoins continuer, contrairement à la recette. Détecter
   l'échec pendant l'exécution et laisser le superviseur arrêter les descendants ;
   conserver la panic utile (show-output la diffère en fin de suite, nocapture
   ou instrumentation immédiate évite de la perdre). Le script canonique F8
   est déjà failfast : cette réserve vise la boucle des ELF, pas sa matrice.

2. Provenance SDK runtime : maps est aussi rempli par les deux inventaires --list,
   et assert maps ne distingue aucune phase. Un inventaire seul peut satisfaire
   cette attestation sans observation d'un juge/module. Les entrées n'ont ni
   starttime, ni phase, ni exe résolu/cgroup réel. Capturer ces champs et exiger
   une observation pendant cible-* ou module, sur le même ELF SHA et SDK29.
   Les inventaires restent des preuves distinctes, jamais des exécutions de juge.

3. Profil : env=dict(os.environ) conserve endpoints de services/modèles et autres
   paramètres personnels. BURN=cpu ne désactive pas un fournisseur distant hérité.
   La promesse « aucun service externe » n'est donc pas portée par la recette.
   Neutraliser les endpoints/API et rediriger explicitement les racines privées
   pertinentes avant Cargo/ELF ; ne pas afficher leurs anciennes valeurs.
   profil.json écrit actuellement tous les noms RAG3* : ce préfixe inclut aussi
   DSN/credentials éventuels et le jeton du reçu. Remplacer par une liste explicite
   de paramètres utiles non secrets ; le contenu hérité n'a pas été inspecté ici.

4. Plafond target : garder_le_target vérifie l'attribution à l'arbre, pas32Gio.
   Ni la boucle run ni le finaliseur ne contrôlent la taille du target pendant
   ou après compilation/F8. Ajouter le contrôle borné32Gio et sa preuve finale
   sans supprimer/vider le target pour faire passer la qualification.

Le refus conservateur UNKNOWN du finaliseur est explicitement documenté et
n'est pas présenté ici comme un défaut nouveau. La revue du produit reste
favorable sur la propagation des deux Result ; aucune causalité du FAILED
physique antérieur n'est établie par cette inspection des pilotes.
