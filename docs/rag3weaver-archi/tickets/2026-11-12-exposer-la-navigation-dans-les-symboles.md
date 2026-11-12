# Exposer la navigation dans les symboles

- **État** : ouvert ; conception autorisée par Lucie le 12 novembre, interfaces proposées et couverture actuelle à qualifier avant code.
- **Famille** : produit
- **Gravité** : capacité manquante ; l'agent doit pouvoir explorer des arbres de code sans lire tous les corps.
- **Atteignable en service** : non pour les nouvelles façades ; outline et les relations de parents existent déjà.
- **Touche rag3weaver** : oui, outils déclaratifs du code et de l'atelier, références de symboles, rendu et enrichissement du grep.
- **Pour** : ED et archi pour les contrats/outils ; codeparsers pour la couverture des membres et relations.
- **Signalé par** : Lucie, demandes list_methods puis list_members, parents/enfants bornés, list_symbols(file) en arbre et option symbol_hierarchy_depth du grep.

## Demande et conception

[Vision](../../orchestration/visions/2026-11-12-explorer-les-symboles-et-leur-hierarchie.md).
Composer outline/Plans/parcours existants, plutôt qu'un deuxième moteur par client.
Ne pas confondre contenance, appartenance déclarée et héritage ; une référence doit
être qualifiée dans sa vue. Décrire les limites de couverture et d'index au lieu
de rendre une absence trompeuse. Grep : profondeur supplémentaire0 par défaut,
enrichissement optionnel avec provenance, sans réindexation implicite.

## Fin attendue

Contrat commun et première tranche reviewables, puis autorisation et validation
du vrai parcours agent : lister l'arbre d'un fichier, choisir un membre, parcourir
ses parents/enfants et lire seulement ce qui est utile. Témoins de profondeur,
ambiguïté, membres partiels, fichier modifié/index périmé et troncature. La note
de conception ne ferme pas ce ticket et ne déclare aucune nouvelle API livrée.

## Première tranche autorisée pour la nuit du12au13novembre

Lucie demande des lots parallèles utiles depuis les visions. Archi prend la
première tranche de navigation réellement accessible, après qualification de
la couverture ; codeparsers prend l'inventaire IR/fixtures en soutien, sans
pointeur parent ni ancienne administration à modifier.
[Répartition](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).

Réutiliser outline/Plans/parcours ; ne pas ajouter un alias sans capacité utile.
Arbre de fichier puis membres/parents/enfants qualifiés, profondeur et couverture
explicites, sans confondre héritage et contenance. Noms et découpage sont des
choix d'implémentation ; les vrais changements de conception sont numérotés sur
la page existante. Archi garde la navigation/gabarits ; ED garde le parcours
agent et annonce toute édition nécessaire d'un fichier partagé. Publier diff
et témoins avant lourd, puis validation du vrai accès commun aux agents.

## Revue de la première source c999774c5 — nuit du12novembre

Relecture statique de `src/symbol_navigation.rs` par orchestration ; la source
testée en cours reste figée. Aucun build ou ciblé supplémentaire lancé par cette
revue, aucune réussite du runtime encore revendiquée. Archi garde la correction
et les témoins dans son lot. Deux écarts sont à traiter avant avis favorable :

- Le plafond96Kio n'est appliqué qu'à la sortie finale (lignes287–296).
  `ambiguous_symbol` retourne déjà aux lignes196–208, avec jusqu'à200 candidats
  incluant signatures512caractères, noms et références. Ce chemin peut dépasser
  le plafond annoncé ; `ambiguous_source` et les autres retours anticipés ne
  passent pas non plus par la réduction commune. Qualifier les sorties d'erreur
  et d'ambiguïté avec le même contrat de budget, sans cacher leur statut ni
  inventer une liste exhaustive. Le budget vise le résultat effectivement rendu.
- Les égalités/plages ambiguës sont signalées dans `coverage.ambiguous_ranges`
  (ligne186), mais le rendu Markdown par défaut ne lit pas ce champ. Un parcours
  parents sur deux scopes de même plage rend `ready`, le seul symbole puis
  l'avertissement générique de couverture ; le demandeur ne voit pas que son
  parent est indéterminable. Rendre cette ambiguïté explicitement aussi en mode
  compact et conserver le choix non arbitraire. Même exigence pour les faits de
  membres malformés et les limites spécifiques, sans devoir deviner qu'il faut
  relancer en JSON pour découvrir une limite de ce résultat.

Point de coût à qualifier avant d'appeler l'outil borné et rapide : la recherche
des parents minimaux (lignes158–170) contient trois boucles. Sur une chaîne de
plages imbriquées, chaque candidat recherche parmi les autres candidats ; le
nombre d'appels `contains` croît au cube, malgré le plafond2000scopes. Cette revue
ne constitue pas une mesure de latence. Prévoir un témoin de travail/complexité
borné ou réduire l'algorithme ; ne pas lancer un gros benchmark pour prouver ce
que le comptage d'opérations peut établir.

Codeparsers a aussi transmis directement à Archi la limite du mode members :
des méthodes Python/Rust présentes comme scopes peuvent manquer quand les seuls
champs `declarations`/`field_types` sont vides. Composer les faits disponibles ou
nommer cette limite précise ; ne pas annoncer une absence de méthode comme un
inventaire complet. Qualification IR livrée dans
[navigation-ir/qualification.md](../../codeparsers/12-11-2026/navigation-ir/qualification.md),
master a4bf86c865. Le ticket reste ouvert pour la façade et ses autres critères.
