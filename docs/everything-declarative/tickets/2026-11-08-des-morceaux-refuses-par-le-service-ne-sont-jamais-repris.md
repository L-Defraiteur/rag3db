# Des morceaux refusés par le service pendant un index ne sont jamais repris par le rattrapage

- **État** : ouvert (8 novembre 2026, 22 h 40, à la demande de l'orchestration) ; lot à faire
  derrière les vues, chez everything declarative ; à prouver par un témoin rouge
- **Famille** : produit
- **Gravité** : perte — une panne du service d'embarquement pendant un index laisserait des
  morceaux sans vecteur pour de bon, comptés en dette, jamais repris
- **Atteignable en service** : oui, par un service qui répond en erreur (500) pendant un index ;
  peut-être un second facteur dans wt-react5 (resté à 15 % de vecteurs après sa réouverture)
- **Touche rag3weaver** : oui

## Ce qu'on a vu

Le témoin `owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`, dans une forme
intermédiaire (cb996541d, branche temoin-rattrapage-service) : un premier index sur 303 fichiers
dont le service déclaré au manifeste refuse chaque texte (erreur 500), le débit noté d'avance pour
passer la sonde. La dette compte alors **606** morceaux. On rouvre ensuite avec un service sain et
`embeddings.catch_up_on_open` : trois index repartent à l'ouverture, et le dernier finit
« indexation terminée » avec « vecteurs **0 sur 606** » — il n'a rien réclamé.

Avec le même témoin **sans aucun service** au premier index (aucun embarquement tenté), le
rattrapage soldait toute la dette (modules verts, luciepc, 8 novembre, 20 h 21). La différence est
donc le refus : un morceau refusé reste compté par `count_marqueur_manquant`, mais n'est plus
réclamé.

## À lire

- `Catalog::preparer_l_embarquement` (catalog.rs, ~4104) : quels morceaux il réclame, et les
  `exclus` qu'il reçoit (`suivi.silent_uuids()`, ~4113) ;
- ce que l'embarquement écrit pour un morceau sans réponse ou en erreur (`EmbedWatch`,
  dataflow/record_nodes.rs : `silent`, `errors`, `empty`) : un marqueur posé, une valeur vide, un
  état noté en méta qui survit à la passe ;
- `count_marqueur_manquant` (catalog.rs, ~3983) contre la condition de réclamation : pourquoi l'un
  compte ce que l'autre ne prend pas.

## La forme attendue

Un morceau refusé redevient réclamable au prochain index (ou à un délai), et la dette qu'on dit est
celle qu'on peut solder. Témoin rouge d'abord : un service qui refuse pendant un premier index,
puis un service sain ; la dette tombe à zéro.
