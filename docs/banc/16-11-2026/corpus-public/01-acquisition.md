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

## Premier gel livré, puis lot complet local

Gel01 : quatre dépôts, dont2500C# et2500Go sélectionnés sans attendre les autres.
`selection-01/selected.jsonl` SHA256
`6ab2b309e0365974d8ee59d6d1c442e7b2d78963659bfd20f225d83a764053b2`.
Leurs archives n'ont pas de.git ; le contratCP accepte freeze.json, SHA archive,
SHA inventaire et contrôle bytes/fichiers, sans git fictif.

Gel local complet :24dépôts,633946116octets d'archives et1888331829octets
stockés ;32382fichiers sélectionnés dans selection-02. ManifesteSHA256
`96c2e771fefc4cd0e204975feb8d1c73b1a66815cb1d3a1edc1b9002b94b644c`.
Les pins, licences et empreintes détaillés sont dans repository-freezes.json ;
les dénominateurs, strates, dialectes et exclusions dans selection-02-summary.json.
Aucune analyse de parsing n'a encore joué côtéBanc.

| Cohorte | Fichiers sélectionnés |
| --- | ---: |
| C | 2500 |
| C++ | 2500 |
| C# | 2500 |
| Go | 2500 |
| JavaScript | 2500 |
| Python | 2500 |
| TypeScript | 2500 |
| Rust | 2441 |
| Vue | 2500 |
| Markdown | 2500 |
| CSS | 1041 |
| SCSS | 1661 |
| Sass, dialecte à qualifier | 267 |
| MDX, dialecte à qualifier | 211 |
| Svelte, limite d'initialisation | 2500 |
| HTML, stub déclaré | 1761 |

Les huit langues de code atteignent2–3k. Les sous-effectifs des façades restent
explicites ; pas de duplication pour gonfler. Vue inclut1321fichiers de
 documentation et540de test ; Svelte2498de test et2de production : ces
compositions ne sont pas une preuve de couverture des applications réelles.

184fichiers dépassent1MiB : dont16.cs,4.go,6.ts,12.js,3.c,5.cc,4.h,2.cpp,
1.scss ; tous restent nommés dans les inventaires, sans analyse cachée. Les
secrets/métadonnées exclus sont comptés1414, liens/spéciaux70, arbres de build
ou dépendances1859. Les octets invalides, générés et tiers sont détaillés à part.

La licence GitHubNOASSERTION ne bloque pas une lecture du vrai texte : NodeMIT
sur lib et sous-arbres de tests relus, ProtobufBSD3 sur src/google/protobuf,
curl selon SPDXcurl, VuetifyMIT limité à packages/vuetify. Périmètres et preuves
hashées dans license-audit.json. Le dépôt demandé foundation/foundation-sites
redirige aujourd'hui vers foundation/yeti avecFSL-1.1-MIT : gel conservé mais non
sélectionné, pas licenceMIT inventée. Les docsVuetify sans preuve de portée ne
sont pas incluses par défaut.

CopiePC gel01 autorisée par C7 : mêmes chemins pour garder freeze.root inchangé,
rsync2concurrents,24Mio/s chacun, ignore-existing, aucun delete ; vérifications
archive/inventaire/fichiers/roots obligatoires avant READY. Cette section ne
prétend pas la copie déjà vérifiée tant que son reçu n'est pas publié.

## Miroir complet prêt pour le pilote Codeparsers

Copie complète autorisée et achevée,24repos et selection-02, même racine absolue
aux deux postes. Les gels01/02 restent inchangés, aucune source externe exécutée.
Validation locale puis PC :24archives SHA,24inventaires SHA,32382fichiers
sélectionnés taille/SHA/UTF8 strict, preuves de licence présentes et racines
strictes. Les deux reçus sont identiques, SHA256
`8894a37a0626071c2206ce23629f4248807bd7eb952c5e064e8b23f5d68436a7`.
Le détail est publié dans mirror-02-verification.json.

Entrées réelles du pilotePC :

```text
/home/lucied/.cache/rag3db-banc-notes/public-corpus-20261116/selection-02/selected.jsonl
/home/lucied/.cache/rag3db-banc-notes/public-corpus-20261116/selection-02/roots.json
```

SHA roots : `98766ce80d550ec1f2ae1a75ba1f0394ac35b0101f4bf819855c0bb4bca495c6`.
SHA occurrences : `2e9a0b9ae7f52c7e27f3c38153ead694dbde33cf058eb582f59a48f0b421a580`.
Aux postes : reçus local-02-verification.json et mirror-02-verification.json,
archives et inventaires complets conservés ; pas de retrait ni nettoyage.

READY transmis à Codeparsers etC7 pour son pilote32382fichiers, sans rejouer
les5000premiersC#/Go de01. Il s'agit d'un **READY des données**, pas d'un verdict
de parsing ni d'un GO de charge. CLI/options/parser/ELF, budget et créneau sont
à Codeparsers/C7. Les tickets coordinateur etCLI restent ouverts jusqu'aux
mesures et à leur interprétation. L'ancien oracle de migration reste indépendant
et NONMATCHED ; cette acquisition ne l'a ni relancé ni qualifié.
