# Le repli par une activation nommée

- **État** : ouvert (15 novembre 2026, embarquement ; décision de c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : manque. Le repli quand la carte est pleine existe pour un service posé
  (`fallback: {"service": "127.0.0.1:7879"}`), pas encore pour une activation nommée.
- **Atteignable en service** : oui (le jour où les services de card2 sont déclarés par une activation).
- **Touche rag3weaver** : oui (`src/model_source.rs`, `src/model_source/activation.rs`)

## Ce que c'est

La conception du repli (`docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md`,
« Le repli vers un service quand la carte est pleine ») prévoit deux formes :
`fallback: {"service": <adresse>}` et `fallback: {"activation": <nom>}`. Le lot du 15 novembre ne livre
que la première. Sous elle, le repli vérifie toujours le modèle et la dimension, et jamais la recette.
L'état le dit : « repli sans recette vérifiée ».

## Ce qu'il faudrait

`fallback: {"activation": "<nom>"}` cite une activation déclarée sous `activations`. Au repli, cette
activation est activée ou rejointe, comme toute déclaration qui la cite.

La recette se compare alors à celle de l'activation nommée, **moins le périphérique et les args de
placement**. Sinon la comparaison refuserait tout repli : la recette porte le périphérique, et un repli
est par construction sur une autre carte. Il faut d'abord définir ce que sont les args de placement
(ceux qui choisissent la carte), puis exclure ces champs du hash comparé.

Témoins, rouges d'abord :
- un repli par activation dont la recette moins le placement est celle déclarée : il passe, et l'état
  ne dit plus « sans recette vérifiée » ;
- une recette étrangère hors placement : refus nommé ;
- carte pleine, repli par activation : l'activation nommée est lancée, et elle seule.

## L'alternance d'un repli pris pour le débit (décision de c7, 15 novembre)

Le lot du 15 novembre pose une tenue : `fallback_hold_secs` (défaut 4 × `window_secs`). Un repli ne revient
pas à la carte avant elle, et l'état le dit : « replié … jusqu'à <heure> au plus tôt ». Il reste un cas : une
carte qui a la place en VRAM mais calcule lentement. Le repli pris pour le débit revient à la fin de sa tenue,
retombe sous le seuil dans la fenêtre suivante, et alterne ainsi.

Ce qu'il faudrait, dans ce lot-ci : un repli pris pour le débit qui revient puis retombe dans la fenêtre
suivante **double sa tenue**. Témoin : deux retombées de suite, et la deuxième tenue est le double de la
première.
