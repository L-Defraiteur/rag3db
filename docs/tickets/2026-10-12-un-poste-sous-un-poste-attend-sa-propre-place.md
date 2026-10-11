# Un `poste` appelé par la commande d'un autre `poste` attend sa propre place, sans fin

- **État** : ouvert (12 octobre 2026, optimiseur, à la demande de l'orchestration)
- **Gravité** : blocage. La trace des fils du cœur a tenu la place exclusive 30 minutes à 23 h 32
  sans courir : deuxième mesure perdue de la soirée. Avant cela, F a tenu la place d'ici pour
  rien de 21 h 54 à 22 h 45.
- **Atteignable en service** : oui, dès qu'une commande lancée par `poste mesure` (ou `poste
  lourd`) appelle elle-même `poste` : `run_e2e.sh` bâtit par `poste lourd`
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/poste`)

## Ce que c'est

Depuis la priorité de la mesure (e70a78a72), une mesure tient la porte et la place exclusive. Un
`poste lourd` lancé par sa commande attend la porte, que son propre ancêtre tient : il attend sans
fin. Une `poste mesure` sous un `poste lourd` attend la place exclusive, qu'un ancêtre tient en
partagé : même impasse. Le seul échappement était une variable posée à la main
(`RAG3WEAVER_VERROU_TENU`, lue par `run_e2e.sh`).

## La recette

```bash
poste mesure bash -c 'poste lourd true'     # ne rend jamais la main
```

## Le témoin

`extension/rag3weaver/scripts/test_poste.sh` : une mesure dont la commande appelle `poste lourd`
finit, et de même un lourd sous un lourd et une mesure sous un lourd.

## Ce qu'il faut pour le fermer

Le poste reconnaît lui-même un ancêtre tenant, par le pid de sa fiche comparé à la chaîne de
ses parents (`/proc`), et passe, mesure ou lourd, sans variable à poser. Il le dit sur stderr.
Le script est déployé sur les deux postes, avec son md5.
