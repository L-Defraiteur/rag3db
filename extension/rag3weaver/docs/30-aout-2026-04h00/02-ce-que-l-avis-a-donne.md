# Ce que l'avis du modèle a donné

**30 août 2026, 4 h.** On a demandé son avis à `gemini-3.5-flash` sur nos sept
outils — [le verbatim est ici](01-avis-du-modele-sur-nos-outils.md), produit par
`tests/e2e_avis_du_modele.rs`. Voici ce qu'on a fait de chaque critique, après
vérification dans le code.

## La méthode, et le piège qu'elle a failli tendre

Les fiches envoyées ne sont pas recopiées : elles sortent de
`tools::graph_tool_defs_with`, la liste qu'un agent reçoit en production. Sinon
on demanderait son avis sur ce qu'on croit avoir construit.

**Le premier essai s'est trompé quand même**, et c'est instructif. Il n'avait pas
branché de catalogue, donc `search.target` arrivait comme une chaîne libre — et
la deuxième critique du modèle portait exactement là-dessus : *« le paramètre
`target` est une boîte noire, comment suis-je censé deviner la liste des cibles
valides ? »*. Juste sur ce qu'il voyait, faux sur ce qui existe :
`SearchSourceNode` déclare `Choices::Targets`, que `tool_def_with(catalog)`
résout en énumération réelle.

Le test branche maintenant un catalogue et **échoue** si l'énumération est vide.
Évaluer une surface plus pauvre que la vraie, c'est récolter des critiques qui
portent sur le montage.

Deuxième piège, en chemin : `GenOptions::default()` donne 512 jetons. Le premier
appel a rendu 86 caractères qui étaient la **queue du raisonnement** — ni le
début, ni la réponse. Un modèle qui réfléchit dépense d'abord son budget en
réflexion, et l'échec n'a l'air de rien : ça ressemble à une réponse brève.

## Ce qui était juste, et corrigé

### Les fiches étaient figées à la construction de l'agent

> *« Si j'instancie un template "Product" ou "User", je ne peux pas chercher
> dedans car ils ne font pas partie de l'enum. C'est bloquant. »*

**Vrai, et pire que ce qu'il dit.** `Agent::new` calculait `tool_defs()` **une
fois** et les gardait dans `opts`. Les listes closes — les cibles de `search`,
les relations — étaient donc celles du catalogue au moment de la construction.
Un agent qui posait une entité avec `place` ne pouvait plus la chercher.

Et cette même liste **contraint le décodage** : le modèle ne pouvait pas même
prononcer le nom de ce qu'il venait de créer.

Le défaut était juste sous les deux outils qu'on venait d'ajouter, et c'est le
modèle qui l'a vu, à partir des fiches seules. Corrigé : les fiches se relisent
à chaque tour, les outils ajoutés à la main survivent, et un test le fixe
(`les_fiches_suivent_le_catalogue_qui_bouge`).

## Ce qui était juste, et reste ouvert

- **`edit` : l'exclusivité `content` / `old`+`new` n'est pas dans le schéma.**
  Rien n'empêche techniquement d'envoyer les trois. Il faudrait un `oneOf`, que
  `params_object_schema` ne sait pas produire.
- **`patterns` est une chaîne à virgules là où JSON a des tableaux.** Le modèle
  a raison : `ConfigParamType::Json` avec un `json_schema` d'array l'exprimerait.
- **Pas de `run` / d'exécution.** Sa critique n°1, et deux fois : *« je code à
  l'aveugle »*. C'est exactement ce que `src/serveur.rs` a commencé à débloquer
  le 29 au soir — savoir lancer un processus. Le verbe manque encore.
- **Pas de `delete_file` / `move_file`.** Créer se fait par l'effet de bord de
  `edit(content=…)` ; supprimer et déplacer, pas du tout.
- **Pas d'inspection : `get_schema`, et un gabarit avant de le poser.** On a
  `generate_full_schema` dans `schema.rs`, non exposé. Et `place` ne dit ce
  qu'il a posé qu'**après** l'avoir posé.
- **Le pied de page de `read` est du texte à décoder** là où un `has_more` et un
  `next_offset` structurés suffiraient.
- **`direction` combiné à des relations déjà orientées** (`CONSUMED_BY` +
  `Incoming`) est une source d'erreur qu'on n'avait pas vue sous cet angle.

## Ce qu'on garde malgré la critique, et pourquoi

**`grep` avec ses paramètres de graphe.** Le modèle trouve le mélange impur :
*« si je cherche une regex je veux des lignes ; si je veux le graphe j'utilise
search »*. C'est un raisonnement de principe, et on a la mesure contraire :
le 28 août, sur quarante appels de trois agents, `grep` a été appelé **dix**
fois et l'outil de graphe séparé **zéro**. Un agent fait son `grep` — son
réflexe — et lève un drapeau quand il veut les relations. On garde.

**`rerank` en entier plutôt qu'en booléen.** *« Si 20 est une bonne valeur,
pourquoi n'est-ce pas le défaut ? »* Parce qu'un cross-encoder sur un nom de
fonction coûte cher et n'apporte rien, et que le rendre explicite oblige à la
seule chose qui le rend utile : poser une vraie question. Mais **la raison n'est
écrite que dans les commentaires `%%` de la fiche, qui ne sont pas envoyés** —
le modèle ne pouvait pas la connaître. C'est un défaut de la description, pas du
paramètre.

## Ce que l'exercice apprend sur l'exercice

Trois choses, qu'on refera :

1. **Envoyer la surface réelle**, catalogue branché, et échouer sinon.
2. **Donner un budget de jetons qui compte la réflexion** — sinon on lit la
   queue d'un raisonnement en croyant lire une réponse.
3. **Vérifier chaque critique dans le code avant de la croire.** Sur sept
   griefs, un était bloquant et vrai, un portait sur notre montage, deux
   étaient des choix documentés ailleurs que là où le modèle regarde.
