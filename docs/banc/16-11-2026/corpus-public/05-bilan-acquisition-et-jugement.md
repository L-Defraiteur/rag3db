# Corpus publics : acquisition et jugement indépendant terminés

Les données et le jugement Banc sont livrés ; aucune correction de parseur n'est
incluse. Mission Lucie, ticket coordinateur e843a7a9ba et ticket CLI c787, contrat
c96. Les tickets de défauts restent ouverts pour des lots distincts. Le comptage
n'est pas une preuve de rappel sémantique ; aucune résolution par projet n'a joué.

## Sources et campagne effectivement mesurées

24dépôts publics figés,633946116octets d'archives et1888331829octets stockés,
32382fichiers sélectionnés. Manifestes, licences/périmètres, exclusions et
sous-effectifs sont publiés dans les fichiers adjacents. Tous les corpus sont
inertes : aucun build, hook, programme ni installation externe exécuté.

Le manifeste02 est inchangé :
`96c2e771fefc4cd0e204975feb8d1c73b1a66815cb1d3a1edc1b9002b94b644c`.
Miroir PC/portable strictement vérifié, reçu commun8894a37a...8436a7 ; pas de.git
fictif. Racines98766ce8...495c6. Les gels01/02 et toutes les exclusions sont
préservés, pas de redistribution de quotas après lecture des sorties.

Campagne Codeparsers02b, ELF
`bff1cafb72e8c6662d2d595457b0ca6da4718686ae64b91ae1bde7a4e95565e2`.
END confirmé :32382/32382, commonrc0, scopevalidation-5r_yy729VERT/code0,
restes[], privé nettoyé et autorité terminée. Le superviseur vert prouve une
exécution bornée et nettoyée, **pas un parsing global vert**.

Relecture indépendante Banc en flux des résultats complets, sans reparse :
identité et ordre32382/32382 égaux au manifeste, SHA d'entrée inchangés, ELF
applicable conforme, dénominateurs recomptés. RésultatsJSONL SHA256
`0667eab18d7892b442b8b912d98528d1253041620f43c60a286b47cd5fa81678`.
Les six sorties d'exécution en échec n'ont pas de provenance complète par worker :
leur identité reste liée aux input/sequence et au reçu ELF de campagne, sans
inventer une métrologie ou un AST absent.

## Résultats bruts, dimensions séparées

| Langue/façade | Fichiers | Exécution terminée | AST valide | AST invalide | Sortie worker / délai | Captures vides |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| C | 2500 | 2499 | 2408 | 91 | 1 sortie | 1 |
| C++ | 2500 | 2500 | 2494 | 6 | 0 | 0 |
| C# | 2500 | 2499 | 2495 | 4 | 1 délai | 0 |
| Go | 2500 | 2500 | 2499 | 1 | 0 | 0 |
| JavaScript | 2500 | 2500 | 2500 | 0 | 0 | 1 |
| Python | 2500 | 2500 | 2500 | 0 | 0 | 57 |
| Rust | 2441 | 2440 | 2437 | 3 | 1 délai | 16 |
| TypeScript | 2500 | 2497 | 2497 | 0 | 3 délais | 1 |
| CSS | 1041 | 1041 | non exposé | non exposé | 0 | 16 |
| SCSS | 1661 | 1661 | non exposé | non exposé | 0 | 54 |
| Vue | 2500 | 2500 | non exposé | non exposé | 0 | 0 |
| Markdown | 2500 | 2500 | non exposé | non exposé | 0 | 107 |
| Sass, dialecte | 267 | 267 | non exposé | non exposé | 0 | 51 |
| MDX, dialecte | 211 | 211 | non exposé | non exposé | 0 | 3 |
| HTML, absence déclarée | 1761 | 0, non tenté | non tenté | non tenté | 0 | non jugé |
| Svelte, limite déclarée | 2500 | 0, non tenté | non tenté | non tenté | 0 | non jugé |

Total :28115completed,1worker_exit,5timeout,4261not_attempted ;105ASTinvalides.
Completed signifie retour du worker, pas validité AST ni qualité d'extraction.
Les captures vides peuvent être légitimes ; aucun défaut n'est déduit du seul
zéro sans attente source. CSS/SCSS/Vue/Markdown n'exposent pas la validitéAST
par cette interface ; leur retour ne la prouve pas. HTML/Svelte ne sont pas
comptés comme tentatives réussies. Les statuts `supported` de Sass/MDX reflètent
la route utilisée, pas une couverture du dialecte démontrée.

Les six incidents sont des données conservées, sans retry de fichier :
OpenSSL ssl/s3_lib.c (worker_exit -6), Roslyn T_1247520.cs, rust-analyzer
crates/hir/src/lib.rs et trois fichiers VSCode TS (claudeAgent.test.ts,
chatInputPart.ts, sessionsList.ts). Codeparsers porte les diagnostics et tickets
de ces incidents, sans doublon ici ; ni cause mémoire ni régression ne sont
inférées du seul délai sous les limites10s/fichier et2GiBAS du worker.

## Reclassification explicite des584générés

Annexe immuable8e673337...6a18a, jointe par repository_id/path/SHA. Aucun fichier
retiré de02, aucun changement ELF.329C et255Cpp ont un header upb_generator
explicite et ont été mal classés par l'heuristique initiale :

| Sous-ensemble | Fichiers | AST valide | AST invalide | Incidents |
| --- | ---: | ---: | ---: | ---: |
| C générés upb confirmés | 329 | 250 | 79 | 0 |
| C autres lignes | 2171 | 2158 | 12 | 1 |
| C++ générés upb confirmés | 255 | 255 | 0 | 0 |
| C++ autres lignes | 2245 | 2239 | 6 | 0 |

« Autres lignes » ne signifie pas toutes écrites à la main : classification
partielle précisément bornée. Le ticket Banc
`2026-11-16-les-sources-upb-gen-du-corpus-sont-mal-classees.md` reste ouvert
jusqu'au vrai témoin puis correction du classificateur et preuve d'usage des
bons dénominateurs ; cette annexe ne prétend pas avoir corrigé l'heuristique.

## Oracle source indépendant : douze fichiers, pas100% de couverture

Les109faits Rust/Py/C/Cpp/JS/TS ont été publiés avant sortie ; C7 porte séparément
ses attentes C#/Go. Les douze lignes de l'échantillon figées pendant la campagne
sont identiques aux douze lignes de ses résultats finaux : sampleSHA256
`3e3ee90db2111430e8bb9f6e27db6bccc92df673346574df7804a0375fc311ff`.
Jugements :80observed,2missing,1API non exposée,26unjudged (champs non conservés
par le compact ou qualificatifs non démontrés). Ce ne sont pas des fractions
comparables de rappel. Plages byte/ligne et propriétaires vérifiés, pas noms seuls.
Les membres sans bornes ne prouvent qu'une ligne. Les signatures/modifiers et
références individuelles non conservés ne deviennent pas des absences de l'IR.

Trois tickets fils avant tout correctif, avec exemples minimaux NON JOUÉS :

- C++ : `2026-11-16-un-template-forward-cpp-contamine-le-namespace.md` — attribut
  generic_parameters de namespace erroné malgré AST valide.
- Rust : `2026-11-16-un-parametre-generique-rust-garde-sa-contrainte-dans-le-nom.md`
  — GenericParameter.name pollué par sa contrainte.
- TypeScript : `2026-11-16-les-parametres-proprietes-typescript-manquent-aux-membres.md`
  — x/y sont des paramètres constructor, mais leurs membres de classe manquent.

Les2missing sont x/y de l'oracle ; les deux erreurs d'attributs sont des
observations supplémentaires après sortie, pas des attentes ajoutées
rétroactivement. Les fonctions/méthodes des douze sources sont bien identifiées
et localisées dans les dimensions jugées. Le trait de l'impl Rust n'est pas
conservé par ces records ; Class pour impl ne prouve pas une déclaration struct.
Le forward C++ sans body est intentionnellement hors des scopes de définition.
Les défauts C# déjà signalés par C7/CP ne sont pas dupliqués par Banc.

## Preuves et suites séparées

`final-independent-review.json` publie le recalcul Banc, SHA256
`5ecbaa0e9c1da8cc740e923ab3c10cbabe99278fd9d896110a84568777cb4499`.
Codeparsers garde l'unique miroir complet des preuvesPC→portable ; Banc n'a
rapatrié que le résumé et les douze lignes, pas une deuxième archive300Mo.
Audit indépendant C7 annoncé à son chemin local
`~/.cache/rag3weaver-build/c7-corpus-oracle-02b/audit.json`, SHA256
`53b6773df2c3128494565fddcd32167b1306fcd181f06e0a859cb10d56d48a76` :
identités/ordre concordants, aucun record compact omis ni borne byte hors fichier.
Ce contrôle ne remplace pas les attentes sémantiques.

Acquisition, miroir, annotations et jugement Banc terminés. Aucune correction
parser, aucun nouveau build tiers, modèle/GPU, DB vivante ou7970 dans ce lot.
Les tickets coordinateur/CLI et enfants restent sous leur porteur pour la suite,
sans fermeture de défaut par un simple comptage. Les seuils/corpus ne sont pas
modifiés pour améliorer les résultats. L'ancien oracle de migration reste
indépendant et NONMATCHED ; rien de ce banc ne l'a qualifié.
