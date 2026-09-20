# Démons : découverte sémantique puis filtre de type

Demande utilisateur : nouveau deck autour des démons. Résultat : **Le notaire des enfers**, Historic BO1 noir/rouge, 60 cartes dont 24 terrains et 10 démons. Aucun craft selon le snapshot.

## Recherche effective

Requêtes et réponses MCP dans `experiments/mtga/data/deck-research-2026-09-20/demons/` :

1. `requests.json` : recherche hybride BM25/vector d'effets de démons + noms candidats.
2. `requests2.json` : supports de défausse, sacrifice et réanimation ; découverte des quatre Rune-Scarred Demon et quatre Faithless Looting possédés.
3. `requests3.json` : sélection `is_primary=true` avec `type_en contains Demon`, plus sélection du pool de supports et terrains.

Le filtre de type a révélé des options absentes du top-k sémantique, notamment Vile Mutilator et Abyssal Harvester, retenues dans le deck. `type_en` est une chaîne et non un tableau de sous-types ; ce filtre est un contains sensible à la représentation, adapté ici aux types anglais canoniques du snapshot. Il ne doit pas être présenté comme une relation de sous-type déjà normalisée.

Le moteur a servi à toutes les recherches : SDK MCP → backend généré → rag3db/lucivy/BGE-M3 local. Le script d'assemblage lit les réponses MCP sauvegardées, pas le catalogue brut. Le dense n'est pas utilisé comme garantie d'exhaustivité.

## Construction et pièges évités

- Cimetière préparé par défausse ; pas de moteur d'auto-meule.
- Victimize exige deux cibles déjà présentes au lancement, et un sacrifice à la résolution.
- Vile Mutilator fournit un effet d'arrivée sans coût de sacrifice supplémentaire lorsqu'il est réanimé.
- Les démons dont le gros effet exige d'avoir été lancés, tels que Bringer of the Last Gift et Doomsday Excruciator, n'ont pas été choisis comme cibles prioritaires de réanimation.
- Rune-Scarred Demon trouve des réponses ou le finisseur Bloodletter + Rush of Dread ; plusieurs cartes singleton sont accessibles par quatre tuteurs créatures.
- Les jetons de Harvester ne s'accumulent pas et la carte copiée est exilée. Vilis peut faire piocher trop de cartes et la vie reste un coût réel.

## Livrables

`experiments/mtga/data/deck-drafts/experiments-2026-09-20/04-le-notaire-des-enfers/` contient l'import Arena, le JSON avec identifiants/textes/provenance, la validation et le guide de jeu.

Reproducteur : `experiments/mtga/scripts/build_demons_engine.py`.

Vérifié : 60 cartes, 24 terrains, 17 créatures dont 10 démons, quantités possédées par impression, maximum quatre non-basiques par nom, terrains de base débloqués, export de 60 cartes, snapshot unique. L'import et les résultats de parties ne sont pas encore mesurés. Pas de changement du moteur nécessaire.
