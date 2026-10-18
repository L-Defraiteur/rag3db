# Une fiche qui vise un symbole de code, et les fiches d'un fichier

*Étude légère du 18 octobre 2026 (mémoire longue), pour l'explorateur de code du proto.
Lucie veut qu'une fiche puisse viser un symbole de code, et qu'en ouvrant un fichier ou
un symbole on voie les fiches qui en parlent, les plus récentes d'abord. Rien n'est
codé ici : la page dit ce qui existe, ce qui manque, et ce que ça demande à l'hôte.*

## 0. Ce qui existe aujourd'hui

- **Les références typées** du gabarit `memory` : `Ref {genre, valeur, dit_comme,
  empreinte, etat}`, identité `hashsafe [genre, valeur]`, reliée par `CITE: Memory → Ref`
  (`templates/backends/memory/backend.json:149-232`, `:308-311`). Les genres sont des
  lignes `RefType {name, description, forme, script, etat}` (`:233-286`) ; **aucun genre
  n'est livré** (ni `path`, ni `url`, ni `row:`).
- **Le harnais d'un genre** est une expression rhai sur `input.value` qui rend
  `{ok, why, normal, fingerprint}` (`src/dataflow/ref_nodes.rs:99-155`). Il normalise,
  mais **ne lit rien** : l'hôte ne lui prête que `json_string` et `content_hash`, sans
  entrée-sortie ; `resolve` « n'est pas faisable du tout aujourd'hui » (`:115-119`).
- **Le cycle de vie d'une Ref** (`vue → a_revoir → vue`, `→ introuvable`, `→ retiree`)
  est déclaré, mais **rien ne le déclenche** : aucune réaction ne regarde les Refs.
- **Côté code** (`src/code.rs`) : `Symbol` n'est qu'un **nom** (`hashsafe [name]`, pas de
  fichier, pas de genre) — un point de rencontre, pas une cible. La vraie définition est
  le **`Scope`** : `file_path` relatif à la racine en `/` (règle B4, 6e3561268),
  `parent_name`, `scope_type`, lignes, `signature`, une clé `key =
  blake3(file:name:type:signature)` stable quand les lignes bougent, et `_content_hash`
  (blake3 du contenu). Les outils `usages`, `impact`, `callees` prennent un `name` et un
  préfixe `path`, pas d'identifiant.
- **Aucune lecture inverse** : rien ne liste les fiches qui citent une valeur
  (`schemas/ref.json:7` l'annonce seulement). Les dates de `Memory` : `created_at`,
  `updated_at`, `reviewed_at`.
- **Aucun pont entre backends** dans l'hôte : un graphe ne reçoit que les services de
  son propre backend (`src/backend.rs:2999-3075`) ; la §12.4 de
  `docs/13-octobre-2026-arbre-principal/01-plusieurs-developpeurs-et-leurs-migrations.md`
  en est la graine (`_Pont`, `entre_bases`, la poursuite d'un saut par l'hôte).

## 1. Deux genres, `path` et `symbol`, une seule forme normale

La forme normale est choisie pour que **le préfixe dise « dessous »** :

| genre | valeur normale | exemple |
|---|---|---|
| `path` | chemin relatif à la racine du dépôt, en `/` (la règle B4), sans `./` ni `/` final ; un dossier finit sans `/` | `extension/rag3weaver/src/daemon_lifecycle.rs` |
| `symbol` | `<chemin>#<parent>::<nom>`, le parent omis au niveau du fichier | `extension/rag3weaver/src/daemon_lifecycle.rs#new_token`, `…/backend.rs#PreparedBackend::open` |

- Le **chemin d'abord** : « les fiches d'un fichier » est un préfixe `<chemin>` (le
  fichier et ses symboles), « d'un dossier » un préfixe `<dossier>/`. Une seule
  requête, un seul index.
- **Pas la clé `key` comme valeur** : elle est illisible, et elle change quand la
  signature change ; une fiche doit rester lisible et survivre à un changement de
  signature (elle passe alors « à revoir », elle ne se perd pas).
- **Pas le seul nom** (`Symbol`) : deux `new` dans deux fichiers sont deux cibles.
- Le **harnais de normalisation** tient dans le rhai d'aujourd'hui : remplacer `\` par
  `/`, retirer `./` et le `/` final, refuser un chemin absolu (comme B4 refuse une base
  d'avant la règle), découper `#` et `::`. Il se pose par `create_ref_type` avec un
  `exemple`, sans rien coder.

**L'empreinte qui fait vieillir** :

- `symbol` : le **`_content_hash` du `Scope`** visé — il change quand le corps change.
  Un deuxième niveau possible : la `key` (elle change avec la signature), pour dire
  « la signature a changé » plus fort que « le corps a changé ».
- `path` : le `content_hash` du `File` ; un dossier n'a pas d'empreinte (il ne vieillit
  pas, il se lit par ses fichiers).

**Les états** : le cycle déclaré suffit.

- empreinte égale → `vue` ;
- empreinte différente → `a_revoir` (la fiche dit encore peut-être vrai, mais son objet
  a bougé) ;
- cible absente → `introuvable` ; retrouvée par son nom dans un autre fichier →
  `retrouver` (→ `a_revoir`), le nouveau chemin proposé, jamais posé seul.

Calculer ces empreintes **demande de lire la base de code** : c'est ce qui manque au
harnais (§0), et c'est le §3.

## 2. La lecture inverse : « les fiches qui citent ce chemin, ce qui est dessous, ou ce symbole »

Un outil de la mémoire, **sans pont** : c'est une lecture de la seule base de la mémoire.

```text
cited_by(value, genre?, under = true, states?, kinds?, since?, limit = 20)
  MATCH (m:Memory)-[:CITE]->(r:Ref)
  WHERE (r.valeur = $value OR ($under AND (starts_with(r.valeur, $value + '/')
                                           OR starts_with(r.valeur, $value + '#'))))
        [AND r.genre IN $genre] [AND m.state IN $states] [AND m.kind IN $kinds]
        [AND m.updated_at >= $since]
  RETURN m, collect(r) ORDER BY coalesce(m.updated_at, m.created_at) DESC LIMIT $limit
```

- Il rend, par fiche, la ou les Refs citées et **leur état** : l'explorateur peut
  montrer « à revoir » à côté d'une fiche dont le symbole a changé.
- Le préfixe porte sur la valeur **normale** : le `#` et le `/` du §1 font le reste.
- Le tri par `updated_at` puis `created_at` ; `reviewed_at` reste un filtre possible
  (« à relire »).
- C'est un graphe de dataflow déclaré (`templates/tools/cited_by.mmd`), comme `recall`,
  rien de propre à la mémoire : la même lecture vaut pour tout gabarit qui a des Refs.
- Un index sur `Ref.valeur` (plein texte ou préfixe) est à mesurer quand le nombre de
  Refs le demandera ; aujourd'hui la mémoire de Lucie en a une.

## 3. Le pont : la mémoire et l'index de code sont deux bases

C'est **le premier cas réel** de la §12.4, et il est plus simple qu'un pont d'arêtes :
une Ref est une **valeur**, pas une arête vers l'autre base. Rien n'est écrit dans la
base de code ; on **lit** la base de code pour vérifier une Ref de la mémoire.

Deux façons de faire vieillir, à choisir :

- **À la lecture** (paresseuse, la plus petite) : `cited_by` et `add_ref` demandent à la
  base de code l'empreinte actuelle des cibles qu'ils rendent ou posent, et appliquent
  `revoir` / `perdre` à l'écart. L'explorateur voit l'état juste au moment où il ouvre le
  fichier ; une fiche jamais relue ne vieillit jamais, ce qui est sans coût.
- **Par réaction** (au fil de la montre) : quand la montre du code réindexe des scopes,
  l'hôte publie « ces chemins ont changé » ; une réaction de la mémoire passe les Refs
  qui les citent. Plus juste, mais il faut un événement qui traverse les backends.

Je propose **la lecture d'abord** : elle sert l'explorateur, ne demande pas d'événement
entre backends, et la réaction peut venir après sur le même service.

## 4. Ce que ça demande à l'hôte

1. **Un backend nommé, lisible par un autre** : la mémoire déclare dans son manifeste
   la base qu'elle lit (`reads: ["code-rag3db"]`, ou l'`entre_bases` de la §12.4 en
   lecture seule) ; l'hôte, qui tient déjà les deux backends dans la même
   `HashMap<PathBuf, Arc<Hosted>>`, prête au graphe de la mémoire un service
   « vue en lecture de `code-rag3db` » — par nom de manifeste, jamais par chemin donné
   par un appel. Un backend non déclaré, ou absent de l'hôte, est refusé en le disant.
2. **La vue, pas le verrou** : la lecture passe par la `read_view` de l'autre base, sans
   prendre son mutex — sinon un `cited_by` attendrait derrière une réindexation (le
   ticket « un appel d'outil attend derrière un index sans le dire », vu en service le
   12 octobre). Une base de code occupée rend « empreinte inconnue », jamais un faux
   `a_revoir`.
3. **Un harnais qui lit** : le rhai du genre reçoit, de ce service, une fonction
   `code_fingerprint(valeur)` → `{found, content_hash, key, file_path}` ; c'est le
   `resolve` que le harnais n'a pas (§0). Lecture seule, bornée, sans écriture dans
   l'autre base.
4. **La réponse dit qu'elle a traversé** : « 3 Refs vérifiées dans code-rag3db, 1 à
   revoir, 0 introuvable » ; et, si l'autre base n'est pas ouverte, « code-rag3db non
   ouvert : états non vérifiés » (le compte des bouts absents de la §12.4).
5. **Plus tard, la réaction** : un événement « scopes changés » publié par la montre du
   code, auquel la mémoire s'abonne ; même service, pas de nouvelle écriture croisée.

Ce que ça ne demande **pas** : aucun travail du moteur (pas d'`ATTACH`, pas de jointure
entre bases), aucune écriture dans la base de code, aucune arête qui traverse.

## 5. L'ordre proposé

1. Les deux genres `path` et `symbol` par `create_ref_type` (normalisation seule) et
   l'outil `cited_by` : **l'explorateur peut s'en servir tout de suite**, les états
   restant `vue`.
2. Le service de lecture d'un backend nommé dans l'hôte (§4.1-4.4), et
   `code_fingerprint` dans le harnais : les états deviennent justes à la lecture.
3. La réaction au fil de la montre (§4.5), si l'usage la demande.

Le 1 ne touche que des gabarits ; le 2 est le premier morceau du pont de la §12.4, en
lecture seule — à présenter à Lucie avec le lot P1 de la page des migrations, dont il
est la graine.
