# Tickets

Un fichier par défaut connu, non corrigé ou en cours. Les défauts que nous découvrons
viennent ici, avec leur témoin. Les « bugs connus » du journal (§6) y migreront.

## Convention

- **Nom** : `<année>-<mois>-<jour>-<titre-court>.md`, le jour de l'ouverture.
- **En tête**, une ligne par champ, `- **Champ** : valeur`, avant la première section ; la garde
  (`extension/rag3weaver/scripts/check_tickets.py`, appelé par `check_features.sh` et la CI)
  refuse un champ absent et un état qui contredit le dossier :
  - **État** : `ouvert …` (ou `en cours …`) pour un ticket de `docs/tickets/` ; pour un ticket de
    `closed/`, un premier mot qui le dit clos (corrigé avec son commit, fermé, fait, résolu,
    abandonné, non reproduit, accepté) ;
  - **Famille**, pour un ticket ouvert : `moteur` ou `produit` (ci-dessous) ;
  - **Gravité** : perte, plantage, blocage, réponse fausse, coût ;
  - **Atteignable en service** : oui ou non, et comment ;
  - **Touche rag3weaver** : oui (où) ou non ;
  - **Pour** et **Ouvert le**, facultatifs.
- **Sections** :
  - ce que c'est, en une phrase ;
  - la recette minimale (Cypher brut), ou pourquoi il n'y en a pas ;
  - le témoin (fichier, nom du test), ou son absence assumée et ce qu'il faudrait pour l'écrire ;
  - la cause, si elle est connue ;
  - le correctif de l'amont s'il existe (à lire, ne pas copier) ;
  - ce qu'il faut pour le fermer.
- **Moteur ou produit** (le champ Famille) : `moteur` si le correctif vit dans `src/`, dans une
  extension C++ du moteur (`extension/vector`, `extension/geo`) ou dans le harnais de test du
  moteur, c'est la famille sur laquelle se juge la stèle du moteur ; `produit` s'il vit dans
  `extension/rag3weaver`, codeparsers, lucivy ou les outils, dont les « perte ou réponse
  fausse » vont à la stèle du produit (arbre principal, avec codeparsers). En cas de doute,
  moteur. Les fermés forment une seule table (décision de Lucie, 11 octobre 2026).
- **Fermeture** : le commit qui corrige passe le ticket à « corrigé `<commit>` », retire le
  témoin de `test/transaction/concurrence/known_red.txt`, **déplace le fichier dans `closed/`**
  (`git mv`) et passe son État à un mot de clôture, dans le même commit. Les
  anciens liens vers un ticket fermé se retrouvent par `git log --follow`.
- **Les filets** (Lucie, 18 octobre 2026) : un garde-fou posé ailleurs à cause d'un défaut
  ouvert (une borne du produit, un refus, un repli, un `known_red`) **cite le ticket** dans son
  commentaire, et le ticket le liste dans une section « ses filets » (fichier et symbole). Le
  commit qui ferme le ticket retire ou recalcule chacun de ses filets, ou dit pourquoi il
  reste. Raison : la borne `TEXT_BYTES_PER_BUFFER_GIB` d'`estimate.rs`, posée le 4 octobre pour
  le point de reprise sous un petit tampon, a survécu au correctif moteur du 11 (47a283e6c) et
  refusait encore le 18 tout dépôt de plus de 64 Mo de texte.

## Index

L'index ne s'écrit plus à la main (7 novembre 2026 : ses comptes épinglés faisaient un conflit à
chaque lot qui ouvrait ou fermait un ticket, quatre rebases le même soir ; ticket
`closed/2026-11-07-l-index-des-tickets-epingle-ses-comptes-et-fait-un-conflit-a-chaque-lot.md`).
Il se génère depuis les en-têtes, avec ses comptes, dans `docs/tickets/INDEX.md`, ignoré par git :

```
python3 extension/rag3weaver/scripts/check_tickets.py --index
```

Avant le push d'un lot, la liste des tickets qu'il ouvre, ferme ou change d'état, à relire :

```
python3 extension/rag3weaver/scripts/check_tickets.py --lot origin/master
```
