# Mission — LadybugDB : la sonde de faisabilité

**3 septembre 2026.** Quatre documents pour qu'une session qui ne connaît rien
du projet puisse mener cette mission sans redécouvrir.

| | |
|---|---|
| [01 — Le repérage](01-reperage-ladybug.md) | Ce qui a été mesuré et ce que ça a trouvé. **À lire en premier**, c'est le constat. |
| [02 — La mission](02-la-mission-et-son-critere.md) | La question exacte à trancher, l'arbre de décision, et ce qui ferait abandonner. |
| [03 — Notre greffe](03-notre-greffe.md) | Ce que font vraiment nos 542 lignes du cœur, qui s'en sert, et pourquoi elles sont au niveau du **plan** et non du stockage. |
| [04 — Knowledge dump](04-knowledge-dump.md) | Comment construire, comment tester, où sont les choses, et les pièges qui coûtent une demi-journée. |

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

## Ce qui est déjà fait, et qu'il ne faut pas refaire

- Le repérage lui-même (document 01) : les chiffres sont mesurés, pas estimés.
- Le point de comparaison est **épinglé** par le tag `ladybug-main-2026-08-31`
  (`bdc162654`, *« Add group-key predicate pushdown optimizer »*). Les objets
  sont dans le dépôt local ; le distant a été retiré. Pour le rouvrir :

  ```sh
  git remote add ladybug https://github.com/LadybugDB/ladybug.git
  git fetch ladybug
  ```

- Aucune fusion, aucun rebasage, aucune modification du code n'a été faite.
