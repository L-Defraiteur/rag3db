# Un appel d'outil attend derrière un index sans le dire

- **État** : ouvert (11 octobre 2026) — vu par le cœur et mesuré par la session codeparsers sur l'hôte de luciepc ; pour la session proto et l'arbre principal
- **Gravité** : blocage (un agent reste suspendu des minutes, sans réponse ni délai, jusqu'à ce qu'on l'interrompe)
- **Atteignable en service** : oui, dès qu'un index (ou une réindexation de la montre) court sur le backend qu'un agent interroge
- **Touche rag3weaver** : oui (verrou du catalogue, service d'embarquement, réponses des outils)

## Ce que c'est

Le 11 octobre à 20 h 16, `code-principal` (hôte de luciepc, binaire master faa00f279) commence sa
réindexation. Le cœur interroge ce même backend : `search_code {signals: [bm25]}` puis `usages` sur
un fichier frais restent **ouverts environ deux minutes, sans réponse, sans « occupé », sans délai**,
jusqu'à ce que Lucie les interrompe. À 20 h 20, un `usages` de la session codeparsers ne rend rien
en 60 s. Pendant ce temps :

- l'index rebâtit le plein texte depuis les lignes en tenant le catalogue ;
- l'index vectoriel « tombe le temps du rattrapage », puis se reconstruit ;
- l'index tourne à 98-99 % des vecteurs plusieurs minutes, le total bougeant (120 316, 120 321,
  120 317) : la montre réindexe les fichiers déposés puis retirés, l'index rattrape sans fin.

Ce n'est pas le « Catalogue occupé : une indexation le tient » de 18 h, qui, lui, **répondait**.

## Ce qu'il faut pour le fermer

- **Un appel d'outil ne reste jamais ouvert** : au-delà d'un délai court, il répond « occupé par un
  index (N %, depuis N s) », ou sert le dernier état lisible, en le disant.
- **Les appels d'outil passent avant les lots d'index**, au verrou du catalogue comme au service
  d'embarquement (la requête d'une recherche ne fait pas la queue derrière des lots de 128).
- **Les deux questions de Lucie** :
  - le signal lexical d'un fichier frais peut-il être servi avant ses vecteurs (le texte est là dès
    l'écriture) ?
  - un fichier relié en 6 s (mesure du cœur, 20 h 10) : où passe le temps — la montre (`settle_ms`,
    l'intervalle de balayage), l'extraction, l'écriture, le catalogue ? Un relevé par étape
    manque.
- Témoin : un index long sur un backend, un `usages` pendant ; la réponse arrive en moins de N s,
  « occupé » ou servie.

## Un second cas : l'ouverture d'un backend qui rattrape ses vecteurs (11 octobre, 20 h 35)

Mesuré par la mémoire longue en redémarrant l'hôte de Lucie (binaire e9d690915, lib de
20 h 08) : le **premier `initialize`** d'un client sur `code-rag3db` a pris **26 s**. Le journal
de l'hôte annonçait juste avant un rattrapage de **2 074 morceaux** à embarquer sur
`Scope_Chunk` (l'index vectoriel tombe le temps du rattrapage) : le backend se rouvre et
rattrape **avant** de répondre au premier appel. Les appels suivants : `daemon_status`
instantané, `usages` 0,46 s. C'est la même forme que le cas de l'index : le rattrapage se fait
derrière une réponse qui dit le retard (« vecteurs de N morceaux en rattrapage »), pas avant
elle — le client attend 26 s sans un mot, Claude Code reste sur « Connecting… ».
