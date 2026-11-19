# Le témoin du relais tronqué du chat perd sa page du navigateur, une fois

- **État** : ouvert (19 novembre 2026)
- **Famille** : produit (un témoin du produit)
- **Gravité** : un faux rouge de passe. Une passe complète verte sur tout le reste tombe sur ce seul témoin
  navigateur, et le lot attend un rejeu de 30 minutes.
- **Atteignable en service** : non. C'est la page du navigateur du témoin qui disparaît, pas le chat.
- **Touche rag3weaver** : oui, les témoins navigateur (`src/daemon_host/chat_contextuel.rs`, `src/daemon_host/navigateur.rs`).
- **Pour** : la session qui tient le chat contextuel et les témoins navigateur (à réattribuer par c7).
- **Signalé par** : ED, à la passe du lot define_ref.

## Ce qu'on voit

Le 19 novembre 2026, à luciepc, lib 3d706f709, tête 250d55be99 de `define-ref-signal-2`. L'étape des témoins
navigateur de la passe épinglée (`passe-ed-def-3`) rend 44 sur 45. Le seul rouge :

```
daemon_host::chat_contextuel::le_relais_du_chat_garde_sa_cible_quand_le_texte_est_tronque ... FAILED
panicked at src/daemon_host/navigateur.rs:254:21:
Runtime.evaluate : {"code":-32000,"message":"Inspected target navigated or closed"}
```

La page que le témoin inspecte a navigué, ou a été fermée, pendant un `Runtime.evaluate`. Le lot define_ref ne
touche ni au chat, ni au navigateur, ni à leurs gabarits.

## Ce qui le dit instable plutôt que cassé

La même étape, rejouée trois fois de suite sur la même tête et la même lib, n'a pas fait rougir ce témoin.
Le journal en est `~/.cache/rag3weaver-build/journaux-ed/nav-{1,2,3}.log` à luciepc. Ces trois essais rendent
cinq autres rouges, mais seulement parce que je les ai lancés sans `RAG3WEAVER_PAQUETS_REACT`, que la passe
pose elle-même. Ces cinq rouges ne disent rien du code.

## Ce qu'il faudrait

- Chercher ce qui fait naviguer la page : un rechargement de l'IDE pendant le témoin, un autre témoin qui partage
  l'onglet.
- Au lieu de paniquer, le témoin devrait dire si la cible a changé, en nommant l'URL d'avant et d'après.
- Ne pas l'affaiblir par un rejeu automatique : un rouge qu'on rejoue en silence n'est plus un témoin.

## Comment le fermer

La cause trouvée et corrigée, puis le témoin rejoué vingt fois sans rouge sous la charge d'une passe.
