# Ce qui reste codé en dur : la recherche, l'analyse du code, l'hôte — le samedi 21 novembre 2026

La question de Lucie (20 novembre, 21 h 3x) : « à quel point le code, pour des features et non des correctifs,
devient juste l'écriture de gabarits, et à quel point on doit encore hardcoder ? ». Cette page est la part d'ED. La
part d'archi, les agents et les tâches, est dans `docs/rag3weaver-archi/20-11-2026/04-ce-qui-reste-code-en-dur-agents-et-taches.md` :
elle pose la ligne, et cette page la suit. Lucie tranche sur les deux.

## La ligne (celle d'archi, et les deux critères de Lucie)

- **Reste en Rust** ce qui touche le système ou une frontière de sûreté, ce qui doit tenir quoi que dise une
  déclaration, et une primitive dont la performance est mesurée.
- **Passe en déclaré** (au gabarit) ce qui est un texte, une borne, un choix ou une composition.
- **Passe en nœud scripté** une logique qui doit pouvoir changer : transformations, mises en forme, règles,
  assemblages. Et tout ce qui est **en construction ou incertain** : on y essaie plusieurs configurations sans rebâtir.
  Ce qui est figé et prouvé ne descend en Rust que s'il est une primitive ou si la performance le demande.

## Domaine par domaine

| Domaine | Déjà déclaré | Encore en Rust | Pourquoi en Rust | Peut passer en déclaré ou en script, et ce que ça coûte |
|---|---|---|---|---|
| **La recherche** | Le graphe entier (`search_workspace.mmd`) : les signaux, leurs poids, le dominant (`bm25_exact`), le nom exact (`exact_name`, `exact_name_leaf`), la lecture par le nom (`ExactNameNode`, sa limite) ; les options (limite, langue, chemins) ; le reclassement à la demande ; les poids par champ d'une entité | Les nœuds de signal (BM25, dense, creux) ; la fusion RRF ; la ligne sans vecteur (le dense compté au dernier rang, et jamais devant la meilleure ligne à vecteur) ; le budget de chaque signal, `(limit + offset) × 2` ; les étiquettes qui « lisent les morceaux » (`vector`, `sparse`) | Des primitives chaudes : une recherche appelle chaque nœud une fois par requête, sur des milliers de lignes. La règle de la ligne sans vecteur a changé trois fois en une nuit, mais elle trie la liste fusionnée entière. | **Déclaré, peu cher** (une demi-journée) : les étiquettes qui lisent les morceaux et le facteur du budget, en paramètres de `FuseResultsNode` et des signaux ; la règle de la ligne sans vecteur, en paramètre à choix (`sans_vecteur='imputed_last'`). **Script, pas encore** : la règle de fusion change encore, mais elle est sur un chemin chaud (1 ms par appel scripté contre une fusion de quelques ms) ; à reconsidérer quand un moteur de script gardé d'un appel à l'autre est mesuré. |
| **L'analyse du code et l'index** | Les seuils (`workspace.container_min_chars`, `variable_min_chars`) ; l'exclusion (`index_exclude` : lignes, tailles, chemins, défauts levables) ; les montres ; `queries_via`, `locks_via` ; la borne des morceaux, tirée du plus petit embarqueur déclaré | La liste des règles de non-candidats (les conteneurs repliés, les déclarations de valeur et leurs types, ce que chacune retire) ; les **défauts** de l'exclusion (5 000 octets, 1 Mio, `dist/`, `build/`, les bundles) ; la marge de la borne (0,9) ; le numéro des règles de l'analyse ; l'analyseur (codeparsers) | Une règle de non-candidat change ce que la base indexe : elle a un numéro (`REGLES_DE_L_ANALYSE`), une empreinte, un repassage. Les défauts doivent valoir sans déclaration. L'analyseur est une primitive lourde. | **Déclaré, moyen** (un à deux jours) : la liste des règles de non-candidats entière dans le gabarit (types, seuils, ce qu'elle retire). Il faut que l'empreinte de l'analyse lise la déclaration, sinon une base ne sait plus quand repasser. **Déclaré, peu cher** : les défauts d'exclusion dans le gabarit livré plutôt qu'en constantes (une base les voit et les lève comme aujourd'hui) ; la marge de la borne. **Script : non.** C'est un chemin chaud : 117 000 scopes à 1 ms, deux minutes par index. |
| **L'hôte et les vues** | Les outils (`builtin:`, graphes), les réactions, les montres (`every_ms`, `settle_ms`), les composants, les vues ; la version des gabarits servis et leur dérive (dites à `/sante`) | La vie de l'hôte (sockets, sessions, ouverture, fermeture) ; la maintenance du plein texte et son heure ; le délai d'arrêt d'une réaction (30 s) ; la garde des comptes du plein texte ; la politique de rafraîchissement des gabarits | Système et sûreté : un arrêt qui laisse un catalogue ouvert, une garde qui ment (le faux « écritures perdues » de ce matin) coûtent des heures. | **Déclaré, peu cher** : l'heure de la maintenance et le délai d'arrêt d'une réaction, en clés du manifeste avec défaut. **Le reste en Rust.** |

## En trois colonnes

| Chez la recherche, l'analyse et l'hôte | Gabarit (une donnée) | Nœud scripté (une logique qui change) | Rust (une primitive, une garde) |
|---|---|---|---|
| La composition d'une recherche (signaux, poids, dominant, nom exact, lecture par le nom) | ✔ (déjà) | | |
| Les options d'une recherche, leurs défauts | ✔ (déjà) | | |
| Les étiquettes qui lisent les morceaux, le facteur du budget, la règle de la ligne sans vecteur | ✔ (à faire) | | |
| Les nœuds de signal, la fusion RRF, le tri de la liste fusionnée | | | ✔ (chemin chaud) |
| La mise en forme d'un résultat rendu (le rendu d'une ligne, ses liens) | | ✔ (une fois par page rendue, pas par ligne indexée) | |
| Les seuils, l'exclusion et ses défauts, la marge de la borne | ✔ (en partie déjà) | | |
| La liste des règles de non-candidats | ✔ (à faire, avec l'empreinte) | | |
| L'analyseur, le découpage, le compte des jetons | | | ✔ (chemin chaud, primitive) |
| L'heure de la maintenance, le délai d'arrêt d'une réaction | ✔ (à faire) | | |
| La vie de l'hôte, la garde des comptes, la fermeture d'un catalogue | | | ✔ (sûreté) |

## Ce que le nœud scripté sait, et ce qui lui manque

Le nœud scripté (`ScriptedNode`, un `.ts` dans QuickJS) a été validé cette nuit sur le bloc « Previous step » de
l'équipier : `docs/everything-declarative/21-11-2026/01-le-noeud-scripte-a-l-essai.md`.

**Ce qui est vrai, et prouvé** :
- le temps est borné : une boucle infinie est arrêtée par son échéance, au chargement comme à l'appel ;
- la mémoire est bornée : une bombe mémoire est arrêtée par son plafond ;
- ni fichier ni réseau : `require`, `fetch` et les imports sont refusés, en les nommant ;
- les erreurs donnent leur ligne, telle qu'écrite dans le `.ts` ;
- les entrées et les sorties sont vérifiées par les schémas des ports ;
- un `.ts` édité sert au pas suivant, et deux configurations du même script se jugent sur la même entrée, sans
  rebâti, quand le point d'appel relit la déclaration ;
- un script cassé ne casse pas le pas : le Rust prend la main, et la boîte le dit.

**Ce qui manque**, pour en faire le défaut d'un nœud neuf :
1. le rechargement à chaud des nœuds d'un graphe dans l'hôte : aujourd'hui, un backend ouvert ne relit pas son
   dossier `nodes/` ;
2. le nom du fichier `.ts` dans les erreurs du nœud lui-même : le moteur donne la ligne, pas le fichier ;
3. une API « jouer un nœud déclaré » hors d'un graphe : ce sont les points déclarés d'archi (environ deux jours) ;
4. **l'accès aux entités par un contrat** : un script ne reçoit que son entrée en JSON. Il ne peut ni lire une
   entité, ni chercher. Pour qu'une règle scriptée juge d'après la base, il faudrait des fonctions données au script
   (lire une entité par son uuid, une recherche bornée), chacune en lecture seule et bornée comme le reste ;
5. pour un chemin chaud, un moteur gardé d'un appel à l'autre : la mesure (1,09 ms par appel, 8,8 µs en Rust) est
   surtout le coût de l'appel au moteur.

## Ce que coûte l'ensemble, dans l'ordre que je propose

1. **Les paramètres de la recherche** (étiquettes qui lisent les morceaux, facteur du budget, règle de la ligne sans
   vecteur) : une demi-journée. Ils sortent du Rust une règle qui a bougé trois fois en une nuit.
2. **Les défauts d'exclusion et la marge de la borne au gabarit** : une demi-journée.
3. **L'heure de la maintenance et le délai d'arrêt d'une réaction** : quelques heures.
4. **La liste des règles de non-candidats au gabarit**, avec une empreinte qui la lit : un à deux jours. C'est le plus
   utile pour essayer des règles sans rebâti, mais chaque essai coûte un repassage de la base.
5. **Pour le nœud scripté** : le nom du fichier dans ses erreurs (quelques heures), puis le rechargement des nœuds
   dans l'hôte (un jour), puis l'accès aux entités par un contrat (deux jours, à concevoir avec archi).

Rien de cela ne relâche une garde : une déclaration règle une borne ou un choix, et le Rust garde ce qu'il garde.
