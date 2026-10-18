# Une relation qui porte le nom d'une entité, à la casse près, se déclare sans refus et casse au premier lien

- **État** : ouvert (vu le 18 octobre 2026 par la session mémoire longue, lot M1)
- **Gravité** : blocage, mal nommé. Le manifeste se charge, la base s'ouvre, et le premier
  lien de la relation échoue avec un message du moteur qui ne dit pas la cause.
- **Atteignable en service** : oui, dès qu'un gabarit nomme une relation comme une entité
  à la casse près (`APPORT` et `Apport`) ; le moteur ignore la casse des noms de table.
- **Touche rag3weaver** : oui (le contrôle du manifeste), et le moteur pour son message.
- **Ouvert le** : 18 octobre 2026, session mémoire longue, à la demande de l'orchestration.
- **Pour** : arbre principal (le contrôle au chargement) ; le cœur pour le message du moteur.

## Ce qu'on voit

Le gabarit `memory` du lot M1 déclarait l'entité `Apport` et la relation
`APPORT: Fiche → Apport`. `PreparedBackend::load` l'accepte (`--describe` rend 19
outils), l'ouverture passe, puis le premier `relier_apport` rend :

```text
LinkRecordNode : « APPORT » — 1 opération(s) en échec : query error: Query execution failed:
Binder exception: Cannot bind Apport as a relationship pattern label.
```

Le moteur résout `APPORT` sur la table de nœuds `Apport` (les noms de table sont
insensibles à la casse) ; rien, ni au chargement ni à la création des tables, n'a dit que
les deux noms se confondent. Contourné dans M1 en renommant la relation `A_POUR_APPORT`.

## Ce qu'il faudrait

1. **Au chargement du manifeste** : refuser une relation dont le nom égale celui d'une
   entité, ou d'une autre relation, **casse ignorée**, en nommant les deux — avant toute
   base ouverte, comme les autres contrôles de `load`.
2. **À la création des tables** (le filet, pour un schéma qui ne passe pas par un
   manifeste) : un `register_relation_with` dont le nom est déjà pris par une table, casse
   ignorée, refusé en le disant, au lieu de laisser le moteur créer ou confondre.

## Témoin rouge à écrire

Un manifeste avec l'entité `Note` et la relation `NOTE: Note → Note` : `load` doit le
refuser en nommant « NOTE » et « Note ». Aujourd'hui il charge.
