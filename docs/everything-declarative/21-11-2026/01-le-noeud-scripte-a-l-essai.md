# Le nœud scripté à l'essai : le bloc « Previous step » de l'équipier

21 novembre 2026, nuit, session everything declarative. Demande de Lucie (20 novembre, 21 h 4x) : « il faut d'abord
valider le scripting, qu'il fonctionne bien, on ne sait jamais, et qu'il est pratique là où il est placé ». Le verdict
est en fin de page.

## La fonctionnalité choisie

Avec archi : le bloc « Previous step » que l'équipier met en tête de chaque pas. C'est ce que le pas d'avant a lu,
écrit et lancé, et son dernier texte, borné à `teammate.carry_over_chars` caractères. En Rust, c'est
`StepWork::block`. Une fonction pure : un état en entrée, un texte en sortie. Le Rust sert d'oracle.

## Ce qui est posé

- **Le nœud livré** : `templates/backends/code/nodes/previous_step.node.json` et `previous_step.ts`. Le nœud se
  déclare par un port d'entrée `work` (un JSON Schema de l'état), un port de sortie `block` (un texte), et une
  configuration `style` (`full` : le bloc du Rust ; `compact` : sans la fin de sortie de la dernière commande).
- **Le branchement** : `teammate.carry_over_node = "PreviousStep"` dans le manifeste, et
  `teammate.carry_over_node_config` pour sa configuration. Sans la clé, rien ne change : le bloc Rust sert.
- **Le repli** : si le script échoue, le bloc Rust sert et la boîte de l'équipier le dit une fois par erreur
  distincte, avec le fichier `.ts` et la ligne.

## Ce qui est prouvé, et comment

| Promesse | Témoin | Rouge vu |
|---|---|---|
| Le script rend le bloc Rust à l'octet près, coupe comprise, sur sept états (accents, emojis à la coupe) | `the_scripted_previous_step_is_the_rust_one_to_the_byte` | contre une ébauche |
| Un `.ts` édité sert à l'appel suivant, sans redémarrer ; deux configurations du même script donnent deux blocs sur la même entrée | `an_edited_script_serves_at_the_next_call_and_two_configurations_give_two_blocks` | contre une ébauche |
| Un script cassé ne casse pas le pas : le bloc Rust sert, l'erreur nomme le fichier et la ligne | `a_broken_script_falls_back_to_the_rust_block_and_names_its_file_and_line` | contre une ébauche |
| Par le chemin de l'hôte : une boucle réelle de l'équipier, nœud déclaré et cassé ; le pas suivant voit son bloc, la boîte dit le repli une fois | `a_failing_scripted_previous_step_falls_back_and_the_box_says_it` | branchement neutralisé |
| Temps borné, mémoire bornée, ni fichier ni réseau, erreur avec sa ligne | déjà prouvé dans `src/script/tests.rs` (boucle infinie, boucle au chargement, bombe mémoire, `require`, `fetch`, import, ligne écrite) | antérieur |

## La mesure, en release

Témoin ignoré `measure_the_scripted_previous_step_against_the_rust_one`, joué en release à la tablette, sous
`poste lourd`. L'état est de taille réelle : 200 lectures, 50 écritures, une commande et ses 1 500 derniers caractères,
un dernier texte de 3 000 caractères, au-delà du budget de 8 000. Médianes sur 200 appels :

| Chemin | Médiane |
|---|---:|
| Le bloc Rust (`StepWork::block`) | 8,8 µs |
| Le nœud tel qu'un pas l'appelle : la déclaration relue, le script préparé, l'appel | 1,33 ms |
| Le script seul, préparé une fois : l'appel | 1,09 ms |

**Lecture.** Le script coûte environ 150 fois le Rust, et c'est l'appel au moteur qui pèse (1,09 ms), pas la relecture
du fichier (0,24 ms). À l'échelle d'un pas de l'équipier, qui attend le modèle plusieurs secondes, 1,3 ms ne se voit
pas. À l'échelle d'une ligne d'index, c'est rédhibitoire : 1 ms par ligne sur les 117 000 scopes d'une base, c'est deux
minutes de plus par index complet.

## L'ergonomie : ce qui est bien, ce qui est gauche

**Bien.**
- Le script est court et lisible (40 lignes), en TypeScript ordinaire. Les types s'effacent au chargement, et ce sont
  les schémas des ports qui vérifient à la frontière.
- Une erreur dit sa ligne telle qu'écrite dans le `.ts`.
- Les gardes de temps, de mémoire et d'accès existent et sont prouvées.

**Gauche.**
1. **Le rechargement à chaud n'existe pas dans l'hôte.** Un backend ouvert ne relit pas son dossier `nodes/` quand un
   `.ts` change. Ce lot contourne en relisant la déclaration à chaque appel (le coût est dans la mesure). Pour un nœud
   posé dans un graphe, il faudrait que l'hôte surveille `nodes/` ou que le graphe relise sa déclaration.
2. **L'erreur ne nomme pas le fichier.** Le moteur donne la ligne et la colonne, pas le `.ts` : ce lot ajoute le
   chemin lui-même. C'est à remonter dans `ScriptedNode`.
3. **Compter des caractères est un piège.** En JavaScript, `.length` compte des unités UTF-16 : un emoji en vaut deux.
   Le script doit passer par `Array.from` pour compter comme Rust. Un nœud qui borne un texte doit le savoir.
4. **Un nœud ne s'appelle pas hors d'un graphe.** Il a fallu une petite fonction (`carry_over_by_node`) pour trouver le
   nœud par son nom de type, le créer, poser son entrée et lire sa sortie. Une API « jouer un nœud déclaré » éviterait
   de la réécrire à chaque point d'appel ; c'est le rôle des points déclarés d'archi.
5. **Une tâche part d'un commit.** Un script modifié dans le dépôt des déclarations sans être commité n'est pas vu par
   une tâche. C'est voulu, mais un agent qui édite son propre script doit le savoir.

**Pour qu'un agent l'édite.** Il lui faut : le chemin du `.ts` (dans `nodes/`, à côté de sa déclaration), le schéma
de ses ports (dans le `.node.json`), un oracle (ici le Rust, ailleurs des cas d'entrée et de sortie), et le retour de
son erreur (la boîte le dit). Il ne lui manque que de savoir quel nœud sert quel point : la clé du manifeste le dit
pour ce cas, les points déclarés d'archi le diront pour les autres.

## Le verdict

**Le scripting fonctionne, et il est sûr.** Le script rend le Rust à l'octet près, y compris sur la coupe et les
caractères hors du plan de base. Un script cassé ne casse rien : le Rust prend la main et la boîte le dit avec le
fichier et la ligne. Un `.ts` édité sert au pas suivant, sans rebâti ni redémarrage, et deux configurations du même
script se jugent sur la même entrée. Les gardes de temps, de mémoire et d'accès sont prouvées.

**Il est pratique là où l'appel est rare** : une fois par pas, par requête, par tour de chat. Mises en forme, règles
d'un pas, assemblages, textes. **Il ne l'est pas sur un chemin chaud** (une fois par ligne d'index, par morceau, par
résultat d'une recherche) : 1 ms par appel y devient des minutes.

**Ce qu'il faut avant d'en faire le défaut d'un nœud de fonctionnalité** (le critère de Lucie : en construction ou
incertain, un script ; figé et prouvé, du Rust s'il s'agit d'une primitive ou d'une performance) :
1. le rechargement à chaud des nœuds d'un graphe dans l'hôte (aujourd'hui seul un point d'appel qui relit sa
   déclaration, comme celui-ci, en profite) ;
2. le nom du fichier `.ts` dans les erreurs du nœud scripté lui-même ;
3. une API « jouer un nœud déclaré » pour les points d'appel hors d'un graphe : ce sont les points déclarés d'archi ;
4. si un script doit servir sur un chemin chaud, un moteur gardé d'un appel à l'autre (le coût mesuré est surtout
   celui de l'appel au moteur), à mesurer avant de le promettre.

Rien de cela ne bloque les points déclarés d'archi : ils sont du premier genre, rares et hors des chemins chauds.
