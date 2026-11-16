# Le témoin ne compile pas : aucun rouge binder observé

Même [ticket](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md), GO unique consommé `C7-ROWS-RED-DB833-16nov2400`.
Source5099a367ddf1e48db2b4065d91a61403de994571, pilote
db833d135cbceb3319424f1619f219e64bfd1fc0, recettea31745c5...ef44d,
bundle69688d86...42ba3 inchangés. **ROUGE de compilation, pas reproduction
de la colonne absente. Aucun test natif exécuté, aucun réessai.**

Vrai poste `run-p4110862-i150764589.scope`, début1193423.672627962,
cpu.max500000/100000, POSTE_FILS5, j5/CPUQuota500%,32G/swap0. Clone6,207s,
checkout0,815s, configure0,765s : code0, aucun résidu. La compilation arrive à
1011/1012 étapes puis refuse l'objet du témoin, code1. Le log complet et la
commande c++ exacte sont conservés ; ce n'est pas un timeout ou un refus mémoire.

Dans `vector_index_diagnostics_test.cpp:74`, ma construction par liste de
`std::vector<LogicalType>` exige la copie des éléments de l'initializer_list.
`LogicalType(const LogicalType&)` est privé (`types.h:272`), le type expose le
déplacement et une copie explicite. Diagnostics réels `construct_at` non viable
puis constructeur privé aux lignes1305/1378 du stdout build. Aucun besoin de
changer l'API moteur : la construction du vecteur attendu de la fixture doit
utiliser ses primitives de déplacement/emplace existantes, comme SHOW_INDEXES.

Premier refus du contrôleur : phasebuild, `build returned 1`,237,284s ; entry
conserve le retour1 de l'unité. Fin237,437s, ActiveState/SubStatefailed,
MainPID0, ControlGroupvide, quiescenttrue, aucun PID attribué restant ; résumé
`DIAGNOSTIC REFUSED`, puis **ROUGE**. Poste rendu. Les quatre fixtures sont toutes
NON JOUÉES. Aucun ELF/header/manifeste d'artefacts exporté, car la phasebuild
n'a pas réussi ; aucune identité native exécutée ni attestation59 fabriquée.

## Conservation et suite autorisée

38fichiers /657694523octets réhashés sur PC puis portable,0écart ;
[manifeste exact](preuves/rows-red-01/manifest.json) SHA256
`f0675562ebba890948da012106fe63b308b7c03799eef30453f3e931f077c179`.
[Logs/commandes/recettes/reçus complets](preuves/rows-red-01/receipts-and-logs.tar.gz),
seul source.bundle volumineux exclu de l'archive Git mais conservé aux deux postes.
PaquetPC `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph-red-v1`,
copieportable `~/.cache/rag3db-moteur-notes/rows-outside-graph-red-01`.
Aucun fichier/branche/ancienne preuve effacé, aucun service/SDK/PG/modèle/base
Lucie touché. Le tmpfs de build disparaît à la fin de l'unité ; les sorties
produites et le source.bundle suffisent à conserver ce refus de compilation.

C7 autorise ensuite PRÉPA FIXTURE SEULE, après cette MAJ du ticket AVANT diff :
remplacer seulement la construction du vecteur LogicalType move-only, garder
les quatre cas/assertions et le vrai appel SHOW_INDEXES. Le refus de compilation
ci-dessus est la preuve avant cette correction de fixture, pas le rouge produit.
Nouveau SHA/diff borné/revue et GO distinct nécessaires avant toute compilation
ou qualification. Ancien début/budget clos, aucun retry/reset/assouplissement
XML et aucun FIX de production autorisé par cette suite préparatoire.

Préparation suivante sur branche/worktree NEUFS
`cx-hotfixs-rows-outside-graph-fixture-fix` : seul diffC++ est la construction
du vecteur expectedTypes par reserve/emplace_back, dans le même ordre de neuf
types et avec les mêmes fabriques LogicalType que5099. La primitive est celle
de `show_indexes.cpp::bindColumns` ; macro EXPLICIT_COPY_DEFAULT_MOVE expose
bien le déplacement tandis que la copie reste privée. Aucun changement des
quatre cas/assertions, de la lecture publique, de la production ou du parseur.
Diff-check seulement ; compilation, test et rouge binder toujours NON JOUÉS
pour cette source révisée. Ne pas modifier ni réutiliser le paquet DB833/a31745
déjà consommé ; nouveau couple source/pilote/reçu et GO distinct requis.
