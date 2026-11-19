# Des morceaux refusés par le service pendant un index ne sont jamais repris par le rattrapage

- **État** : fermé (19 novembre 2026, everything declarative) ; ouvert le 8 novembre 2026, 22 h 40 ; corrigé le
  11 novembre par `b3f31b544f`, trouvé par bisection
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

## Fermé le 19 novembre 2026 (ED) : corrigé le 11 novembre, prouvé par bisection

**Corrigé par `b3f31b544f`** (11 novembre 2026, « rendre les réclamations à la sortie de l'index »). La cause, lue le
9 novembre : les réclamations d'une passe restaient au nom de son écrivain, et le catalogue rouvert, un autre
écrivain, les sautait une minute. Ce n'était pas l'état « silencieux » d'`EmbedWatch` : un refus du service (une
erreur, pas un délai) n'y est pas noté.

**La preuve**, à lucie-tablette, avec la lib `7c2b506ec` :
- **la forme où le ticket l'a vu**, le rattrapage à l'ouverture, a maintenant son témoin :
  `daemon_host::worktrees::tests::chunks_refused_by_the_service_are_caught_up_on_open`. 303 fichiers, un service
  qui refuse tout, puis une réouverture avec un service sain et `embeddings.catch_up_on_open` ;
- **rouge à `cb996541d`**, le commit du ticket : 606 vecteurs dus après le rattrapage, le chiffre même du ticket ;
- **vert sur master** ;
- **la bisection** (`git bisect run`, treize pas dont un sauté (non compilable), la forme courte du témoin) désigne `b3f31b544f` comme le premier
  commit corrigé ;
- le témoin de ce commit, `refused_chunks_are_retaken_by_the_next_index_after_a_reopen`, couvre la reprise par un
  `index` explicite. Les deux sont verts ensemble.
