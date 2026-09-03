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
fermée deux fois : par la forme du prédicat et par le type d'index. Ils n'ont
d'ailleurs aucune extension vectorielle, donc jamais eu notre problème.

Les deux généralisations sont **orthogonales**. Notre greffe se réapplique — 15
de nos 24 greffons du cœur se reposent seuls, 2 seulement demandent un arbitrage
réel — et notre `IndexSearchBindData` devient une contribution amont évidente.

La vraie difficulté est ailleurs, et le repérage ne pouvait pas la voir : Ladybug
a **sorti ses extensions et ses API de langage en dépôts séparés**. Nos 171
fichiers hors du cœur ne se fusionnent plus fichier par fichier. Le chantier
n'est pas le code, c'est le découpage en dépôts.

Tout est dans [05 — La réponse](05-la-sonde-et-sa-reponse.md), qui corrige aussi
deux erreurs des documents précédents.

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
