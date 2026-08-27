# 04 — La session tient l'invite : absorber, renvoyer, et rendre l'attente visible

Le [doc 13](../25-aout-2026-18h58/13-la-session-comme-graphe.md) décrivait six
points de décision dans un tour d'agent. Deux d'entre eux étaient **la même
pièce manquante, réclamée deux fois** :

- `absorb` — ce qu'on garde d'un résultat d'outil dans l'historique ;
- `assemble` — ce qu'on envoie au modèle, et où le [bloc d'attentes](../26-aout-2026-20h29/12-conversations-a-plusieurs.md#9-ce-qui-attend-doit-se-voir-pas-se-rappeler)
  du doc 12 §9 devait être injecté.

Les deux vivent maintenant dans `src/session.rs` et dans deux méthodes de
`Agent`. Ce document dit ce qui a été fait, ce qui a été **mesuré**, et les
trois choix qui pourraient se discuter.

## 1. Le problème, en une phrase

Une conversation avec outils ne grossit pas linéairement : le résultat d'un
`read` obtenu au tour 2 est **réenvoyé au modèle à chaque tour suivant**. Au
tour 10 on l'a payé neuf fois, et presque toujours pour rien — il a servi une
fois, au tour où il est arrivé.

C'est **quadratique**, et c'est le genre de coût qui ne se voit pas : rien ne
casse, rien ne ralentit visiblement, la facture monte.

## 2. Ce qui a été fait

### 2.1 `Absorb` — trois politiques, un défaut qui ne fait rien

| Politique | Ce qu'elle garde dans l'invite |
|---|---|
| `Whole` (**défaut**) | tout, tel quel — la boucle d'hier, à la lettre |
| `Bounded { max_chars }` | la tête, puis un renvoi. S'applique dès l'arrivée |
| `Stale { max_chars, after_turns }` | bornée, et **périmée** : passé `after_turns`, une ligne |

`Stale` est celle qui paye, parce qu'elle vise **ce qui coûte** — l'ancien —
et pas ce qui est gros. Un résultat énorme qui vient d'arriver est
probablement celui dont le modèle a besoin maintenant.

Le premier test du module est `le_defaut_ne_touche_a_rien`, et il est premier
exprès : *exister ne change rien*. Aucun agent ne se comporte différemment du
seul fait que ce fichier est là.

### 2.2 La table de renvois, et l'outil qui les résout

Réduire l'historique sans rien perdre suppose que le contenu entier reste
adressable. Il est gardé **une fois** dans la session, sous un nom court et
stable : `#read-2`, numéroté par outil.

Ce qui rend le renvoi utile n'est pas l'étiquette, c'est `recall` — et le
doc 13 §5 le disait déjà : *sans l'outil, ce sont des étiquettes
décoratives*. `SessionTools` enveloppe une boîte existante et **ajoute** un
outil, sans en retirer aucun :

```rust
let tools = SessionTools::new(&inner, session.clone());
let agent = Agent::new(&llm, &tools).with_session(session.clone());
```

L'enveloppement est **explicite**, et c'est un choix : la boucle n'ajoute pas
un outil dans le dos de l'appelant. Absorber sans `recall` reste correct — le
contenu est toujours dans la session — mais devient une perte pour le modèle,
qui voit un renvoi qu'il ne peut pas suivre. Le doc-comment de `with_session`
le dit à cet endroit précis, parce que c'est là qu'on peut se tromper.

### 2.3 Idempotent, et daté

Deux propriétés qui n'ont l'air de rien et sans lesquelles ça ne marche pas :

- **Chaque forme est dérivée du contenu entier mémorisé**, jamais de ce qui
  est actuellement dans l'historique. Sans ça, chaque passage tronquerait la
  troncature et l'historique fondrait tour après tour
  (`absorber_deux_fois_ne_ronge_pas`).
- **Un résultat garde le tour où il est arrivé.** Sans ça, il rajeunirait à
  chaque passage et ne périmerait jamais
  (`un_resultat_garde_le_tour_ou_il_est_arrive`).

Les deux sont des tests, pas des commentaires.

### 2.4 Le bloc d'attentes, enfin injecté

`Postures::describe_for` existait depuis hier et **personne ne l'appelait** :
la matière était calculable, il n'y avait pas d'endroit où la mettre.
`Agent::refresh_waiting_block` est cet endroit. Trois choix, chacun payé :

- **dérivé, donc jamais périmé** — la pause tombe, la ligne s'en va, et
  personne n'a à penser à nettoyer ;
- **vide quand il n'y a rien** — un bloc toujours présent apprend au modèle à
  ne plus le lire ;
- **en dernier, pas en tête** — il change à chaque tour, et le mettre au début
  invaliderait le préfixe mis en cache par le fournisseur. On paierait la
  visibilité au prix de tout l'historique.

Le troisième point ne figurait pas dans le doc 12 : il est apparu en
l'écrivant. C'est un vrai arbitrage, et il va dans le sens contraire de
l'intuition (« ce qui est important se met en haut »).

Il est **remplacé, jamais empilé** — `le_bloc_d_attentes_ne_s_empile_pas` :
dix tours d'attente ne font pas dix blocs.

## 3. Le chiffre

Le doc 13 §9.4 demandait exactement ce test : *une conversation de dix tours
dont on mesure les jetons, avec et sans*. Il existe, il tourne à chaque
`cargo test`, et il imprime son résultat :

```
[dix tours] sans absorption 900180 caractères, avec 37567
```

**Facteur 24**, avec `Stale { max_chars: 2_000, after_turns: 2 }`, sur neuf
appels rendant vingt mille caractères chacun.

Ce que le test **fixe**, ce n'est pas ce ratio — il dépend entièrement de la
taille des résultats, et un agent qui lit des fichiers de trois lignes ne
gagnera rien. C'est la **forme** : le témoin est quadratique, l'absorbé ne
l'est pas, et rien n'est perdu — la dernière assertion du test vérifie que
`recall("#read-1")` rend bien vingt mille caractères.

C'est aussi pour ça que le témoin est dans le même test : un chiffre seul ne
dit rien, deux chiffres mesurés dans la même minute disent tout.

## 4. Ce qui pourrait se discuter

Trois choses, dites franchement.

1. **Le bloc d'attentes est un tour `system` en fin d'historique.** Certains
   fournisseurs n'aiment pas un message système en dernière position. Aucun de
   ceux qu'on utilise ne s'en plaint aujourd'hui, mais c'est le premier
   endroit à regarder si un fournisseur refuse une requête. L'alternative —
   un tour `user`, comme le fait déjà `final_nudge` — mentirait sur qui parle.
2. **`absorb` réécrit `turns` en place.** L'appelant qui garde son historique
   après le run récupère la forme réduite, pas l'originale. C'est voulu — c'est
   ce qui rend la réduction persistante d'un run à l'autre — mais ça surprend
   si on ne l'attend pas. Le contenu entier reste dans la session.
3. **La numérotation `#read-2` est par outil et par session.** Deux sessions
   ont chacune leur `#read-1`. Tant que la table vit dans la session, c'est
   cohérent ; le jour où un renvoi doit traverser un run, il lui faudra le
   préfixe du run.

## 5. Ce qui reste du doc 13

| Étape (doc 13 §9) | État |
|---|---|
| 1. Rendu (liens, hiérarchie, regroupement) | fait le 26 |
| 2. Handles + outil qui les résout | **fait ici** (`recall`) |
| 3. Entité `Turn` + `IN_RUN` | pas fait — `Conversation`/`Message` existent, pas `Turn` |
| 4. `absorb` en nœud, `compact.mmd` | **la politique est faite**, le nœud non |
| 5. `assemble`, `stop`, `grounded.mmd` | `assemble` fait pour le bloc d'attentes ; le reste non |

La différence entre « la politique est faite » et « le nœud est fait » est
réelle : ici `absorb` est une méthode appelée par la boucle, pas un nœud d'un
graphe de session que l'appelant compose. Le doc 13 §8 disait de toute façon
que `Agent::run` ne devait pas disparaître — c'est le chemin simple, et il
vient de gagner un facteur 24 sans cesser d'être simple. Le graphe reste
justifié le jour où quelqu'un voudra *une autre* politique d'assemblage ; il
ne l'était pas pour celle-ci.

**Ce qui manque encore et qui vaut de lui-même** : l'entité `Turn` liée
`IN_RUN` (étape 3), qui rend les conversations cherchables au tour près —
« l'agent retrouve ce qu'il a dit au tour 2 ». La table de renvois de ce
document en est la moitié en mémoire ; l'autre moitié est sa persistance.
