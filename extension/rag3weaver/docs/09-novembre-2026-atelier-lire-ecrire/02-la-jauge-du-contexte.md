# La jauge du contexte

Session archi, 9 novembre 2026. C'est une demande de Lucie, du 8 au soir : « la barre de
progression du contexte, je la vois pas beaucoup avancer, ni le pourcentage ». c7 me l'a confiée.

## Ce qu'elle mesure

`used` vaut les jetons d'entrée du **dernier appel** au modèle, tels que le fournisseur les rend
(`LastCall`, `agent.rs`). `max` vaut la fenêtre du modèle déclaré (`Llm::context_len`). La page
fait `used / max`, à l'échelle linéaire et sans plafond inventé.

## Les trois causes, relevées sur l'atelier de Lucie

L'hôte réel donnait, pour l'atelier de Lucie : `gemini-3.8-flash`, `max = 1 000 000`,
`used = null` sur 10 tours, et environ 4 900 jetons d'entrée au dernier tour.

1. **Le plafond écrase la barre, et rien n'écrit le pourcentage.** 5 000 jetons sur 1 000 000
   font 0,5 %, et un `read_file` de 200 lignes en ajoute environ 0,3 %. La jauge ne mentait pas,
   mais elle ne se voyait pas, et elle n'écrivait que « 4,9 k / 1 M ».
2. **Elle ne bougeait qu'en fin de tour,** alors que le flux porte `LlmCall` (`prompt_tokens`)
   à chaque appel du tour.
3. **Elle se perdait au redémarrage de l'hôte :** `last_call` n'était pas gardé avec la session.

## Ce qui est fait (branche jauge-du-contexte)

- **Le pourcentage s'écrit** : une décimale sous 10 %, « < 0,1 % » plutôt qu'un zéro. La barre
  garde au moins 3 px dès qu'il y a un compte.
- **La jauge suit chaque `LlmCall`** du run du chat, pendant le tour.
- **`last_call` est gardé dans le fichier de session** (`ChatFile`), et relu au démarrage.
- **Au survol** : les jetons du dernier appel, le plafond du modèle, l'heure.
- **Le témoin navigateur** : `the_context_gauge_follows_each_model_call_says_its_percentage_and_survives_a_restart`.
  Il joue un faux modèle d'un million de jetons et trois `read_file` de 200 lignes, puis un
  redémarrage de l'hôte. Il a été vu rouge deux fois : une seule valeur en fin de tour, puis,
  sans la persistance, la jauge vide après le redémarrage.

## Lot suivant : ce qui pèse, au survol

Le fournisseur ne rend qu'un total. Le découpage s'estime donc au prorata des caractères du
dernier appel :
- les instructions (le message système) ;
- les définitions des outils ;
- l'historique, et dans l'historique la part des résultats d'outils.

C'est l'hôte qui l'estime et le rend avec `context`. Ce lot aura son propre rouge.
