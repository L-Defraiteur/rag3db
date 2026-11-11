# Tickets

Un fichier par travail identifié : défaut connu, capacité manquante, suite de lot,
intégration, mesure, conception à compléter ou décision bloquante. Depuis le 11 novembre 2026,
la règle concerne aussi les actions signalées entre sessions. Un ticket existant est complété
plutôt que dupliqué ; un message sans nouvelle action n’a pas besoin d’un nouveau ticket.

Les ouverts vivent dans `docs/<session>/tickets/` chez leur porteur, ou dans `docs/tickets/`
si le porteur reste à attribuer. Tous les fermés vont dans `docs/tickets/closed/`.

## Création, transmission et reprise

La session qui identifie un nouveau travail crée ou complète son ticket avant de le transmettre.
Le message à une autre session cite son chemin ; une preuve ou une demande ne reste pas
seulement dans la messagerie. Le ticket précise la demande ou le constat, les éléments vérifiés,
ce qui reste incertain, le travail attendu, le porteur et les critères de fin.

Le destinataire confirme la prise en charge et relie le même ticket à son lot. Un changement
de porteur déplace le fichier par `git mv`, sans en garder une seconde copie ; les sessions
concernées sont informées du nouveau chemin. L’orchestration suit les actions sans porteur.
Ouvrir ou recevoir un ticket n’autorise pas à lui seul une opération qui exige une consigne
particulière de Lucie : par exemple une publication ou une activation en mémoire réelle.

Terminer ou abandonner le travail ferme le ticket dans le même commit que son déplacement
vers `docs/tickets/closed/`. La clôture cite les commits et validations de réalisation, ou
la raison d’abandon et son éventuel remplacement. Envoyer un message, écrire une conception,
rencontrer un blocage ou remettre le travail à plus tard ne clôt pas une implémentation attendue.

À chaque reprise ou compte rendu de lot, relier les actions encore en cours à leurs tickets et
régulariser les suites qui n’en ont pas. Le rapport garde le contexte ; le ticket garde l’action.

## Convention

- **Nom** : `<année>-<mois>-<jour>-<titre-court>.md`, le jour de l'ouverture.
- **En tête**, une ligne par champ, `- **Champ** : valeur`, avant la première section ; la garde
  (`extension/rag3weaver/scripts/check_tickets.py`, appelé par `check_features.sh` et la CI)
  refuse un champ absent et un état qui contredit le dossier :
  - **État** : `ouvert …` (ou `en cours …`) pour un ticket de `docs/tickets/` ; pour un ticket de
    `closed/`, un premier mot qui le dit clos (corrigé avec son commit, fermé, fait, résolu,
    abandonné, non reproduit, accepté) ;
  - **Famille**, pour un ticket ouvert : `moteur` ou `produit` (ci-dessous) ;
  - **Gravité** : perte, plantage, blocage, réponse fausse, coût ; pour une tâche sans défaut,
    préciser l’impact attendu ou « planification », sans inventer une panne ;
  - **Atteignable en service** : oui ou non, et comment ;
  - **Touche rag3weaver** : oui (où) ou non ;
  - **Pour** : le porteur ou la session attendue ; **Signalé par** : la session ou la demande
    à l’origine du travail, avec le lien vers sa source ; **Ouvert le**, facultatif.
- **Sections**, adaptées au travail :
  - ce que c'est, en une phrase ;
  - la demande à réaliser ; pour un défaut, la recette minimale (Cypher brut) ou pourquoi il
    n’y en a pas ;
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
- **Fermeture** : la réalisation passe le ticket à « fait `<commit>` » ou « corrigé `<commit>` » ;
  un abandon le passe à « abandonné », avec sa raison. **Déplacer le fichier dans
  `docs/tickets/closed/`** (`git mv`) et changer son État dans le même commit. Pour un défaut
  suivi dans `test/transaction/concurrence/known_red.txt`, traiter aussi son entrée selon le
  verdict réel, sans retirer un témoin rouge parce que le correctif est abandonné. Les
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
