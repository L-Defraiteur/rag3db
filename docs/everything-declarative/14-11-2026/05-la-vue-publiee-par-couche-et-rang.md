# La vue publiée par (couche, rang) : ce qu'une recherche admet, posé une fois

2a′-2, au mot de c7 du 14 novembre 2026 au soir, après les mesures de la page 04 (§ 2 bis) et des
tickets du même soir. **Ce qui est à corriger** : sur une base à versions, chaque recherche
recalcule le masque des versions, puis l'écrit **en liste** :
- en `_uuid IN [72 652]`, résolu par balayage pour le lexical (5 s) ;
- **en clair** dans le Cypher de la projection du dense (2,9 Mo, 17 s par appel). Chaque analyse
  de ce Cypher fait un pic de 3,6 Gio (le cœur : environ 50 Kio par élément, rendu ensuite, mais
  les arènes de glibc gardent la pente d'un fil à l'autre).

## 1. Le masque n'est pas une liste : c'est un prédicat

Une version est admise si au moins une de ses occurrences est visible au rang lu. Une occurrence
visible, c'est la clause de 2a′-1 sur `Scope`. Le masque s'écrit donc en **jointure et prédicat**,
sans énumérer personne :

```
MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(c:ScopeContent)
      -[:ScopeContent_DERIVED_FROM]->(s:Scope)
WHERE <visible(s, couche, rang)> [AND <filtre de l'utilisateur sur s>]
RETURN DISTINCT n
```

Ce Cypher fait quelques centaines d'octets, quelle que soit la base. C'est la forme de R filtré,
dont la projection courte ne fuit pas (page 04, ticket de la projection) : 20 projections, mémoire
plate.

## 2. Ce qui se pose une fois par (couche, rang)

**Le lexical : l'ensemble des offsets admis.**
- Les offsets de `ScopeContent` dont une occurrence est visible au rang lu. C'est une requête de
  prédicat, jouée **à la publication** de la génération (et à l'ouverture pour le rang courant),
  pas à chaque recherche.
- Il est gardé en mémoire (`Arc<[u64]>`, 72 652 offsets, soit environ 580 Kio) et passé à lucivy
  en `allowed_ids`. lucivy n'y ajoute que 6 à 10 ms par requête : sa vue nommée (`define_view`)
  n'est pas nécessaire pour l'instant.
- Il est retiré quand la génération suivante est publiée et que plus aucune lecture ne cite
  l'ancienne.

**Le dense : la projection courte, par prédicat.**
- Chaque recherche dense filtrée projette le Cypher du § 1, sans liste.
- **Pas de graphe posé une fois** (réponse du cœur, lue dans le code, 14 novembre 2026, 21 h) :
  - le graphe projeté appartient à la **connexion** (`GraphEntrySet`, membre du `ClientContext`) ;
    posé sur la connexion du catalogue, il est invisible aux connexions de lecture et du pool ;
  - il ne survit pas à sa connexion, et n'est jamais écrit sur disque ;
  - son masque se **recalcule à chaque** `QUERY_VECTOR_INDEX` : l'entrée ne garde que le texte du
    Cypher, que la requête replanifie.

  Le poser une fois n'économiserait donc que l'analyse d'un texte de 150 octets. **La projection
  courte par requête reste la forme du dense.**

## 3. Le filtre de l'utilisateur se compose

- **Lexical** : les offsets du filtre (une requête courte, comme sur R) intersectés avec
  l'ensemble posé de (couche, rang). Ou bien une seule requête de prédicat
  « visible ET filtre ».
- **Dense** : le prédicat de l'utilisateur ajouté au `WHERE` du § 1, toujours une projection
  courte.
- **Sans filtre ni vue** : c'est le court-circuit d'arbre principal (lot de 1a) ; rien n'est
  projeté. Sous 2a′-2, son test devient « aucune version sans occurrence visible au rang lu »
  (page 04, § 2 bis).

## 4. La règle : jamais une grande liste en clair dans un Cypher

- `inline_params` (`src/search.rs`) **refuse une liste de plus de N éléments**, avec N de l'ordre
  du millier. L'appel échoue en le disant ; rien n'est tronqué en silence. Un témoin l'éprouve.
- `_uuid IN liste` passé en paramètre reste permis, mais s'écrit en `UNWIND` et accès par clé
  (×44). C'est le lot de recherche, et le ticket moteur du planificateur.

## 5. Les témoins, rouges d'abord

1. **Aucune projection n'écrit de liste** : sur une base à versions, une recherche dense filtrée par
   chemin ne projette qu'un Cypher de moins de 4 Kio. On l'observe par un compteur des projections
   et de leur taille dans `Rag3dbSearchBackend`, lisible par les tests. C'est rouge aujourd'hui :
   2,9 Mo.
2. **La mémoire reste plate** : dix recherches denses filtrées sur une copie de la base migrée, RSS
   plat à quelques dizaines de Mio près. C'est une sonde sous `poste mesure` (`sonde_vecteur_filtre`,
   `sonde_recherche_sans_filtre`), et non un test de la passe. Rouge aujourd'hui : +3,7 Gio par
   appel.
3. **L'ensemble lexical se pose une fois** : deux recherches lexicales filtrées sous la même
   génération ne résolvent l'ensemble qu'une fois. Compteur de résolutions ; rouge aujourd'hui :
   une par signal et par requête.
4. **Les réponses ne changent pas** : le juge de banc, régime « optimisation », au bit près contre
   le relevé de la base migrée, sur les trois formes.
5. **`inline_params` refuse une grande liste** : une liste de 10 000 éléments rend une erreur nommée.

## 6. L'ordre

1. ~~La question de la vie d'un graphe projeté va au cœur.~~ Répondue : par connexion, recalculé à
   chaque requête. La projection courte reste.
2. Les témoins 1, 3 et 5, rouges.
3. Le masque par prédicat, le dense puis le lexical, et la règle d'`inline_params`.
4. Les témoins verts, la parité de banc, les sondes 2 sous `poste mesure`.
