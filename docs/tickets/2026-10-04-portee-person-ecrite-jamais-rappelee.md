# La portée `person` d'une mémoire est écrite, et jamais rappelée

- **État** : ouvert — dette nommée, avec sa condition de sortie
- **Gravité** : réponse fausse (une mémoire existe en base et ne revient jamais)
- **Atteignable en service** : oui, dès que le gabarit `memory` sert
- **Touche rag3weaver** : oui (le crochet de rappel, et le protocole d'outil)

## Ce que c'est

Une mémoire peut déclarer `reach: person`, et **aucun lecteur n'existe pour
cette portée**. Vérifié à la source le 4 octobre 2026 : le gabarit
(`templates/backends/memory/backend.json`) déclare le champ, le met dans
`hashsafe` et dans `returnFields` — et le crochet de rappel qui devrait le lire
**n'est pas encore écrit** (contrat acté avec la session recherche, code après
la passe Gemini). Quand il le sera, il devra **exclure** cette portée au lieu de
la **filtrer**, faute d'identité d'appelant au protocole.

Donc le défaut a deux étages, et il faut les distinguer sous peine de corriger
le mauvais : **aujourd'hui** le champ n'a aucun lecteur du tout ; **demain**, il
en aura un qui ne saura qu'exclure. La mémoire est écrite, stockée, cherchable
en principe — et invisible en pratique dans les deux cas.

C'est le défaut que la session a passé la nuit à nommer ailleurs, dans sa forme
la plus pure : **une information existe, et rien ne la consulte.** Ici elle est
assumée et datée, pas découverte par surprise.

## La recette minimale, ou pourquoi il n'y en a pas

Pas de recette : c'est un défaut de **chemin absent**, pas de requête fausse.
Écrire une mémoire avec `reach: "person"` réussit, et un
`MATCH (m:Memory) WHERE m.reach = 'person'` la montre en base. Il n'y a rien à
exécuter qui donne un résultat faux — il n'y a personne à qui le demander.

## Le témoin, ou son absence assumée

**Aucun témoin n'existe**, et il ne peut pas en exister aujourd'hui : on ne
teste pas l'absence d'un crochet qui n'est pas écrit. Le défaut vit dans le
gabarit et dans la proposition
(`docs/3-octobre-2026-20h41/01-une-memoire-longue-proposition.md`), commité en
`7438b2909` comme dette nommée.

**Ce qu'il faudrait pour l'écrire** : le crochet. Le jour où il existe, le
témoin est immédiat et il doit être écrit **dans le même commit** — une mémoire
`reach: person` écrite puis cherchée, qui ne revient pas, et un message de test
qui dit que c'est l'attendu *tant que la dette vit*. Sans cette dernière
phrase, le test suivant à le voir le prendra pour un défaut de la recherche.

## La cause

Le protocole d'outil ne porte **pas d'identité d'appelant**. Le crochet ne peut
donc pas répondre à « cette mémoire est-elle à la personne qui parle ? », et il
prend le seul parti sûr du point de vue de la confidentialité : ne jamais rendre
une mémoire de portée `person`. Le parti est juste ; son silence ne l'est pas.

## Ce qu'il faut pour le fermer

**Sa condition de sortie est écrite depuis le début** : le jour où le protocole
porte une identité d'appelant, le crochet **filtre** au lieu d'exclure, et la
dette se ferme sans rien changer au gabarit ni aux données.

D'ici là, deux choses séparables, dont la seconde vaut d'être faite tout de
suite :

1. le filtrage, qui attend l'identité d'appelant (session recherche, protocole
   du chat) ;
2. **que l'écriture le dise.** Aujourd'hui `remember` accepte `reach: person`
   sans un mot. Il devrait rendre, dans sa charge ordinaire, que cette portée
   n'est pas rappelable pour l'instant — l'appelant choisirait alors `project` en
   connaissance de cause. C'est deux lignes, ça ne demande aucune identité, et ça
   transforme un silence en information.
