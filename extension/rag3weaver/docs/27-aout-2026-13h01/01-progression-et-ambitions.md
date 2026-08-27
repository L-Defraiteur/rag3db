# 01 — Où on en est, et où on va

27 août 2026, 13 h 01. Trente-neuf commits depuis le rapport du 26 au soir.
Ce document est le point de reprise : ce qui est fait, ce qui est visé, et
**qui a voulu quoi** — parce que la moitié des bonnes idées de cette nuit
sont venues d'une objection de Lucie, pas d'un plan.

## 1. Ce qui est en place

| Domaine | État |
|---|---|
| **Identité d'un scope** | ne dépend plus ni de la signature ni du contenu. Un `edit` ne détruit plus rien |
| **Identité d'un fichier** | `(source, chemin absolu)` — trois versions pour y arriver ([04](../26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md)) |
| **Coordonnées** | `repo` / `repo_path` / `revision`, par fournisseurs souscrits ; deux clones se reconnaissent |
| **Relations** | toute référence laisse un `MENTIONS` **typé** ; l'ordre d'ingestion ne change plus rien, y compris pour `IMPLEMENTS` |
| **Ingestion** | index en masse (21,3 s → 6,2 s), court-circuit de l'inchangé (19,5 s → 2,1 s) |
| **Pré-filtre** | **exact** sur les trois signaux, après la correction du masque HNSW |
| **Domaine de travail** | `WorkDomain` — une sélection, câblée par le registre de services |
| **Rendu** | lentille de chemins : stockage absolu, affichage relatif |
| **Outils asynchrones** | accusé + poignée, résultat plus tard dans la boîte ([10](../26-aout-2026-20h29/10-outils-asynchrones.md)) |
| **Se taire / raccrocher** | `pause_dialogue`, `confirm_pause`, cinq genres, blocage circulaire détecté ([11](../26-aout-2026-20h29/11-le-droit-de-se-taire.md), [12](../26-aout-2026-20h29/12-conversations-a-plusieurs.md)) |
| **Conversations** | `Conversation`, `Participant`, nature sur l'arête ; cherchables, datées |
| **Dates** | `at` lisible, intervalles de borne à borne, fuseau réel (`jiff`) |
| **Machine** | `jobs = -2`, build confiné en cgroup, `unswap.sh`, carte par rôle |

**33 suites, 276 E2E, 862 unitaires.**

## 2. Les ambitions — celles de Lucie

Reprises de ses mots, avec où elles sont écrites.

1. **« Tout est un graphe. »** La recherche, les outils, la trace, la
   session, les politiques. C'est l'axiome, et il tient : un rendu est un
   nœud, une politique est une fiche, une conversation est un objet.
2. **« Une racine devrait rester qu'un point de vue. »** → [doc 04](../26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md).
   Fait, après deux versions fausses.
3. **« Un projet ne veut rien dire en soi ; un git s'appelle un git. »** →
   [doc 05](../26-aout-2026-20h29/05-origine-cellule-domaine.md). `Origin`,
   cellule, domaine : quatre axes séparés, rangés par vitesse de changement.
4. **« Domaine d'agent — ce qu'il a dans sa vision, indépendamment du
   projet. »** → `WorkDomain`, fait. Le défaut est **étroit et dérivé**, pour
   qu'un agent lâché sur un disque entier ne soit pas perdu.
5. **« Les tools asynchrones, dès maintenant — en vocal c'est chiant si
   l'agent sait plus parler. »** → [doc 10](../26-aout-2026-20h29/10-outils-asynchrones.md), fait.
6. **« Qu'il sache faire une pause, et terminer une discussion tout seul. »**
   → [docs 11](../26-aout-2026-20h29/11-le-droit-de-se-taire.md) et
   [12](../26-aout-2026-20h29/12-conversations-a-plusieurs.md), faits.
7. **« Pas de plafond codé en dur. »** La correction la plus utile de la
   nuit : mon plafond traitait le symptôme, sa pause traite la cause.
8. **« Les conversations cherchables, avec la date. »** Fait, avec le fuseau
   qui connaît ses règles — parce qu'elle a demandé « pourquoi on ajouterait
   pas la dépendance ? » et qu'il n'y avait pas de bonne raison.
9. **Les ponts entre projets** — relations multi-projets, projet-pont, ou
   `policyDomain` par entité. Noté au [doc 05 §10.4](../26-aout-2026-20h29/05-origine-cellule-domaine.md),
   **non tranché**, et aucune des trois voies ne demande de migration.
10. **La réputation des abstractions** — apportée le 27 après-midi
    (`inspi-chatgpt.md`), analysée au [doc 05](05-la-reputation-des-abstractions.md).
    Les capacités gagnent ou perdent du crédit **par expérience accumulée**,
    et non par ancienneté. Ça enrichit directement la promotion sur preuve du
    [doc 49](../23-aout-2026-20h33/49-vision-le-catalogue-comme-graphe-outils-tags-memoire.md),
    dont le critère unique (`n >= 5 AND echecs = 0`) est justement le piège de
    l'inertie. Quatre apports à prendre — séparer popularité et confiance,
    politique de sélection par contexte, avis cherchables, diversité des
    contextes — et deux à corriger : **pas de score agrégé stocké**, et
    **pas d'auto-notation** (une étoile est l'opinion que le doc 51 refuse ;
    les signaux forts sont ceux que personne ne tape).

11. **L'agent est une identité, pas une session** — apportée le 27 après-midi
    (`Résumé de projet Tony Stark.md`), analysée au
    [doc 06](06-le-tamagotchi-et-le-compilateur.md). La mémoire appartient à
    l'agent, pas au workspace ; le workspace est **le lieu où il travaille**.
    À trois fils de ce qui existe : `Run`, `Message`, `Conversation`,
    `Participant` sont livrés, il manque **une identité au-dessus du run**.
    Ça referme une asymétrie qu'on n'avait pas vue — le fil est cherchable,
    celui qui parle ne l'est pas. Avec elle viennent l'**axe agent** de la
    mémoire (le cinquième, à côté d'org/cellule/origine/domaine) et le
    **commit comme souvenir vérifiable** : on stocke l'acte, pas la phrase sur
    l'acte.
12. **« Qu'un backend soit facile à poser quelque part et à écrire par les
    agents m'importe beaucoup. »** Le schéma comme programme
    ([doc 06 §1.4 et §2.1 bis](06-le-tamagotchi-et-le-compilateur.md)). Le
    modèle produit une **représentation intermédiaire contrainte** — entités,
    signaux, relations, politiques, transitions — et le compilateur descend
    vers le stockage, la recherche, les graphes, les événements, la surface.
    On a déjà la forme (`register_entity`, `drain()`, `choices`,
    `NodeTypePolicy`) : un graphe-outil **est** une RI compilée.

    Ce qui rend l'ambition atteignable par étapes plutôt que d'un bond : la
    frontière ne sépare pas les applications simples des compliquées, elle
    sépare **les invariants exprimables dans la déclaration de ceux qui ne le
    sont pas**. On ne la déplace donc pas en rendant l'agent plus malin, mais
    **en agrandissant le langage de déclaration** — et chaque tranche est
    utile toute seule. Le vrai mur n'est pas la v1, c'est **la v2** : le
    changement de schéma avec des données dedans. Là on a un avantage réel —
    un schéma étant une donnée versionnée, une migration peut être **dérivée**.
    Première tranche : **l'état et ses transitions**, et le premier backend
    ainsi décrit sera le nôtre.

    Et la contrainte de conception qui décide de tout, dans ses mots :
    *« faut éviter d'encourager que tout soit systématiquement embedded, et
    s'assurer que les index et les relations soient bien foutus, encourager
    les bons comportements en connaissance du natif »*. Le
    [doc 07](07-le-langage-de-declaration.md) en tire quatre règles. La
    première n'est pas hypothétique : `EntityConfig::default()` vaut
    `HYBRID`, et ce défaut **a déjà fait calculer 3 275 embeddings que
    personne n'avait voulus** (`src/code.rs:180`). Il a piégé des gens qui
    connaissent le système ; un modèle ne fera pas mieux.

13. **« Un terminal multi-agent, ma vengeance de la schizophrénie. »** Des
    personnages — Alma l'architecte, Zed le reviewer sécu — à qui on parle et
    qui répondent à plusieurs, qu'on peut interpeller un par un. Analysée au
    [doc 09](09-le-terminal-a-plusieurs.md), avec sa phrase qui tient tout :
    *« un prompt devrait être agnostique de personnalité et de rôle, et lié à
    la tâche »*. D'où trois choses à ne pas confondre — **identité** (dans le
    graphe, faite le 27), **rôle** (une enveloppe de capacités, vérifiable) et
    **personnalité** (qui n'a le droit de porter que le registre). Plus le
    **tour de parole** : couper la parole existe déjà comme mécanisme
    (`Flow::Stop`), à condition que la préséance se **dérive** au lieu de se
    stocker.

## 3. Les ambitions — celles que je vois

Ce que je pousserais, et pourquoi.

1. ~~**La session comme graphe**~~ — **faite le 27 après-midi**, dans sa
   moitié qui payait ([doc 04](04-la-session-tient-l-invite.md)) :
   `Absorb` (`Whole` par défaut, `Bounded`, `Stale`), la table de renvois et
   l'outil `recall` qui les résout, et le bloc d'attentes enfin injecté au
   moment d'assembler. Le chiffre de vérité demandé par le doc 13 §9.4 existe
   et tourne à chaque `cargo test` : **900 180 → 37 567 caractères sur dix
   tours, facteur 24**, sans rien perdre. Reste l'entité `Turn` liée `IN_RUN`
   (doc 13, étape 3), qui vaut d'elle-même.
2. **`Origin` comme entité du graphe**, avec `local_path` par poste. Débloque
   le cloud, et la fiche de promotion du [doc 16](../25-aout-2026-18h58/16-le-monde-est-ouvert.md)
   (« tu touches souvent à ce dépôt, je l'ingère ? »).
3. **Honorer `Consistency`**, et corriger `flush_insertions` qui écrit sans
   indexer. Déclaré depuis longtemps, jamais tenu — c'est la plus vieille
   dette de la maison.
4. **Le fil à plus de deux** : un identifiant de conversation dans
   l'enveloppe. Petit, et il débloque « plusieurs humains, deux agents ».
5. **Le partage entre projets d'une org** par `ExportableStats` — lucivy a
   livré `search_filtered_with_global_stats`, on ne s'en sert pas encore.
6. **Le TZif**, si un jour on veut afficher une heure locale sans que le
   lecteur déclare son décalage.

## 4. Ce qui a le mieux marché, et qu'il faut continuer

**Les canaris.** Des tests qui *affirment un défaut*, avec le mode d'emploi
de leur propre mort. Trois sont tombés en vingt-quatre heures, dont un écrit
par une session précédente qui disait quoi faire quand il tomberait.

**Mesurer plutôt que déduire.** Chaque fois qu'on a cru savoir, on s'est
trompé : le masque HNSW « qui fuit » (il ne fuyait pas, il n'était pas lu),
la lenteur « des embeddings » (c'était l'index), le poste qui rame (c'était
le zram, pas le CPU).

**Et la phrase de lucivy**, qui vaut pour nous :

> Une explication convaincante d'un artefact est plus dangereuse que
> l'artefact.

## 5. Les documents, par sujet

| Sujet | Où |
|---|---|
| Vision, chaos contrôlé | `25-aout/01`, `vision_roadmap_08_2026/` |
| Feuille de route | `25-aout/06`, `25-aout/08` |
| Relations à travers les lots, couche `Symbol` | [`25-aout/17`](../25-aout-2026-18h58/17-relations-a-travers-les-lots.md) |
| Index vectoriel, coût, différé | [`25-aout/18`](../25-aout-2026-18h58/18-index-vectoriel-differe.md) |
| Identité d'un fichier | [`25-aout/15`](../25-aout-2026-18h58/15-identite-d-un-fichier.md), [`26-aout/04`](../26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md) |
| Monde ouvert, politiques de lecture | [`25-aout/16`](../25-aout-2026-18h58/16-le-monde-est-ouvert.md) |
| Session comme graphe, poignées | [`25-aout/13`](../25-aout-2026-18h58/13-la-session-comme-graphe.md) (dessin), [`04`](04-la-session-tient-l-invite.md) ici (fait, et mesuré) |
| Tout est écoutable | [`25-aout/14`](../25-aout-2026-18h58/14-tout-est-ecoutable.md) |
| Origine / cellule / domaine | [`26-aout/05`](../26-aout-2026-20h29/05-origine-cellule-domaine.md) |
| Cahier des charges lucivy, et leurs réponses | [`26-aout/06`](../26-aout-2026-20h29/06-cahier-des-charges-lucivy-partage.md), `07`, `08`, `09` |
| Outils asynchrones | [`26-aout/10`](../26-aout-2026-20h29/10-outils-asynchrones.md) |
| Se taire, raccrocher, conversations | [`26-aout/11`](../26-aout-2026-20h29/11-le-droit-de-se-taire.md), [`12`](../26-aout-2026-20h29/12-conversations-a-plusieurs.md) |
| Réputation, promotion sur preuve | [`23-aout/49`](../23-aout-2026-20h33/49-vision-le-catalogue-comme-graphe-outils-tags-memoire.md), [`23-aout/51`](../23-aout-2026-20h33/51-vision-le-chaos-controle.md), [`05`](05-la-reputation-des-abstractions.md) ici |
| Identité d'agent, mémoire, commits, concepts | [`06`](06-le-tamagotchi-et-le-compilateur.md) ici |
| Langage de déclaration, schéma-programme | [`07`](07-le-langage-de-declaration.md) ici |
| Compteur, coût, unités | [`08`](08-le-compteur.md) ici |
| Terminal à plusieurs, identité/rôle/personnalité | [`09`](09-le-terminal-a-plusieurs.md) ici |
| Commandes, mémo | [`26-aout/03`](../26-aout-2026-20h29/03-commandes.md), et [`03`](03-knowledge-dump.md) ici |
