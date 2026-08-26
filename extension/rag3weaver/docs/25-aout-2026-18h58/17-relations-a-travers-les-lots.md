# 17 — Les relations à travers les lots : résoudre sans dépendre de l'ordre

26 août 2026, 10h. Question de Lucie : *« les relations se reconstruisent
comment quand codeparsers lit un fichier, puis un autre fichier qui le
référence ? Possible ou pas ? Sinon truc qui devrait dans tous les cas
marcher : j'indexe un dossier, puis j'indexe un nouveau fichier du dossier,
les relations doivent se créer. »*

## 1. L'état des lieux, sans fard

**Non, pas aujourd'hui.** `codeparsers` résout **à l'intérieur d'un lot** :
`analyze(root, sources)` appelle `parse_project` avec `resolve_relationships`,
et le résolveur ne connaît que les fichiers qu'on lui donne. Une référence
vers un symbole défini ailleurs n'a pas d'extrémité :

```rust
let (Some(from), Some(to)) = (identity.get(&r.from_uuid), identity.get(&r.to_uuid))
else { analysis.relations_dropped += 1; continue };   // src/code.rs
```

Trois conséquences mesurables :

| Scénario | Ce qui se passe |
|---|---|
| Ingérer A, puis B qui référence A | la référence de B vers A est **perdue** |
| Ingérer un dossier, puis un fichier neuf du dossier | perdue **dans les deux sens** : ses références vers l'existant, et les références de l'existant vers ses définitions |
| `edit` puis `reingest_file` | le fichier est reparsé **seul** : toutes ses relations inter-fichiers tombent |

Et le compteur existe déjà — `relations_dropped` — mais personne n'en fait
rien. **Une perte comptée et non réparée, c'est une dette qui se sait.**

## 2. Pourquoi c'est structurel

Le résolveur est un **passage unique sur un ensemble clos**. Il construit sa
table de symboles à partir des sources reçues, résout, jette le reste. Rien
là-dedans ne survit au lot : une référence non résolue n'est pas *stockée*,
elle est *comptée*.

Or l'ingestion, elle, est **incrémentale par nature** — c'est tout l'intérêt
de `edit`, du `FileSource`, de la mémoire à TTL du
[16](16-le-monde-est-ouvert.md). Le résolveur suppose un monde clos ; le
reste du système suppose un monde qui grandit. C'est là que ça casse.

## 3. Le principe

> **Une référence non résolue est une donnée, pas un échec.**

Si elle est stockée, l'ordre d'arrivée cesse d'avoir de l'importance : la
résolution devient une opération qu'on peut refaire, dans n'importe quel
sens, à n'importe quel moment.

## 4. La couche `Symbol`

Une entité de plus, invisible pour l'agent, avec `hashsafe = ["name"]` —
donc un uuid déterministe, obtenu **sans requête** (`Catalog::entity_uuid`) :

```
Scope ──DEFINES──▶ Symbol{name}          (ce que ce scope offre)
Scope ──MENTIONS─▶ Symbol{name}          (ce qu'il attend, en attente)
```

À chaque ingestion d'un lot, deux passes, toutes deux locales :

1. **Ce que le lot attend** : pour chaque référence non résolue dans le lot,
   `MENTIONS` vers `Symbol(nom)`. Si ce symbole a déjà un `DEFINES`, on
   matérialise `CONSUMES` / `CONSUMED_BY` tout de suite.
2. **Ce que le lot offre** : pour chaque scope défini, `DEFINES` vers
   `Symbol(nom)`, puis on relit les `MENTIONS` entrants de ce symbole — ce
   sont les scopes qui attendaient, parfois ingérés il y a des jours — et on
   matérialise.

Le cas de Lucie tombe alors tout seul, **dans les deux sens** : le fichier
neuf trouve ce qui existait (passe 1), et l'existant trouve ce que le
fichier neuf apporte (passe 2). Aucune ré-analyse du dossier.

**On garde `CONSUMES` matérialisé** plutôt que de le dériver en deux sauts à
chaque recherche : `search_expand` traverse déjà cette relation, et une
requête d'agent ne doit pas payer la dette d'ingestion.

## 5. L'ambiguïté, qui est le vrai risque

Un nom global n'est pas une identité : `len`, `new`, `execute` sont définis
cent fois. Une résolution par nom, seule, **sur-relierait** massivement — et
on a déjà payé ce prix une fois (47 relations par scope avant les options du
résolveur, doc 04 du 25 août).

Trois garde-fous, dans cet ordre :

1. **Le résolveur du lot reste la voie précise.** Il connaît les imports, la
   portée, le fichier. La couche `Symbol` ne sert qu'à ce qu'il a dû
   abandonner — les références **inter-lots**.
2. **Un seul définisseur, sinon rien.** Si `Symbol(nom)` a plusieurs
   `DEFINES`, on ne matérialise pas : on laisse le `MENTIONS`, et on
   compte. Mieux vaut une relation manquante qu'une relation fausse — un
   agent qui suit `CONSUMED_BY` doit pouvoir croire ce qu'il lit.
3. **Le chemin d'import quand il existe.** Une référence qui vient d'un
   `use a::b::c` porte son chemin ; il départage les homonymes. C'est
   l'affinage qui viendra après, pas le premier jet.

## 6. L'oracle : la ré-résolution complète

Pour savoir si l'incrémental est juste, il faut un témoin : **ré-analyser
tout le projet en un lot** donne le graphe de référence. Sur notre propre
module : 27 fichiers, 1 373 scopes, 12 418 relations, 1,2 s de parsing.

C'est aussi le **repli** honnête : une commande explicite (« reconstruis
les relations ») pour les cas où l'incrémental a dérivé, et le mode par
défaut pour une première ingestion.

Le test qui tranche, et c'est celui que Lucie a posé :

> Ingérer le dossier en un lot, noter les relations. Recommencer en ingérant
> les fichiers **un par un, dans un ordre quelconque**. Les deux graphes
> doivent être **identiques**.

S'ils le sont, l'ordre n'a plus d'importance — ce qui est exactement la
propriété qu'on cherche.

## 7. L'ordre, et ce que ça coûte

1. **Stocker les références non résolues** (`Symbol`, `DEFINES`,
   `MENTIONS`) et les compter par nom plutôt que globalement. *Test* :
   `relations_dropped` tombe à zéro, remplacé par des `MENTIONS` en attente.
2. **Les deux passes de matérialisation**, avec la règle du définisseur
   unique. *Test* : A puis B, et B puis A, donnent le même graphe.
3. **Le test d'équivalence** avec la ré-résolution complète, sur notre
   propre module. *Test* : fichier par fichier = tout d'un coup.
4. **`reingest_file` passe par là** : après un `edit`, les relations
   inter-fichiers du fichier modifié se refont — aujourd'hui elles
   disparaissent en silence.
5. **Le chemin d'import** pour départager les homonymes, si la mesure de
   l'étape 3 montre qu'il en manque trop.

Coût à l'ingestion : deux liens de plus par référence et par définition,
et une lecture de voisins par symbole défini. À l'échelle de notre module,
quelques milliers d'arêtes de plus — invisible à côté des 12 418 déjà là.
Coût à la recherche : **zéro**, puisque `CONSUMES` reste matérialisé.
