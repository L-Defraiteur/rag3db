# Mission — LadybugDB : la sonde de faisabilité

**3 septembre 2026.** Quatre documents pour qu'une session qui ne connaît rien
du projet puisse mener cette mission sans redécouvrir.

| | |
|---|---|
| [01 — Le repérage](01-reperage-ladybug.md) | Ce qui a été mesuré et ce que ça a trouvé. **À lire en premier**, c'est le constat. |
| [02 — La mission](02-la-mission-et-son-critere.md) | La question exacte à trancher, l'arbre de décision, et ce qui ferait abandonner. |
| [03 — Notre greffe](03-notre-greffe.md) | Ce que font vraiment nos 542 lignes du cœur, qui s'en sert, et pourquoi elles sont au niveau du **plan** et non du stockage. |
| [04 — Knowledge dump](04-knowledge-dump.md) | Comment construire, comment tester, où sont les choses, et les pièges qui coûtent une demi-journée. |
| [05 — La réponse](05-la-sonde-et-sa-reponse.md) | **La sonde est faite.** Les trois phrases du critère, étayées ligne par ligne. À lire après 01. |
| [06 — Le second fork](06-reperage-vela.md) | **Vela**, l'autre continuation MIT. Même histoire git, aucun renommage, 55 fichiers sur 58 se fondent seuls. Mais l'écriture multi-processus n'est pas résolue. |

## La mission en trois phrases

Kuzu est mort le 10 octobre 2025. **LadybugDB** en est la continuation MIT, et
elle a construit entre mai et juin 2026 un mécanisme d'**index secondaires
enfichables avec descente de prédicat**. Nous avons greffé au cœur, de notre
côté, 542 lignes qui font descendre un prédicat vers un index fourni par une
extension — aujourd'hui la recherche vectorielle HNSW.

La question est de savoir si les deux mécanismes se recouvrent. **Le leur
interroge par clé** — recherche exacte, parcours d'intervalle. **Le nôtre
interroge par score** — un top-K classé. S'ils se rejoignent, notre greffe se
réduit et on redevient un utilisateur avec des extensions. S'ils sont
orthogonaux, elle survit, et devient une contribution amont évidente.

C'est une **sonde de faisabilité**, pas une migration : elle se répond par oui
ou par non, et elle décide de tout le reste.

## La réponse, obtenue le jour même

**Non — leur descente de prédicat n'atteint pas un index classé**, et elle est
fermée deux fois : par la forme du prédicat et par le type d'index. Ils ont bien
une extension vectorielle — dans un dépôt séparé, et c'est celle de Kuzu,
inchangée : on l'interroge par une **fonction de table**, jamais par un `WHERE`.
Ils ont donc exactement le manque que notre greffe comble, sans l'avoir comblé.

Les deux généralisations sont **orthogonales**. Notre greffe se réapplique — 15
de nos 24 greffons du cœur se reposent seuls, 2 seulement demandent un arbitrage
réel — et notre `IndexSearchBindData` devient une contribution amont évidente.

La vraie difficulté est ailleurs, et le repérage ne pouvait pas la voir : Ladybug
a **sorti ses extensions et ses API de langage en dépôts séparés**. Nos 171
fichiers hors du cœur ne se fusionnent plus fichier par fichier. Le chantier
n'est pas le code, c'est le découpage en dépôts.

Tout est dans [05 — La réponse](05-la-sonde-et-sa-reponse.md), qui corrige aussi
deux erreurs des documents précédents.

## Et le second fork, celui de la mémoire d'agent

Repéré dans la foulée, puisque la sonde avait tranché vite.
`Vela-Engineering/kuzu` est **l'inverse de Ladybug point par point** : notre
dernier commit amont est leur ancêtre, ils n'ont rien renommé, et tout est resté
dans un seul dépôt — HNSW compris. Leur travail sur l'écriture concurrente se
fond dans le nôtre à **55 fichiers sur 58**, avec trois conflits d'une zone
chacun, dont un qui *simplifie* notre code.

Mais il faut être précis sur ce qu'ils résolvent : leur concurrence est
**interne au processus**, et l'écrivain garde le verrou exclusif de fichier.
Plusieurs processus **lecteurs** deviennent possibles, plusieurs processus
**écrivains** non — et c'était notre douleur.

Tout est dans [06 — Le second fork](06-reperage-vela.md).

## Ce qui est déjà fait, et qu'il ne faut pas refaire

- Le repérage lui-même (document 01) : les chiffres sont mesurés, pas estimés.
- Le point de comparaison est **épinglé** par le tag `ladybug-main-2026-08-31`
  (`bdc162654`, *« Add group-key predicate pushdown optimizer »*). Les objets
  sont dans le dépôt local ; le distant a été retiré. Pour le rouvrir :

  ```sh
  git remote add ladybug https://github.com/LadybugDB/ladybug.git
  git fetch ladybug
  ```

- Aucune fusion, aucun rebasage, aucun renommage. La sonde s'est répondue **en
  lecture seule**, sans compiler LadybugDB.
- Le nettoyage du code mort (64 lignes, deux en-têtes résiduels) **est fait**.
