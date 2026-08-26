# 16 — Le monde est ouvert : lire n'est pas indexer

26 août 2026, 9h30. Correction du [15](15-identite-d-un-fichier.md), demandée
sur pièce : *« je te dis vas-y va regarder à tel chemin, et toi tu es bloqué
car en dehors du projet ? Ce serait dommage non ? »* — et la suite, qui est
la vraie idée : *« une politique qui retrouve tout seul le fichier et
l'ingère dans une mémoire à TTL faible… ou bien il se rend compte qu'il
touche beaucoup à tel projet, il demande à ce qu'on ingère depuis la racine
git ? Ou bien une politique s'en rend compte toute seule. »*

## 1. Ce que le 15 ratait

Le 15 a raison sur l'identité (une URI) et sur la couverture (une racine
qui n'élague que chez elle). Il se trompe sur un point, et ce point contamine
le reste : **il suppose que tout fichier intéressant est déjà dans un
projet.** « Chemin relatif au projet » n'a pas de sens pour
`/etc/hosts`, pour le dépôt d'à côté qu'on veut comparer, ou pour le fichier
qu'on vient de télécharger. Un agent qui répond *« ce chemin n'est pas dans
la source »* à un humain qui vient de lui donner ce chemin n'est pas
prudent, il est inutile.

La bonne nouvelle : l'URI, elle, tient dans un monde ouvert —
`file:///etc/hosts` est une identité parfaitement valable. C'est la
*présentation* et l'*accès* qu'il faut rouvrir.

## 2. Lire n'est pas indexer

> **L'index est un service rendu, pas une porte.**

Trois niveaux, du moins cher au plus cher, et le passage de l'un à l'autre
est une décision, pas un accident :

| Niveau | Ce qu'on obtient | Ce que ça coûte |
|---|---|---|
| **Lecture directe** | le contenu, numéroté, borné | rien — un `open()` |
| **Lecture analysée** | plus les scopes de la fenêtre lue (`codeparsers` à la volée, rien de persisté) | quelques millisecondes |
| **Mémoire à TTL** | plus la recherche, les relations, la péremption — dans une racine qui **expire** | une ingestion |
| **Projet** | tout, durablement | une décision humaine, ou une politique explicite |

Aujourd'hui on n'a que le dernier : hors de la `FileSource`, `read` échoue.
Les trois premiers manquent, et ce sont eux qui rendent l'agent utilisable
sur ce qu'on lui montre.

## 3. La frontière n'est pas le projet, c'est une capacité

Ce qui doit borner un agent n'est pas « ce qui est indexé » mais **ce que
l'opérateur autorise** : une liste de racines permises, décidée une fois.

- agent local : `~/git_workspaces`, `~/ML/models`, et pas `~/.ssh` ni
  `.vault/` ;
- agent nuage : le clone, et rien d'autre.

C'est la même forme que `NodeTypePolicy` pour les nœuds : une frontière
déclarée, vérifiable, et qui dit non avec une raison. Un refus devient
*« hors des racines autorisées : ~/git_workspaces, ~/ML »* — un fait, pas
un mystère. Et c'est ce qui remplace l'accident « pas dans la source » par
une décision.

## 4. Les chemins se décrivent eux-mêmes — pas de mode

L'idée d'un outil pour basculer absolu ↔ relatif est tentante ; je crois
qu'un **mode est un état de plus à rater**, et l'agent en rate déjà (cette
nuit : cibles inventées, chemins du dépôt). Mieux : rendre la forme
**lisible dans le chemin lui-même**.

| Forme | Sens | Quand |
|---|---|---|
| `/home/lucied/x/a.rs` | absolu, sans ambiguïté | ce que l'humain colle, ce qu'on passe à un shell |
| `proj:rag3weaver/src/a.rs` | relatif à une racine **nommée** | ce que l'agent écrit quand il sait où il est |
| `src/a.rs` | relatif à la racine courante | le raccourci du travail en cours |

Un outil **accepte les trois** et normalise ; il n'y a rien à basculer.
`proj:<id>` répond quand même à la demande — l'agent nomme la racine
ingérée qu'il vise — mais sans état caché : c'est écrit dans l'argument.
Et l'**affichage** reste un réglage du graphe
([13](13-la-session-comme-graphe.md) §6) : `paths: project | absolute`,
selon qu'on parle à un modèle distant ou à un shell local.

## 5. L'échelle de promotion

C'est la partie qui manque vraiment, et c'est l'idée de Lucie :

```
lecture directe  ──(2ᵉ ou 3ᵉ visite du même fichier)──▶  mémoire à TTL
mémoire à TTL    ──(N fichiers sous la même racine git)──▶  projet proposé
projet proposé   ──(accord, ou politique qui l'autorise)──▶  projet ingéré
```

- **La mémoire à TTL** est une `Root { uri, expires_at }` ordinaire, dans
  la **même cellule** que le reste — pas une cellule à part, pour ne pas
  se battre avec l'isolation du [14](14-tout-est-ecoutable.md) §3. Un
  graphe d'entretien élague ce qui a expiré ; « oublier » devient une
  ingestion négative, pas un cas particulier.
- **La racine git est la frontière naturelle** : remonter jusqu'au `.git`
  répond à « quel projet est-ce ? » sans rien demander à personne. Trois
  fichiers touchés sous la même racine, c'est un projet qui se signale.
- **La promotion n'est jamais silencieuse.** Soit elle passe par un message
  à l'humain (« tu as lu six fichiers de `~/proj` ; je l'ingère ? »), soit
  par une politique qui l'autorise d'avance — et qui le **dit** dans la
  trace. Ingérer trente mille fichiers sans prévenir serait le genre de
  surprise qui coûte une confiance.

## 6. La politique est un graphe qui écoute

Et c'est là que tout se raccorde. Une politique de promotion, c'est un
**réacteur** ([14](14-tout-est-ecoutable.md)) sur les événements d'outils :

```
%% tool: promotion
%% tags: policy
%% on: kind=ToolCallFinished, tool=read
%% on: kind=ToolCallFinished, tool=grep
%% policy: debounce 2000
```

Il compte les visites par racine git, décide, et agit — en ingérant, en
posant un TTL, ou en envoyant un `Message` à l'agent ou à l'humain. Rien de
nouveau à inventer : les compteurs sont un nœud, la décision est un
`BranchNode`, l'ingestion est un graphe qui existe déjà, et le message est
`SendMessageNode`. **Une politique n'est pas du code du moteur, c'est une
fiche** — donc lisible, remplaçable, et différente entre le local et le
nuage sans recompiler.

## 7. Ce qu'on ne fait pas

- **Pas d'ingestion en douce.** Une promotion se voit dans la trace, ou
  demande.
- **Pas de TTL infini** : une racine éphémère sans `expires_at` est une
  fuite de disque déguisée en mémoire.
- **Pas de lecture hors capacité**, même « juste pour voir ». Le refus dit
  ce qui est permis.
- **Pas de mode global** : ce qui change le sens d'un argument doit être
  dans l'argument.

## 8. L'ordre, et les tests qui tranchent

1. **Racines autorisées** (capacité) et **lecture directe hors index**.
   *Test* : `read('/etc/hostname')` marche si la racine est permise, et
   refuse avec la liste sinon ; aucun `File` n'est créé.
2. **Lecture analysée à la volée** : les scopes de la fenêtre lue
   apparaissent pour un fichier non indexé. *Test* : mêmes annotations
   qu'indexé, `stale` absent, zéro écriture en base.
3. **Chemins auto-descriptifs** (`/abs`, `proj:id/chemin`, relatif) et
   normalisation. *Test* : les trois formes rendent le même fichier.
4. **`Root` avec `expires_at`** et le graphe d'entretien. *Test* : une
   racine expirée disparaît avec ses fichiers, ses scopes et ses relations,
   et rien d'autre ne bouge.
5. **La fiche `promotion`**, réacteur sur les appels d'outils, avec la
   détection de racine git. *Test* : trois lectures sous la même racine
   déclenchent un message ; six déclenchent l'ingestion quand la politique
   l'autorise, et rien du tout quand elle ne l'autorise pas.

Les étapes 1 et 2 suffisent à répondre à *« va regarder à tel chemin »* —
c'est le minimum pour que l'agent cesse d'être bloqué par son propre index.
Le reste est ce qui le rend intelligent à ce sujet.
