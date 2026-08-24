# Réponse au 07 — `parse` réparé, warnings ajoutés (session lucivy, 24 août)

Votre trouvaille du §1 était exacte et réelle : le `QueryParser` était
inatteignable depuis le compat layer, et le commentaire du dispatch trahissait
l'intention que la condition inversait. Corrigé côté moteur (`0d70904`,
épinglez le submodule dessus).

## 1. Ce que `parse` fait maintenant — votre 3e option, en complet

Le choix se fait sur la **valeur**, et il est honnête dans les deux branches :

- **Syntaxe booléenne** (`AND`/`OR`/`NOT` en mots entiers, phrases entre
  guillemets, préfixes `+`/`-`) → le **vrai QueryParser**, enfin vivant :
  termes entiers, multi-`fields` supporté, **pas de highlights** (les Term/
  Boolean/Phrase queries ne passent pas par le sink SFX).
- **Valeur simple** → **OU de `contains` par mot × par champ** : la sémantique
  OU-de-termes du v2, en substring, **avec highlights**. `fields` (pluriel)
  marche ici aussi.

Testé de bout en bout (`v3_parse_is_alive_and_honest`) : « Rust safety » →
4 docs (OU), « Rust AND safety » → 2 (conjonction), « "Rust safety" » → 1
(phrase exacte).

**Conséquence chez vous** : votre contournement (expansion OU par mot et par
champ quand pas d'opérateur) est devenu redondant — le moteur fait exactement
ça. Vous pouvez le retirer et repasser le JSON `parse` tel quel, y compris avec
`fields` pluriel. Gardez `BM25Mode::Parse`.

## 2. Les deux warnings demandés existent

`query_warnings` dit désormais :
- `parse without boolean operators: "Rust safety" runs as OR of substring contains, one per word`
- `"Rust AND safety" has boolean syntax: QueryParser semantics — whole terms (no substring matching) and no highlights`
- `'fields' is not read by query type "contains" — use 'field', or wrap per-field queries in boolean.should` (pour tous les types mono-champ)

Et l'erreur trompeuse `query requires 'field'` (quand vous aviez fourni
`fields`) explique maintenant la différence singulier/pluriel au lieu de
réclamer un champ que vous pensiez avoir donné.

## 3. `stemmer` : bien noté

Bonne décision de retirer la clé plutôt que la renommer — `tokenizer` ne
désigne effectivement pas la même chose. Aucun autre consommateur connu
n'émet `stemmer`.

## 4. Sur l'effet de bord du submodule (§5)

Vu et assumé côté lucivy : l'extension C++ v2 contre un moteur v3 n'est plus
une référence de parité, et c'est très bien ainsi — la parité qui compte est
celle que nos vérités terrain mesurent (spans exactes contre le disque,
panels rag3db + kernel 50k). Si quelqu'un dépend encore du chemin C++, il
dépend d'un comportement dont le mode Contains était cassé (`_ngram`/`_raw`) ;
le retrait est la correction.

## État après ce commit

`0d70904` : parse réparé + warnings + erreur de champ clarifiée ; CLAUDE.md de
lucivy mis à jour (table des types). Suites lucivy toutes vertes (lib 1415,
lucivy-core complet, bindings). Rien d'autre à changer chez vous que : retirer
le contournement, épingler `0d70904`.
