# Trois témoins rougissent sous charge, avant comme après l'embarquement hors du verrou

- **État** : ouvert
- **Gravité** : tests instables sous charge ; en service, à établir pour le premier (une lecture
  qui verrait un lot à moitié)
- **Atteignable en service** : peut-être, pour `a_read_during_a_reindexed_batch_sees_the_batch_before_whole`
- **Touche rag3weaver** : oui (`backend.rs`, `commande.rs`)
- **Ouvert le** : 18 octobre 2026, everything declarative, en départageant le ticket fermé
  `closed/2026-10-18-la-reprise-apres-arret-rougit-sous-charge.md`
- **Pour** : à attribuer

## Ce qu'on voit

Sur luciepc, dix tours alternés de la lib entière plus seize boucles CPU, à 113e388fd et à son
parent 2b0c3739d (lib du magasin 87a362e30). Rouges sur vingt passes :

| test | 113e388fd | 2b0c3739d |
|---|---|---|
| `backend::tests::the_watch_does_not_index_what_the_index_excludes` | 3 | 2 |
| `backend::tests::a_read_during_a_reindexed_batch_sees_the_batch_before_whole` | 2 | 2 |
| `commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit` | 1 | 0 |

Les messages :

- `the_watch_does_not_index…` : « le fichier admis n'est pas passé » — `usages` ne voit pas
  encore le fichier admis à la fin de son attente.
- `a_read_during_a_reindexed_batch…` : « un lot vu à moitié », la réponse marquée « Une écriture
  de l'index est en cours : la réponse porte sur l'état validé, d'avant elle ». **C'est le seul
  qui pourrait dire un défaut** : la lecture par la vue devrait voir le lot entier ou pas du tout.
- `en_fond_un_journal…` : « la fin est gardée ».

Quatre autres rouges de ces passes (`origin::`, `work_domain::`, `code_tools::cwd_tests::`)
venaient du harnais. Le binaire était lancé hors de cargo, et ces tests lisent une variable que
cargo pose (`NotPresent`). Ils ne sont pas comptés ici.

## Ce qu'il faut pour le fermer

Pour chacun : reproduire seul sous charge, dire si c'est le délai du test ou le comportement, et
pour le second en particulier établir si une lecture peut voir un lot à moitié.

## `a_read_during_a_reindexed_batch_sees_the_batch_before_whole` mis de côté (arbre principal, 18 octobre 2026)

Rouge en CI le 18 octobre (run 39149195242, sur 6695017d6), seul rouge sur 1 674 tests. La cause
est lue : `usages` par la vue de lecture fait plusieurs requêtes (le pivot, les définitions, le
rendez-vous), chacune dans sa propre transaction. Un COMMIT du lot tombé entre deux d'entre elles
rend un rapport à cheval (l'appel sans la définition). Le lot n'est jamais coupé : dans les vingt
passes sous charge du proto, la ligne [montre] dit toujours les deux fichiers ensemble. Le test a
raison ; c'est la lecture qui manque d'un instantané.

Il porte `#[ignore]` avec ce renvoi, pour que la CI redise la vérité. Le lot **L6** (arbre
principal, après L5 : un instantané par appel d'outil, sur un réservoir de connexions de lecture)
le corrige, retire la marque et ferme cette ligne. Les deux autres témoins du ticket restent au
proto.

**Fermé par L6 (19 octobre 2026).** Un appel de lecture lit maintenant dans un instantané READ
ONLY d'une connexion du pool, ouvert à sa première requête : le pivot, les définitions et les
rendez-vous d'`usages` voient le même état, et un COMMIT du lot entre deux d'entre eux ne se voit
plus. La marque `#[ignore]` est retirée. Sur luciepc, lib cf71c929d, seize boucles de calcul en
fond : **0 rouge sur 20 passes**, contre environ 1 sur 10 avant. Témoin direct, sans montre :
`read_pool::tests::deux_requetes_d_un_instantane_voient_le_meme_etat_autour_d_un_commit`.

## `the_watch_does_not_index_what_the_index_excludes` : un défaut de la montre, corrigé (18 octobre 2026, la nuit)

Départagé sous charge (le test seul, la lib entière en fond et seize boucles CPU, vingt passes,
lib 87a362e30) avec un diagnostic en cas de rouge : **3 rouges sur 20**, et chaque fois
`admis.rs` sur le disque mais **absent de la base**, la montre à `lists: 1` pour 17 balayages, et
1 à 3 exclus vus sur 3. Ce n'était pas une famine : **la montre avait perdu le fichier.**

La cause, lue dans `watch.rs` : la liste de la source était lue, *puis* la date des dossiers
(`remember_dirs`). Un fichier créé entre les deux changeait la date sans être dans la liste ; les
tours suivants trouvaient la date inchangée, ne relisaient plus la liste, et le fichier restait
invisible jusqu'au filet de 60 s. C'est le « racy git » ; la granularité de la date d'un inode
(un tic du noyau) en donne une seconde forme.

Le correctif : une date de dossier qui n'est pas plus vieille que le début de la liste moins
`DATE_DOUTEUSE` (2 s) est tenue pour douteuse, et le dossier se relit au tour suivant. Témoin
déterministe, rouge d'abord sans le filtre : `watch::tests::a_file_created_between_the_list_and_the_dates_is_not_lost`
(un crochet de test crée le fichier entre la liste et les dates).

Cette ligne du ticket est fermée ; les deux autres restent (le lot vu à moitié, au L6 de l'arbre
principal ; le journal de commande, au proto).

## Un quatrième, vu le 19 octobre — c'était le test, corrigé

`dataflow::run_nodes::tests::en_fond_la_main_tout_de_suite_puis_tail_lit_le_neuf` a rougi une
fois, à la seconde passe de la lib entière du push du lot 4 des composants (luciepc, base
bbd81dc0a, lib 87a362e30) ; la première passe l'avait vu vert. Le message :

> les trois dernières : `#cmd-1` (`tail -f -n +1 /tmp/.tmpYl4dqn/suivi.log`) **tourne encore**.
> _Journal vide._

**La cause est dans le message** : le test cherchait les lignes `l1` à `l5` comme des
sous-chaînes du texte rendu, et le rendu cite la commande, donc le chemin du dossier
temporaire. `.tmpYl4dqn` contient `l4` : la boucle d'attente s'est arrêtée au premier regard,
le journal encore vide. Ni le délai ni `tail` n'étaient en cause. Le même défaut pouvait
frapper chaque recherche (`l1`, `l2`, `l5`, et les absences).

**Le correctif** juge les lignes du bloc rendu (`lignes_du_bloc`), jamais le texte entier, et le
dossier du test prend le préfixe `l1l2l4l5-`, qui garde ce cas : le chemin porte toujours les
lignes cherchées.

- Rouge d'abord : l'ancien test avec ce seul préfixe rougit, et dit le message même du
  19 octobre (`/tmp/l1l2l4l5-nYD6j3/suivi.log`, « tourne encore », journal vide).
- Le correctif : 0 rouge sur 20 passes sous seize boucles CPU (luciepc, base f32d97383).

Cette ligne est fermée ; les deux autres du tableau restent ouvertes.
