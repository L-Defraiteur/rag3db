# Deux COPY annulés dans une transaction laissent les clés du premier dans l'index

- **État** : corrigé le 4 octobre 2026, commit `5c8507577` (« fix(stockage): l'annulation d'un COPY retire de l'index les clés de toutes ses lignes, et seulement elles — plus de clés fantômes après deux COPY annulés »). C'était une régression de `05788a868`, poussé deux heures plus tôt ; le code d'avant n'avait pas ce défaut.
- **Gravité** : blocage (la reprise d'un index après l'annulation refuse : « duplicated primary key »).
- **Atteignable en service** : non par défaut ; oui avec `RAG3WEAVER_TX_PAR_PAQUET=1` (premier index, gain 2), sur le chemin d'échec d'un paquet.
- **Touche rag3weaver** : oui (la transaction par paquet).

## Ce que c'est

Dans une transaction, deux COPY dans la même table de nœuds, puis `ROLLBACK`. Après la
fermeture de la base (point de reprise) et sa réouverture, les clés du **premier** COPY
restent dans l'index de clé primaire : aucune ligne ne les porte (balayage et recherche par
clé rendent 0), mais un `CREATE` de l'une d'elles est refusé, « Found duplicated primary key
value ». Les clés du dernier COPY sont bien retirées. Un seul COPY annulé, ou un COPY suivi
d'un MERGE, ne laissent rien.

## Recette

Moteur ≥ 05788a868 (rebâti le 4 octobre à 18 h 03), en Cypher brut par la connexion Rust,
sans rag3weaver :

```cypher
CREATE NODE TABLE T(k STRING PRIMARY KEY, v INT64);
COPY T(k) FROM 'a.csv' (header=false);      -- k10000 … k12499, validé
BEGIN TRANSACTION;
COPY T(k) FROM 'b1.csv' (header=false);     -- k100 … k109
COPY T(k) FROM 'b2.csv' (header=false);     -- k110 … k119
ROLLBACK;
-- fermer la base, la rouvrir
MATCH (t:T) RETURN count(t);                -- 2500 : juste
CREATE (:T {k: 'k100'});                    -- refusé : duplicated primary key (fantôme)
CREATE (:T {k: 'k115'});                    -- accepté
```

Variantes vérifiées sans défaut : un seul COPY annulé (petit ou de 5 000 lignes) ; COPY puis
MERGE d'une clé existante ou nouvelle.

## Témoin

- Sonde de recette : un fichier de test non commité de l'arbre principal, à reprendre dans la
  liste C++ du moteur.
- Côté produit : `un_paquet_qui_echoue_est_defait_et_la_reprise_rend_les_memes_comptes`
  (`e2e_tx_par_paquet_arret`), mis de côté dans
  `~/.cache/rag3weaver-build/test-rollback-en-attente.patch` tant que le moteur n'est pas
  corrigé. Il fait échouer le paquet 6 (quatre paquets par validation), et la reprise bute
  sur une clé de Symbol que les COPY des paquets 4 et 5, annulés, ont laissée.

## Pour le fermer

Que l'annulation retire de l'index de clé primaire les clés de tous les COPY de la
transaction, pas seulement du dernier. Puis appliquer le patch du témoin, le rejouer, et le
pousser avec la fermeture.

## Cause et correctif (session cœur C++, 4 octobre)

L'annulation relit des blocs de lignes entiers, pas la plage qu'elle annule, et annuler un
enregistrement rend invisibles toutes les lignes de son bloc ajoutées par la transaction
(`VectorVersionInfo::rollbackInsertions`) : le balayage des enregistrements suivants ne rend
plus rien. Le code d'origine retirait la clé de toute ligne relue, dès le premier
enregistrement — c'est ce qui lui faisait aussi retirer des clés de lignes validées.
`05788a868` ne retirait une clé que si elle menait à la plage de l'enregistrement ou
au-delà : les clés des COPY précédents n'étaient plus retirées par personne.

Le critère est maintenant : une clé sort de l'index si et seulement si elle ne mène pas à
une ligne validée. Témoin du moteur : `test/transaction/rolled_back_copies_test.cpp`, quatre
cas, rouges sur `05788a868`. Le témoin produit mis de côté (`e2e_tx_par_paquet_arret`) est à
rejouer par la session de l'arbre principal.
