# Le code d'un refus du modèle n'est gardé nulle part

- **État** : ouvert, le 9 novembre 2026. Relevé par la session mémoire (96) pendant l'essai de
  l'équipier sur les README ; pour everything declarative, et pour Lucie (son quota Vertex).
- **Famille** : produit
- **Gravité** : la cause d'une attente de dix-sept minutes ne peut pas être dite après coup.
- **Atteignable en service** : oui, chat, équipier, juge et archiviste (même client).
- **Touche rag3weaver** : oui, le client du modèle et l'événement `LlmCall` du journal des runs.

## Ce qu'on voit

Le run `agent-1a1c02c5885-2122` compte les reprises de chaque appel (`retries: 4` sur l'appel 8)
mais ne garde ni le code rendu par Vertex (429 RESOURCE_EXHAUSTED, 503, une coupure), ni l'heure
ni l'attente de chaque reprise. `daemon.log` n'en dit rien non plus. Une seule trace indirecte :
une connexion de l'hôte vers Google restée en CLOSE-WAIT pendant l'attente. On ne peut donc pas
dire si le projet `lr-hub-472010` touchait son quota, ni à quel rythme.

## Ce qu'il faut

- `LlmCall` porte la liste de ses reprises : code, message court du fournisseur, attente.
- Le journal de l'hôte dit chaque refus une fois (le code et le modèle).
- Ce qu'il faut pour Lucie : le compte des 429 par heure sur son projet, lisible depuis les runs.
