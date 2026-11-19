# Une locale C ou C++ se relie à un scope homonyme de son fichier

- **État** : ouvert (19 novembre 2026) — correctif en cours dans la pile des mots vides (branche codeparsers
  `stop-words-every-language`)
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
