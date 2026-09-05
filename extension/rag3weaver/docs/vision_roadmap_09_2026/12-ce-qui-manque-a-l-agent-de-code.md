# Ce qui manque à l'agent de code

**30 août 2026.** Cette liste n'est pas la nôtre : elle vient du modèle, à qui
on a envoyé la surface d'outils que le moteur publie vraiment et demandé ce
qu'il ne pouvait pas faire. Le verbatim est dans
[`docs/30-aout-2026-04h00/01`](../30-aout-2026-04h00/01-avis-du-modele-sur-nos-outils.md),
et ce qu'on en a vérifié dans le
[`02`](../30-aout-2026-04h00/02-ce-que-l-avis-a-donne.md).

Elle est ici parce qu'elle est une feuille de route, et qu'elle a une propriété
rare : **elle a été écrite par celui qui s'en sert**.

## 1. Exécuter — sa demande n°1, dite deux fois

> *« Je code à l'aveugle. »*
> *« Un agent qui ne peut pas tester ses modifications est un agent qui produit
> du code cassé. »*

C'est ce qui ferme la boucle : écrire, lancer, lire l'erreur, corriger. Sans
elle, tout le reste est de la lecture améliorée.

**Où on en est** : `src/serveur.rs` sait lancer un processus et savoir s'il
répond (29 août). `src/commande.rs` a la porte, les modes et le verdict
(30 août). `codeparsers::shell` réduit une ligne de commande en argv ou refuse
en le nommant (30 août). **Il manque le verbe** — l'outil que l'agent appelle.

## 2. Supprimer et déplacer un fichier

Créer se fait aujourd'hui par effet de bord de `edit(content=…)` ; supprimer et
déplacer, pas du tout. Le modèle : *« impossible de mener à bien des tâches de
refactoring ou de nettoyage de dette technique »*.

Et un fichier qu'on ne peut pas supprimer **pollue l'index** — il reste
cherchable après avoir cessé d'exister.

## 3. Voir le schéma du graphe

> *« Vous avez un graphe riche, mais je n'en ai pas la carte. Sinon je vais
> halluciner des noms de relations. »*

`generate_full_schema` existe dans `schema.rs`. Il n'est pas exposé. C'est le
dixième mécanisme construit et jamais branché relevé cette semaine.

## 4. Inspecter un gabarit avant de le poser

`place` dit ce qu'il a posé — **après** l'avoir posé. Le modèle : *« je vais
poser le gabarit au hasard et prier pour que la structure me convienne »*.

## 5. Deux défauts de forme, vrais et non corrigés

- **`edit` : l'exclusivité `content` / `old`+`new` n'est pas dans le schéma.**
  Rien n'empêche techniquement d'envoyer les trois. Il faudrait un `oneOf`,
  que `params_object_schema` ne sait pas produire.
- **Le pied de page de `read` est du texte à décoder** là où un `has_more` et
  un `next_offset` structurés diraient la même chose sans analyse.

## 6. Ce qu'on garde malgré la critique

**`grep` avec ses paramètres de graphe.** Le modèle trouve le mélange impur.
On a la mesure contraire : le 28 août, sur quarante appels de trois agents,
`grep` a été appelé **dix** fois et l'outil de graphe séparé **zéro**.

**`rerank` en entier plutôt qu'en booléen.** La raison tient, mais elle n'est
écrite que dans les commentaires `%%` de la fiche — **qui ne sont pas
envoyés**. Le modèle ne pouvait pas la connaître : c'est un défaut de la
description, pas du paramètre.

## 7. L'ordre

1. **exécuter** — ferme la boucle, et tout le reste en dépend
2. **supprimer / déplacer** — sans quoi on accumule
3. **voir le schéma** — évite les relations inventées
4. **inspecter un gabarit** — évite de poser au hasard
5. les deux défauts de forme
