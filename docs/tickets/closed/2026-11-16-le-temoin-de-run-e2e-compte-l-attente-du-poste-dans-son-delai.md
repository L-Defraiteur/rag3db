# Le témoin de run_e2e.sh compte l'attente du poste dans son délai

- **État** : fait (18 novembre 2026, branche run-e2e-witness-holds-poste) : test_run_e2e.sh se relance une fois sous poste lourd (32G) et ses cas trouvent le poste tenu ; derrière un lourd qui tient l unique place, 2 ÉCHEC en 124 avant, 22 OK après (une seule attente) ; sous un poste déjà tenu, 22 OK en 20 s sans attente
- **Famille** : produit (outillage : `extension/rag3weaver/scripts/test_run_e2e.sh`)
- **Gravité** : faux rouge. Le témoin rougit quand le poste est occupé, sans rien dire du code.
- **Atteignable en service** : non (le témoin d'un script de passe)
- **Touche rag3weaver** : oui (`scripts/test_run_e2e.sh`)
- **Pour** : G (optimiseur)
- **Signalé par** : G, aux rejeux du lot `run-e2e-temoin-lib`, le 15 et le 16 novembre 2026

## Ce qu'on voit

Plusieurs cas de `test_run_e2e.sh` (« une suite sous une feature absente », « le jeu par défaut porte openai-llm… »,
les comptes « --test e2e e2e_lectures ») rendent 124 sous leur `timeout 120`. Ils rougissent quand le portable a un
lourd en cours. Rejoués seuls avec le poste libre, ils passent en 29 s.

## La cause

Chaque cas lance `run_e2e.sh`, qui se relance par `confiner_validation.py lancer`, donc par `poste lourd`. Le portable
n'a qu'une place de lourd. Le `timeout 120` du témoin compte donc l'attente dans la file du poste, en plus du travail du
cas.

## Ce qu'il faudrait

Que le délai du témoin ne compte que le travail du cas. Deux pistes :
- le témoin tient lui-même la place du poste une fois, et ses cas passent par la garde sans la reprendre ;
- ou les cas qui ne bâtissent rien sautent le confineur par un mode de test explicite, qui ne vaut que sous le témoin.

En attendant, le témoin se joue poste libre (`poste --qui` avant), et un 124 se rejoue avant d'être cru.
