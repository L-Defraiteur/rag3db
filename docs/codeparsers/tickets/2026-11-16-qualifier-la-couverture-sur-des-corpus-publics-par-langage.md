# Qualifier la couverture sur des corpus publics par langage

- **État** : ouvert — mission explicite de Lucie transmise par orchestration ; protocole et réservations à publier avant acquisition et mesure
- **Famille** : produit
- **Gravité** : preuve de couverture ; un parsing sans erreur ne démontre pas des captures ou relations complètes
- **Atteignable en service** : non, banc isolé ; les défauts de parseur révélés seront qualifiés séparément
- **Touche rag3weaver** : oui, faits d'index de code ; première entrée limitée au producteur et à ses fixtures de corpus
- **Pour** : codeparsers pour CLI/protocole/qualification ; Banc pour acquisition, orchestration pour sélection et coordination
- **Signalé par** : Lucie via orchestration le 16 novembre, corpus publics pour chaque langage supporté, environ 2–3 mille fichiers source par langage

## Périmètre autorisé

Téléchargements de dépôts publics autorisés, sans exécuter leurs scripts de
build, tests, installations ou programmes. Ni secrets, services, base de Lucie,
modèles ou reprise d'ancien GO. Commits/licences/hash et sélection déterministe
à publier ; langages et feature flags réels à relire avant de choisir les corpus.
C7 prend sélection et documentation ; Banc prend acquisition en parallèle.
Aucun défaut masqué par le statut parseOK, aucun claim d'exhaustivité.

## Travail attendu et preuve de fin

1. Inventaire des langages réellement reconnus, versions parser/grammaires,
   extensions, options et limites ; distinguer reconnaissance et extraction.
2. Protocole/CLI pour chaque fichier : extension non prise en charge, zéro scope,
   erreurs AST/parseur, captures absentes/non jugées, scopes/symboles/relations,
   temps et mémoire/pic avec provenance de mesure. Définir ce qui n'est pas mesuré.
3. Corpus figés et hashés, sélection triée déterministe d'environ 2000–3000
   fichiers pertinents/langage, manifeste source/licence/exclusions, pas de code
   téléchargé exécuté. Sous-effectifs explicites, pas duplication pour gonfler.
4. Source et recette publiées, créneau et budget frais coordonnés avant charge.
   Rouges conservés, premier échec inattendu diagnostiqué, suites non jouées
   explicites. Rapport séparé par langage et cas moins usuels, défauts révélés
   rattachés à leurs tickets. Parsing réussi n'est pas un oracle de couverture.

La qualification du correctif Rust7601 et ce banc sont distincts ; aucune
nouvelle campagne ne remplace ses validations restantes. Le présent ticket
n'autorise pas de nouveau Cargo ou mesure sans son profil et son créneau.

## Inventaire statique initial

Voir `docs/codeparsers/16-11-2026/language-corpus/01-inventaire-et-protocole.md` :
huit langages dans le routeur, API spécialisées distinctes. HTML est un stub
explicite ; Svelte initialise un parser sans langage puis appelle `parse().expect()`.
Ces limites restent ouvertes dans ce ticket, sans exécution ni correction annoncée.
Le protocole d'entrée est proposé à Banc avant implémentation de la CLI réservée.
