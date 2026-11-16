# Le mode plain et la fin d'un pas que l'hôte relance

- **État** : ouvert, à surveiller. Le défaut T1 est corrigé (tty-20, 9f0cbe7ce4) ; un retour anticipé reste possible quand l'hôte relance un pas après sa fin (session tty, 16 nov. 2026).
- **Famille** : produit
- **Gravité** : gêne pour les scripts (la campagne d'essais de c7) : le terminal rendrait la main avant la vraie réponse, et le journal serait incomplet.
- **Atteignable en service** : oui, en mode `--plain --demande`, quand l'hôte relance l'agent après un `ended` (règle (B) : « un pas ne finit jamais sans réponse »).
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`, `src/main.rs`).

## Ce que c'est

Depuis tty-20, le mode texte ne compte que les lignes arrivées après sa demande, et s'arrête à la première ligne `kind: "ended"` qui suit. Or l'hôte peut poser un `ended` puis relancer l'agent : on l'a vu le 16 nov. sur une tâche d'essai (`essai-nom-tty`), où l'hôte a posé « relance : ta demande attend une réponse… » après un `step_done` sans compte rendu. Si un `ended` précède cette relance, le mode texte s'arrête avant la réponse.

## Ce qu'il faut faire

1. **Le prouver d'abord** : ajouter au faux hôte (`mesures/faux_hote.py`) un scénario « fin puis relance » (une route `POST /faux/relance` qui pose `ended`, puis 1 s plus tard une ligne `host`/`report` de relance, puis un compte rendu, puis le `ended` final), et un témoin à côté de `mesures/plain_stays_attached.py` (nom anglais, par exemple `mesures/plain_waits_for_a_relaunch.py`) qui exige que le journal porte le compte rendu final.
2. **Le corriger si le témoin est rouge** : ne finir qu'à un `ended` après lequel l'état de la tâche (`GET /ide/teammate/list?supervisor=supervisor`, champ `state` de la tâche) n'est plus `running` pendant un délai de grâce (2 s), ou mieux, si archi l'ajoute, à un champ qui dit « fin de la demande » plutôt que « fin du pas ». Demander à archi ce qu'il préfère avant de choisir.
3. Ne pas toucher au terminal riche (`tui.rs`) : il ne s'arrête pas à `ended`.

## La preuve

Le nouveau témoin rouge puis vert ; `plain_stays_attached.py` toujours vert ; la passe courte (crate, `check_features`, harnais).

## Taille et blocage

2 à 4 heures. Rien ne bloque le témoin ; le correctif propre peut attendre un champ d'archi.
