# Brouillon de vision — un dépôt de référence, indexé à la demande

4 octobre 2026. **Brouillon** : rien n'est décidé ni codé. Idée de Lucie :

> « L'agent veut indexer un git de référence — "inspire-toi de ce git pour
> faire ça" — hop, il propose à l'utilisateur de l'indexer comme référence, et
> là il fait ses recherches dessus, et le grep se sert de l'index. J'ai peur
> d'une désynchronisation pour pas grand-chose avec un grep par lucivy
> [sur le dépôt de travail]. »

Ce document complète les visions du produit code
(`../3-octobre-2026-20h37/01`).

## 1. L'idée

Un agent à qui l'on dit « inspire-toi de tel dépôt » n'a aujourd'hui que deux
moyens : le cloner et le lire au grep, ou s'en souvenir de travers. La
référence en fait un objet de plein droit : un dépôt **figé à un commit**,
indexé à part, en lecture seule, sur lequel tous les outils de recherche
marchent.

## 2. Pourquoi le grep par l'index est sûr ici, et pas ailleurs

- **Le dépôt de travail bouge sans arrêt** : un grep par l'index y rendrait
  un contenu périmé ou raterait une occurrence en silence. Le vrai grep y
  reste, on n'y touche pas.
- **Une référence ne bouge pas** : indexée à un commit, jamais éditée. L'index
  et le contenu ne peuvent pas diverger ; le grep par lucivy y est exact par
  construction.
- **Une référence n'a pas besoin de fichiers sur le disque** : son contenu
  peut ne vivre que dans l'index. Le grep par l'index y est alors le chemin
  naturel, pas une optimisation.

La source de fichiers est déjà une abstraction (`FileSource`,
`src/code_tools.rs`) : un genre « instantané » existe, un genre « git à un
commit » est prévu.

## 3. Le parcours

1. L'agent propose : « indexer `<dépôt>` comme référence ? » — c'est une
   proposition, la personne décide.
2. `estimate` dit le coût et la durée ; la personne confirme.
3. Le dépôt est indexé à un commit, à part du dépôt de travail.
4. Les outils prennent la référence comme source : recherche, grep, `usages`,
   `impact`, liens. Chaque résultat dit de quelle source il vient.

## 4. Ce qui est à trancher

- **Les mots d'abord, les vecteurs ensuite** : cherchable par mots en
  quelques minutes, les embarquements en fond (l'état d'index par entité le
  dit déjà).
- **Où elle vit** : une base par référence, gardée en cache par adresse et
  commit, réutilisable d'un projet à l'autre ; ou dans la base du projet.
- **La mise à jour** : un nouveau commit donne un nouvel instantané ; l'ancien
  reste valable tant qu'on ne le jette pas. Qui jette, et quand.
- **La licence** : la fiche de la référence porte sa licence et la dit à
  l'agent — reprendre du code, ou seulement s'en inspirer. Une référence sans
  licence se lit, ne se copie pas.
- **Le mélange des sources** : une recherche porte sur le dépôt de travail,
  sur une référence, ou sur les deux ; par défaut, jamais les deux sans le
  dire.
- **La mémoire** : une fiche peut pointer vers une ligne d'une référence
  (`row:…` dans la source de la référence), comme vers toute adresse
  (`../4-octobre-2026-00h09/01`).
- **Ce que le grep par l'index sait faire** : sous-chaînes et expressions
  régulières par lucivy, à vérifier ; ce qu'il ne sait pas faire se dit, il
  ne rend pas un résultat partiel en silence.

## 5. Ce que le moteur donne et ce qui manque

| Besoin | État |
|---|---|
| Indexer un dépôt de code, par mots puis par sens | livré (`index`, `estimate`, états d'index) |
| Une source figée, en lecture seule | genre « instantané » livré ; genre « git à un commit » prévu |
| Plusieurs sources dans les outils | à faire : un paramètre de source, et la source dite dans les résultats |
| Grep par l'index | à faire ; dépend de ce que lucivy sait des sous-chaînes |
| La proposition par l'agent, avec confirmation | la confirmation d'`index` existe |
| Le cache des références | à faire |

## 6. Par où commencer, si on le fait

1. Une référence locale : un dossier déjà cloné, indexé comme instantané,
   interrogé par les outils avec un paramètre de source. Vrai grep encore.
2. La source « git à un commit », sans fichiers sur le disque.
3. Le grep par l'index, sur les références seulement.
4. La proposition par l'agent, la fiche de licence, le cache.
