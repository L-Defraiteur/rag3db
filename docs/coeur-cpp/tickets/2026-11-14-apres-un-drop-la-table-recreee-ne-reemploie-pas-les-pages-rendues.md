# Après un DROP, la table recréée ne réemploie pas les pages rendues

- **État** : ouvert (14 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : place perdue (le fichier grossit au lieu de réemployer les pages libérées)
- **Atteignable en service** : oui. Toute table supprimée puis recréée, ce que fait toute
  migration de schéma.
- **Touche rag3weaver** : par le moteur (la taille des bases)
- **Signalé par** : cœur c++ — e2e_test entier, en passant le lot du pliage des listes littérales
- **Pour** : cœur c++

## Ce qu'on voit

`transaction~ddl~ddl_empty.DropNodeTableReclaim`
(`test/test_files/transaction/ddl/ddl_empty.test`, l. 572) est **rouge sur master**. Le cas suit
ces étapes :

1. COPY de `movies`, puis CHECKPOINT ;
2. il compte les pages du fichier (`file_info()`) ;
3. DROP de `movies`, puis CHECKPOINT ;
4. il recrée `movies` et refait le même COPY et le même CHECKPOINT ;
5. il demande si le fichier n'a pas grossi. La réponse est `False` au lieu de `True`.

Joué seul à luciepc le 14 novembre :

| Commit | Ce qu'il contient | Tours |
|---|---|---|
| `00a772c79` | master `1000f5a74` + des tests seuls | rouge 3/3, `False` |
| `0cda1d12c` | le même, plus le pliage des listes littérales | rouge 3/3, `False` |

Le rouge est donc antérieur au pliage, et indépendant de lui. Dans e2e_test entier, joué avec
`--gtest_filter='*'`, c'est le seul rouge sur 1 868 tests. Lancé sans filtre, e2e_test n'inscrit
aucun test : `PASSED 0 tests` n'est pas un vert.

## Ce qu'il faut pour le fermer

- Trouver depuis quand il rougit : une bissection entre le dernier e2e_test entier vert connu et
  master. Les suspects sont les lots qui touchent l'allocation des pages : le retrait des
  générations V2-2, `retireStorage`, et les pages d'un COPY journalisé écrites au point de reprise.
- Puis la cause, le correctif, et le cas vert.
