# Le repli par une activation nommée

- **État** : corrigé (15 novembre 2026, embarquement, branche `repli-activation`)
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

## Livré (15 novembre, branche `repli-activation`)

Conception relue par c7. Ses décisions :
- **le placement**, c'est `peripherique`, `address` et les variables de carte de l'env (`RAG3WEAVER_BURN_DEVICE_*`,
  `*_VISIBLE_DEVICES`), rien d'autre ;
- **la carte du repli se mesure aussi**. Si les deux cartes sont pleines, l'activation principale se fait, et l'état
  dit les deux mesures ;
- **le plafond du doublement est un réglage**, `fallback_hold_max_factor`, 16 par défaut ;
- **l'adresse du repli** est la clé `address` du bloc d'activation, exigée quand le bloc est cité en repli. Le port 0
  est admis.

Ce que fait le code :
- **au chargement**, `fallback: {"activation": "<nom>"}` se résout en son bloc nommé, et un nom inconnu se refuse
  avec les noms déclarés. `ecart_hors_placement` refuse, nommé, un repli qui diffère hors placement (`modele`,
  `programme`, `args`, `profil`, une variable qui n'est pas de carte) ou qui n'a pas d'`address` ;
- **au repli**, l'activation de repli est lancée ou rejointe sur sa source, avec son bail. Sa recette complète est
  attestée, et l'état dit « replié vers l'activation <nom> (recette vérifiée) ». Au retour, elle rend son bail, et
  son éviction fait le reste ;
- **le doublement de la tenue** : la première fenêtre mesurée après un retour, si elle retombe sous le débit après
  un repli pris pour le débit, double la tenue et le dit (« retombé sous le débit dans la fenêtre suivante : tenue
  doublée à N s »). Une fenêtre qui tient remet la tenue à sa base.

Sept témoins, rouges d'abord (9d5e6750bf), verts ensuite (646556c267).
