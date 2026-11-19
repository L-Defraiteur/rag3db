# Une locale C ou C++ se relie à un scope homonyme de son fichier

- **État** : corrigé (19 novembre 2026) — pointeur codeparsers b67ad75, branche rag3db `codeparsers/stop-words-locals`
- **Famille** : produit
- **Gravité** : réponse fausse : une variable locale passe pour une référence, et une arête `CONSUMES` la relie à une
  fonction, une méthode ou un namespace du même nom dans le fichier
- **Atteignable en service** : oui, dès qu'une locale C ou C++ porte le nom d'un scope de son fichier
- **Touche rag3weaver** : oui, par codeparsers (les références des scopes C et C++)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, sonde de marque du lot des mots vides des huit langages

## Constat

`collect_local_symbols` (`base_scope_extraction_parser.rs`) relève les locales par le champ `name` d'un nœud, par
`is_definition_identifier`, et par les motifs de Rust (`let`, `for`), des fermetures et de Go (`:=`). Elle ne connaît
pas les déclarateurs du C et du C++ : `init_declarator` (`auto total = …`, `int total = …`), une `declaration` simple
(`int count;`) et `for_range_loop` (`for (auto& item : …)`). Ces variables restent donc parmi les références du scope,
et la résolution dans le fichier les relie à un scope de même nom.

Sur le moteur (`src/`, arbre figé), le lot des mots vides a retiré `function` des mots vides du C++ (c'était un mot de
JavaScript) : 81 arêtes sont apparues vers `namespace function` du même fichier, dont 2 justes
(`using namespace rag3db::function`) et **79 fausses** : 72 variables `auto function = …` et 7 variables de boucle
`for (auto& function : …)`. Le défaut agit déjà sur master pour tout autre nom de locale qui a un homonyme dans son
fichier ; le mot vide le masquait pour `function` seulement.

## Ce qu'il faudrait

Relever les noms des déclarateurs C et C++ comme locales (`init_declarator`, `declaration`, `for_range_loop`, à travers
les pointeurs et les références), pour C et C++ ensemble. Témoin rouge d'abord : une locale `auto total = …`,
`int total = …` et une variable de boucle ne sont pas des références, l'appel voisin en reste une. Puis la sonde de
marque sur le moteur : les 79 arêtes partent, et compter les autres arêtes fausses qui tombent.

## Fermé

19 novembre 2026, session codeparsers. Le nom d'un déclarateur C ou C++ (`init_declarator`, `declaration`,
`for_range_loop`, à travers `*`, `&`, `[]` et `( )`) entre dans les locales du scope, sous sa propre clé (`c_local`) :
elle cache le nom nu, mais n'est pas une liaison de Rust. Un appel de méthode sur la locale garde donc son lien par le
nom. La première forme (la clé de Rust) appliquait au C++ la règle du receveur non typé et coupait environ 577 liens
justes, chacun vers la seule déclaration de sa méthode : rattrapée avant le dépôt.

Témoin `tests/c_family_locals.rs` (codeparsers), joué sur trois têtes : la locale nue est rouge avant le correctif
(2b9a38e) ; l'appel sur une locale dans une méthode définie hors de sa classe est rouge sur la première forme (439a8d7,
`untyped_receiver true`) ; les deux sont verts sur b67ad75. Suite codeparsers 75 lots sur 75.

Sonde de marque en release sur le moteur, contre les mots vides seuls : 154 arêtes retirées — les 79 vers
`namespace function`, 71 autres locales homonymes d'une fonction, un paramètre, un `vector.resize` de la bibliothèque
standard, et 2 liens d'une classe vers une lambda de sa méthode ; 63 ajoutées, des appels qu'une
locale homonyme masquait (`auto lock = chunkedGroups.lock();`). rag3weaver/src et l'UI inchangés ; `tools/python_api` :
une locale `query` en moins.

Les 2 liens de classe s'expliquent sur les vrais fichiers (`udf_function.h`, `column_reader_writer.cpp`, essai sur
2b9a38e et b67ad75) : ils partaient des locales `execFunc` (l. 99) et `readFromPageFunc` (l. 190), qui tiennent une
lambda, vers la méthode qui la contient (`createUnaryExecFunc`, `readCompressedValues`). Ce sont deux locales de plus ;
les références de ces classes aux méthodes elles-mêmes sont identiques avant et après.
