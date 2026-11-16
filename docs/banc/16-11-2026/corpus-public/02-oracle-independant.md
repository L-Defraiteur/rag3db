# Attentes indépendantes pour six langues, avant résultats

Ticket coordinateur e843a7a9ba et ticket CLI c787 ; contrat c96.
Complète l'échantillon C#/Go de C7 (09bb6acd3f) sans le remplacer.
Aucun résultat du parseur n'a été consulté pour préparer cet échantillon.

`oracle-six-langues.json` fixe12sources sélectionnées, deux par langue,
109faits sourcés, leurs commits/SHA et ancrages ligne/colonne/octet. SHA256 :
`38f7cf22ffa11042ccca0d3238071a46765040a70cb8a406bff6d39cb0d2d31b`.
Manifeste de sélection :96c2e771fefc4cd0e204975feb8d1c73b1a66815cb1d3a1edc1b9002b94b644c.
Le JSON est le détail normatif des attentes, pas une sortie du parseur.

| Langue | Sources | Constructions vérifiables |
| --- | --- | --- |
| Rust | tokio-stream/src/empty.rs ; tokio-util/src/io/read_arc.rs | struct tuple générique, impl de traits et méthodes, associated type, async générique, macros |
| Python | django/template/loaders/filesystem.py ; django/contrib/postgres/aggregates/statistics.py | imports relatifs aliasés, héritage, méthodes et générateur, deux __init__ de propriétaires distincts, attributs de classe |
| C | crypto/rand/rand_meth.c ; lib/curl_memrchr.c | fonctions static, retour pointeur, table de callbacks, includes, compilation conditionnelle et macro |
| C++ | src/core/lib/resource_tracker/resource_tracker.h ; include/grpc/event_engine/internal/slice_cast.h | classe et méthodes déclarées, destructeur virtuel, templates, spécialisation partielle et trois surcharges distinctes |
| JavaScript | lib/internal/crypto/certificate.js ; scripts/webpack/copyright-banner-plugin.js | fonctions et constructeur historique, prototype distinct d'une classe ES, classe héritée/constructor, CommonJS |
| TypeScript | src/vs/editor/common/core/2d/point.ts ; packages/components/message/src/instance.ts | classe/méthodes/static, paramètres-propriétés readonly, fonction typée/closure, type alias générique, imports type et fonctions fléchées |

Chaque nom est ancré dans la vraie source, pas dans un commentaire mentionnant
ce nom. Les octets UTF8/BOM/CRLF ne sont pas normalisés. Les spans produits
peuvent commencer avant l'ancrage avec attributs/commentaires ; ils doivent
contenir l'identifiant. Une méthode représentée par membre ou scope ne doit pas
être comptée deux fois. Les source_kind sont des catégories sémantiques de
l'oracle, pas une obligation de reprendre les chaînes exactes de l'API.

Définitions/déclarations d'une part, faits conditionnés à leur exposition par
l'API d'autre part : imports, macros, héritage, liens et closures ne deviennent
pas des faux manquants si cette dimension n'est pas exposée ; cette limite se
dit explicitement. Les jugements après campagne seront observed/missing/
not_exposed_by_api/indeterminate, avec preuve compacte et motif. Une définition
non extraite malgré un AST correct et un contrat d'extraction correspondant
exige un ticket fils avant tout correctif. Une erreur AST garde ses diagnostics.

Échantillon orienté diversité syntaxique, pas un sondage statistique : aucun
rappel global,100% de couverture ou résolution inter-fichiers n'est prouvé.
Aucun code tiers/build/modèle/service/DB exécuté par la préparation.
