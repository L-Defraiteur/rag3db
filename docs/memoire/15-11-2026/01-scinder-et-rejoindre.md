# Scinder et rejoindre des fiches — la conception

*15 novembre 2026, session mémoire (96). Ce sont les deux gestes de Lucie qui restent sur les fiches
que l'archiviste propose. `confirmer_fiche` est sur master depuis `10b7a0edb`.*

## En quatre lignes

1. **`scinder_fiche(uuid, groupes)`** : chaque groupe de sections devient une fiche `proposed`, avec
   une phrase de six mots au moins et les apports de ses sections. Une section qui n'est citée dans
   aucun groupe est un refus.
2. **L'ancienne fiche est retirée** (`reject` si `proposed`, `retire` si `current`). Chaque fiche neuve
   est ancrée à une Ref de genre `issue-de` qui porte l'ancien nom.
3. **`rejoindre_fiches(uuid_a, uuid_b)`** : les apports de A passent sur B, A est retirée, et B est
   ancrée à une Ref de genre `alias` qui porte le nom de A.
4. **Les genres `issue-de` et `alias` sont déclarés** (`create_ref_type`). Ni champ ni relation de plus.

## Ce que la lecture du code a changé : un apport ne se déplace pas, il se recopie

La conception validée le 14 novembre disait « les apports déplacés par la relation, sans toucher
l'apport immuable ». Deux faits du code s'y opposent :

- **L'identité d'un `Apport` est `{fiche, source, section}`** : le nom de sa fiche en fait partie, et
  un L1 ne change plus une fois posé. Défaire `A_POUR_APPORT` et le refaire vers la fiche neuve
  laisserait le champ `fiche` sur l'ancien nom. Or `recall_apports` filtre par ce champ, et le fichier
  de la fiche ne l'afficherait pas.
- **Aucun outil ne défait une relation.** La déliaison existe (`Catalog::unlink`, `retirer` dans
  `RelationBatchNode`), mais seule l'indexation du code l'emploie.

Ce que je propose : **une copie par fiche neuve**, `Apport {fiche: <nom neuf>, source, section, texte,
niveau, state}` identique hors du champ `fiche`, reliée à la fiche neuve. L'apport d'origine reste sur
l'ancienne fiche retirée, comme provenance. Rien n'est défait, rien d'immuable n'est touché, et le
rappel d'une fiche neuve rend ses apports. Le coût : un apport existe en deux exemplaires, l'un sous
une fiche retirée, que le rappel écarte par l'état de sa fiche.

L'autre voie serait d'exposer la déliaison (`retirer: true` dans les liens de `link`) et d'accepter un
champ `fiche` qui ment. Je ne la recommande pas.
