# Le lot suivant, par famille

Préparé en lecture seule la nuit du 19 au 20 novembre, à la demande de c7 : les trois familles qu'il nomme, chacune avec
ses tickets, son état, le témoin rouge à écrire, puis l'ordre que je propose. Aucun bâti avant le réveil des hôtes sur
la nouvelle lib.

## 1. Le plein texte hors du point de reprise

**Ticket** : `docs/tickets/2026-10-10-index-plein-texte-en-fichiers-hors-du-point-de-reprise.md`.

**État** : décision de Lucie du 10 octobre, « on fera plutôt comme Neo4j pour l'instant » ; « pas sur la stèle, chantier
du moteur à cadrer quand une marche le demande ». Rien n'est commencé. L'index lucivy vit dans `<base>.fts/`, hors de
la transaction : une annulation, un point de reprise ou un rejeu couvrent la base, pas l'index ; rag3weaver répare
après coup (état d'index, rebâti).

**La part du moteur** :
1. le point de reprise connaît les fichiers d'index qu'une extension déclare, et en prend l'instantané dans le même
   geste que le magasin ;
2. le rejeu les ramène au même point ;
3. l'annulation d'une transaction rend l'index à son état d'avant, ou en borne le rebâti au delta.

**Le témoin rouge à écrire** : en C++, une extension de test qui déclare un fichier d'index, une mort entre le point de
reprise du magasin et celui de l'index, puis la réouverture, qui doit trouver les deux au même point. Côté produit, le
cas « tués » d'`e2e_arret_brutal`.

**Taille** : la plus grande des trois, plusieurs jours, et à cheval sur trois sessions (moi pour le point de reprise,
l'arbre principal pour l'état d'index, lucivy pour l'annulation par shard). **Préalable** : une page de cadrage
courte, avec le cœur et l'arbre principal, comme le ticket le demande. Je ne la commencerais pas sans ton mot : elle
engage le format (un instantané d'index au point de reprise).

## 2. La rétention des index hors de l'index vectoriel (geo)

**Tickets** :
- `docs/tickets/2026-10-18-la-retention-des-index-ne-couvre-que-l-index-vectoriel.md` (la part R2) ;
- `docs/tickets/2026-10-05-rtree-recoit-des-decalages-provisoires.md`.

**État** : la part commune (R1) est faite depuis le 19 octobre : l'accès par nom prend sa transaction, et la capture
d'une recherche est commune à tous les genres d'index. Ce qui reste est propre à geo, et **injoignable** : l'index
spatial ne se crée pas (« Trying to create nested type LIST without child information », faute d'`inferInputTypes`),
et `QUERY_SPATIAL_INDEX` lie une liste de colonnes vide. L'orchestration l'a laissé « pour quand geo servira » (10 et
19 octobre, 7 novembre).

**Le témoin rouge à écrire**, dans l'ordre :
1. créer un index spatial, chercher, trouver le bon point (rouge aujourd'hui, au CREATE) ;
2. une fois l'index créable, une recherche ouverte avant un DROP validé, qui doit lire l'index ou le refuser par un nom,
   jamais rendre zéro ligne ;
3. `BEGIN`, une insertion, `ROLLBACK`, puis une recherche qui couvre le point (les décalages provisoires).

**Taille** : rendre geo utilisable est un chantier de produit (types des paramètres, une sortie `node`/`distance`, le
R-tree par `commitInsert` avec son crochet d'annulation), puis R2. **Préalable** : la décision de servir geo. Personne
ne s'en sert aujourd'hui, rag3weaver compris.

## 3. Le COPY de relations sans remappage

**Déjà fait** : `docs/tickets/closed/2026-10-05-copy-de-relations-vers-des-noeuds-locaux-sans-remappage.md`, fermé par
`c4f8610b9` (lot G1, 7 novembre).

**Le voisin encore ouvert** : `docs/tickets/2026-10-05-copy-de-relations-annule-gonfle-l-estimation.md`. Un COPY de
relations annulé, ou un DELETE, laisse gonflée l'estimation du nombre de relations (`nextRelOffset`, qui est aussi
l'allocateur des identités). Aucune réponse fausse, des plans peut-être moins bons. L'orchestration : « rien à coder
tant qu'un mauvais plan n'est pas observé ». Témoin rouge prêt à écrire le jour venu : l'estimation lue par un accès de
test après un COPY de relations annulé, puis après un DELETE.

## L'ordre que je propose

Les trois familles attendent chacune une décision (le cadrage du point de reprise, servir geo, un mauvais plan
observé). Pour ne pas rester sans lot, je propose d'avancer d'abord ce qui n'en demande pas :

1. **Fermer en docs le ticket du nombre de lignes hors du graphe**
   (`2026-11-08-le-nombre-de-lignes-hors-du-graphe-…`). Son lot (a) est sur master depuis le 18 au soir. Il lui manque
   la ligne promise dans le ticket du 18 octobre : le produit peut décider la reconstruction au seuil.
2. **Le ticket du banc sur les points les plus isolés** (`docs/banc/tickets/2026-11-19-les-points-les-plus-isoles-…`),
   à chiffrer d'abord sur des vecteurs réels (une base de code), avec un témoin de points ordinaires à côté. Lecture et
   mesure avant tout code.
3. **Le cadrage du plein texte au point de reprise** (famille 1), page courte avec le cœur et l'arbre principal, si tu
   le veux maintenant ; c'est le seul des trois qui touche le produit chaque jour.
4. **geo** (famille 2) et **l'estimation des relations** (famille 3) : rien tant que leur décision n'est pas prise ;
   les témoins rouges sont écrits ci-dessus, prêts.

Restent ouverts à mon nom, hors de ces familles : l'ouverture sans une extension (le refus avec option, à trancher par
l'orchestration), les morceaux de Scope à plusieurs arêtes (fermé par le retrait de l'ancien index), les traces de
`run_e2e.sh` qui font rougir la garde de retrait, et le mode pile de la garde (pour G).
