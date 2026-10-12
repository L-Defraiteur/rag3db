# Deux nœuds liés par leur clé dans un même MATCH se croisent avant la jointure avec la liste

- **État** : ouvert (12 octobre 2026, 11 h 40 ; trouvé en écrivant le témoin du troisième pas de la jointure par clé, page coeur-cpp/14) — au cœur C++
- **Gravité** : coût (une instruction de quelques milliers de lignes épuise le tampon ou dure des minutes)
- **Atteignable en service** : en principe oui, par toute écriture `UNWIND … MATCH (a {clé: x}), (b {clé: y}) CREATE (a)-[]->(b)` ; rag3weaver n'écrit pas ses relations ainsi (à vérifier par codeparsers)
- **Touche rag3weaver** : non connu

## Ce que c'est

`UNWIND range(0, 9999) AS i WITH i, 'd-' + CAST(i % 20000, 'STRING') AS kd MATCH (m:M {id: i}),
(d:D {uuid: kd}) CREATE (m)-[:R]->(d)` sur 60 000 M et 20 000 D : « Buffer manager exception: the
buffer pool is full » sous 512 Mo, des minutes sinon — avant comme après le troisième pas, et avec
le second niveau seul. Le planificateur planifie les deux nœuds (deux composantes du motif) par un
produit cartésien `m × d`, puis joint le tout à la liste par les deux clés ; les masques par clés
réduisent bien `m` et `d` à leurs 10 000 lignes chacun, mais le produit des deux ensembles masqués
(10⁸ paires) se construit avant la jointure, qui n'en garde que 10 000. La même écriture en deux
MATCH (`MATCH (m {id: i}) WITH m, kd MATCH (d {uuid: kd}) CREATE …`) passe en une seconde.

## La forme

Quand deux composantes d'un motif sont chacune liées par leur clé à la partie précédente, les
joindre l'une après l'autre à cette partie (deux jointures par clé) au lieu de les croiser d'abord —
dans `planRegularMatch` / `planQueryGraphCollection`, en présence de `keyBoundNodeIDs` dans plus
d'une composante. Témoin : la forme ci-dessus sous 512 Mo doit passer et le plan ne doit porter
aucun CROSS_PRODUCT entre deux balayages masqués.

**Portée produit** (F, 12 octobre, 12 h) : le dialecte rag3weaver lie déjà un nœud par MATCH
(`unwind_par_cle` écrit `UNWIND $items AS item WITH item, item.from AS __cle_0 MATCH (a {_uuid:
__cle_0}) WITH a, item, item.to AS __cle_1 MATCH (b {_uuid: __cle_1})`, texte fixé par les tests de
dialect.rs) ; aucune requête de src/ ne lie deux nœuds par leur clé dans un même MATCH. Le produit
ne touche pas l'indexation : défaut du moteur sur une forme écrite à la main.
