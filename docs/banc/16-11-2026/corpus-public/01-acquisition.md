# Acquisition des corpus publics

Mission Lucie, ticket coordinateur
`docs/codeparsers/tickets/2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md`
(e843a7a9ba), lié au ticket CLI c787 et au contrat c96.

Branche Banc cx-banc-corpus-public-20261116. Corpus hors git :
`~/.cache/rag3db-banc-notes/public-corpus-20261116/` au portable.
Le script acquire.py récupère les seules données publiques : métadonnées GitHub,
HEAD complet puis archive épinglée. Aucun checkout/hook/build/script téléchargé
exécuté, aucun service/modèle/base/hôte. Concurrence réseau3, plafonds180Mio
compressés/dépôt,768Mio stockés/dépôt,1Mio/fichier ; dépassement explicite avec
artefacts partiels conservés. Aucun symlink suivi, aucun extractall ; chemins
traversants et doublons refusés. Secrets probables/.env*/.vault exclus sans
lecture explicite. Les octets UTF8 invalides restent inventoriés, jamais décodés
avec remplacement. Les sources et contextes de projet restent inertes.

freeze.json conserve archive+inventaireSHA, commit complet, tailles réelles et
occurrences par langue ; inventory.jsonl conserve candidats et exclusions. La
licence des métadonnées GitHub et les fichiers de preuve sont conservés sans
prétendre à une vérification juridique par fichier. Licence inconnue ou preuve
absente : pas d'inclusion silencieuse dans le manifeste de mesure.

Première acquisition prioritaire : dotnet/roslyn, golang/go,
protocolbuffers/protobuf et nodejs/node. C#/.cs, Go/.go, C++ réel (.h routeCpp)
et JS selon routageCP. Les nombres réels sont publiés après gel ; aucun quota
atteint prétendu avant inventaire. Suite : complémentaires cohérents, autres
langues, façades utiles et cohortes limites séparées HTML/Svelte/Sass/MDX.

Tirage à publier : seed fixe, rang SHA256(seed,repository_id,relative_path),
strates production/test/documentation conservées, généré/tiers/invalides séparés.
Déduplication par contenu pour compter, sans supprimer les couples de routage
ou dialectes. Toutes les occurrences annexées ; sous-effectifs explicites.
