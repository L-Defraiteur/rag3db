# Le pré-passage des ids vivants relit les postings

- **État** : ouvert (14 novembre 2026), souhait.
- **Gravité** : souhait. Une compaction de maintenance prend 12 % de plus depuis que la fusion lâche les ids sans
  document vivant ; l'indexation ne bouge pas.
- **Atteignable en service** : oui, à chaque fusion de segments qui portent des supprimés.
- **Touche rag3weaver** : non (lucivy seule).
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

Depuis la PR lucivy #37 (branche `dictionary-compaction`), `merge_segments_dict` (`src/indexer/sfx_dag_v3.rs`) ne
garde dans le `.gmap` fusionné que les ids globaux qui ont encore un document vivant : `live_ids_of` relit, avant la
fusion, les listes de documents des postings de fragments et de mots de chaque segment d'entrée. La fusion les relit
ensuite pour écrire les siens.

Mesuré à luciepc (5 cœurs) le 14 novembre 2026, main contre la branche, deux passes chacun, sur une copie de
`Lucivy_Scope` :

| | main | branche |
|---|---|---|
| indexation de 89 894 documents, commit tous les 5 000 | 105,4 / 99,5 s | 107,3 / 100,1 s (+0,6 à +1,8 %, sous le bruit) |
| compaction de 27 segments à 12 174 supprimés | 47,8 / 50,9 s | 55,6 / 55,3 s (**+12 %**) |

Le surcoût ne touche que les fusions de segments qui portent des supprimés.

## Ce qu'on pourrait faire

- (a) Sur un segment sans supprimé, juger un id de fragment vivant par `doc_freq`, sans décoder sa liste. Il reste
  à décoder les mots, et les segments fusionnés avant le 14 novembre, dont les ids morts n'ont pas de supprimés.
- (b) Marquer la vie pendant la boucle principale de la fusion, qui décode déjà ces postings, puis renuméroter les
  sorties : une seule lecture, plus de code.

Décision de c7 (14 novembre) : garder tel quel. Les 6 s s'échangent contre 17,7 Mo rendus et un `.gmap` juste.
