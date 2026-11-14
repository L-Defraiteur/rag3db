# Sans service, la démo Node.js embarque quand même, sans dire par quoi

- **État** : fermé (14 novembre 2026, embarquement) : départagé, ni l'une ni l'autre des deux
  lectures — un réglage du poste ; la passe sans service retire toutes les variables de service et
  chaque passe nomme son embarqueur ; ouvert le 14 novembre, relevé par hotfixs en jouant la démo, à
  la demande de c7
- **Famille** : produit
- **Gravité** : réponse fausse en apparence. La démo annonce « plein texte seul, l'avertissement
  visible », et rend des vecteurs prêts à 100 % sans dire quel embarqueur les a calculés.
- **Atteignable en service** : oui, à qui lance `bindings/nodejs/demo/demo.sh`, et sans doute à qui
  lance le binaire sans `RAG3WEAVER_EMBED_SERVICE`
- **Touche rag3weaver** : oui (`extension/rag3weaver/bindings/nodejs/demo/`, et l'embarquement du
  binaire si un repli local répond)

## Ce que c'est

La passe « Sans service d'embarquement (plein texte seul) » de `demo.js` retire bien
`RAG3WEAVER_EMBED_SERVICE` (`delete process.env.RAG3WEAVER_EMBED_SERVICE`, ligne 59). Pourtant, son
index rend `File`, `Library` et `Scope` avec « vecteurs ready (100%) », comme la passe avec service, et
aucun avertissement ne s'affiche. Démo jouée le 14 novembre 2026 au portable, avec `rag3weaver@next`
0.0.1-alpha.1. Journal : `~/.cache/rag3db-locks/demo-nodejs/demo.log` au portable.

## Les deux lectures possibles (c7)

1. L'embarqueur local du binaire, le repli par défaut, prend la main sans le dire.
2. L'en-tête de `demo.sh` promet un « plein texte seul » qui n'existe plus.

## Ce qu'il faut pour le fermer

- Départager : quel embarqueur a répondu dans la passe sans service, à lire dans `/sante` de l'hôte
  ou dans le journal de l'index.
- Faire dire à la démo, à chaque passe, quel embarqueur a embarqué.
- Si le plein texte seul n'existe plus, réécrire l'en-tête de `demo.sh` ; sinon, rétablir
  l'avertissement.

## Départagé (14 novembre 2026, embarquement) : la variable commune du poste

Ni l'une ni l'autre des deux lectures : **un réglage du poste**.

- **La cause** : le binaire lit `RAG3WEAVER_SERVICE_EMBED` d'abord, puis l'alias d'avant,
  `RAG3WEAVER_EMBED_SERVICE` (`Capability::variables`, `src/model_source.rs`). Le creux lit
  `RAG3WEAVER_SERVICE_SPARSE`, puis ces deux-là. La démo ne retirait que l'alias.
- **Le poste** : il pose `RAG3WEAVER_SERVICE_EMBED=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`, à
  trois endroits : `~/.config/fish/config.fish`, `~/.zshrc`, `~/.config/environment.d/rag3weaver.conf`.
  La passe « sans service » recevait donc encore les tunnels.
- **L'embarqueur qui a répondu** : `granite-278m`, 768 dimensions, sur `127.0.0.1:7979`, d'après son
  `/sante` (`rag3weaver-embeddings`, adresse servie `127.0.0.1:7878` à luciepc). `7980` sert bge-m3 ;
  `7981` ne répond pas.

### Les preuves

Le binaire publié, `rag3weaver@next` 0.0.1-alpha.1, a été joué depuis le dossier de la démo du
14 novembre au portable, avec `codeparsers/src` :

| Passe sans service | Ce qu'on voit |
|---|---|
| `env -u RAG3WEAVER_SERVICE_EMBED -u RAG3WEAVER_EMBED_SERVICE -u RAG3WEAVER_SERVICE_SPARSE` | plein texte seul, vecteurs `never (0%)` partout, l'avertissement « pas de service d'embarquement » dans describe et dans index_state |
| `env -u RAG3WEAVER_EMBED_SERVICE` seul, comme la démo | vecteurs `ready (100%)`, aucun avertissement : le ticket reproduit |

Ce que ces deux passes établissent :

- le binaire n'a **pas de repli local** : la lecture 1 tombe ;
- **le plein texte seul existe** et l'avertissement marche : la lecture 2 tombe, et l'en-tête de
  `demo.sh` dit vrai.

### Le correctif

- **`demo.js`, sans service** : la passe retire **toutes** les variables de service,
  `RAG3WEAVER_SERVICE_*` et l'alias.
- **`demo.js`, chaque passe** : elle dit son embarqueur après l'index, par la règle du binaire
  (l'adresse déclarée, sinon la première variable posée, puis la première adresse dont le `/sante`
  sert le modèle déclaré). Elle écrit soit `embarqueur : granite-278m servi par <adresse>
  (<variable> ; /sante : …)`, soit `embarqueur : aucun — …`.
- **L'en-tête « Avec le service »** lit la variable commune. Il affichait « (undefined) » sur un poste
  qui ne posait que la commune, et l'alias quand les deux étaient posées.
- **`demo.sh`** lit et exporte `RAG3WEAVER_SERVICE_EMBED`, sinon l'alias, sinon les tunnels par
  défaut. Il sonde la première adresse de cette variable.

### Le témoin et le rejeu

- **Le témoin** est `bindings/nodejs/test/demo_dit_son_embarqueur.js`. Il joue `demo.js` contre un
  faux paquet `rag3weaver`, qui note l'environnement que le binaire recevrait, et contre un faux
  `/sante`. Il ne demande ni binaire ni réseau. Il était rouge (4) sur le `demo.js` de master :
  `RAG3WEAVER_SERVICE_EMBED` et `_SPARSE` restaient, et l'en-tête affichait « (undefined) ». Il est
  vert après le correctif.
- **Le rejeu avec le binaire publié**, l'environnement du poste tel quel :

| Passe | Ce que la démo affiche |
|---|---|
| sans service | « embarqueur : aucun », vecteurs `never (0%)`, l'avertissement |
| avec service | « embarqueur : granite-278m servi par 127.0.0.1:7979 (RAG3WEAVER_SERVICE_EMBED ; /sante : rag3weaver-embeddings, 768 dim., adresse 127.0.0.1:7878) », vecteurs `ready (100%)` |

### Ce qui reste, à part

`test/essai.js`, l'épreuve « sans service » du paquet, ne retire aucune variable non plus : ticket
`2026-11-14-l-epreuve-sans-service-du-paquet-herite-des-variables-du-poste.md`.
