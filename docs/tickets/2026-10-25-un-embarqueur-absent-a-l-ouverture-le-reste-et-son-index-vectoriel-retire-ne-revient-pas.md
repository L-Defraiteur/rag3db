# Un embarqueur absent à l'ouverture le reste, et son index vectoriel retiré ne revient pas

- **État** : ouvert, le 25 octobre 2026 (G). Le lot est en cours chez G.
- **Gravité** : réponse fausse en silence. L'état dit « vectors running 97 % » et garde un avertissement
  faux, alors que rien ne se calcule ; la recherche dense se replie.
- **Atteignable en service** : oui. Il suffit d'ouvrir un backend quand le service d'embarquement n'est
  pas encore là : un hôte réveillé par sa socket avant ses tunnels, un service qui redémarre.
- **Touche rag3weaver** : oui (`Backend::open_with_reason`, `Backend::avertissements`, le rattrapage
  des vecteurs, `index_states`).

## Ce que c'est

Deux défauts, vus le 25 octobre 2026 sur l'hôte de Lucie après sa bascule de 15 h 44.

1. **L'embarqueur absent le reste.** Sans service à l'ouverture, `open_with_reason` pose un
   `AbsentEmbedder` dans le catalogue, et l'avertissement « pas de service d'embarquement ». Le
   service revient (les tunnels 7979-7981 sont reposés à 15 h 45), mais rien ne réessaie : l'index de
   15 h 46 pose 0 vecteur, la dette reste, la recherche dense se replie, et `daemon_status` garde
   l'avertissement. Seule une réouverture du backend reprend le service.
2. **L'index vectoriel retiré ne revient pas sans index.** Après la réouverture, la dette (2377
   morceaux) dépasse le seuil de 2000, et l'index `Scope_Chunk_vec__granite_278m` est retiré « le
   temps du rattrapage ». La montre embarque 554 morceaux, puis plus rien. L'index vectoriel reste
   absent jusqu'à un `index` lancé à la main. Pendant ce temps, l'état dit « vectors running 97 % »
   avec un `updated_ms` figé.

## Le témoin (à écrire avec le lot)

Un service d'embarquement absent à l'ouverture, puis présent :
- l'avertissement disparaît ;
- un index pose les vecteurs ;
- la recherche dense répond.

Puis une dette au-delà du seuil rattrapée par la montre : l'index vectoriel est recréé, et tant qu'il
est absent, l'état le dit en clair.

## Pour le fermer (la forme décidée par l'orchestration)

- Un backend sur `AbsentEmbedder` retente `connect_embedder` au plus une fois par N secondes, à chaque
  état (`daemon_status`, `index_state`) et avant chaque index. En cas de succès : `Catalog::set_embedder`,
  l'avertissement tombe, et la dette se rattrape à l'index suivant.
- Un index vectoriel retiré pour rattrapage est recréé dès que la dette est rattrapée, par la montre ou
  par un index. Tant qu'il est absent, l'état le dit (« index vectoriel retiré, rattrapage : n
  restants »), jamais « running 97 % ».
