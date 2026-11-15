# Recommencer une écriture refusée par un ALTER validé sur sa table

- **État** : ouvert (15 novembre 2026) ; à faire après l'atterrissage du lot du cœur (l'ALTER qui
  attend les écrivains) et la bascule de lib qui le porte
- **Famille** : produit
- **Gravité** : perte. Sans ce lot, un paquet refusé échoue, et la synchronisation se dit perdue
- **Atteignable en service** : oui, dès cette lib : tout ALTER validé (une colonne ajoutée par le
  chemin additif, `poser_les_generations`, un champ déclaré) pendant une écriture ouverte sur sa table
- **Touche rag3weaver** : oui (`src/code_sync.rs` : la transaction par paquet de `synchroniser`, et
  les instructions de l'écrivain des générations)
- **Pour** : everything declarative (ED)
- **Signalé par** : c7 (15 novembre 2026, 8 h 1x), à partir du lot du cœur

## Ce que c'est

Le moteur refuse désormais, à sa première prise de verrou, une transaction d'écriture ouverte avant
qu'un ALTER soit validé sur sa table. Le refus est nommé : « The schema of table T changed after this
transaction started …; retry the transaction ». L'écrivain et `sync_source` doivent **recommencer** sur
ce refus, qu'ils reconnaissent à son préfixe, au lieu de perdre l'écriture ou de la dire perdue.

## Les deux grains

- **La transaction par paquet** (chemin d'instantané, en mode masse) : le paquet entier se recommence,
  à partir de sa lecture, puisque l'annulation a tout rendu.
- **Les instructions validées seules** (l'écrivain des générations, une resynchronisation) : chacune
  est sa propre transaction, et elle se recommence seule. À vérifier dans le lot du cœur : un refus
  peut-il tomber au milieu d'une instruction d'ingestion à plusieurs COPY ? Si oui, c'est l'écriture
  du fichier qu'on recommence, et l'écrivain la rejoue sans dommage (§ 2 de la page 06).
- Le nombre de reprises reste borné et dit dans le rapport, pour qu'un ALTER en boucle ne fasse pas
  tourner une synchronisation sans fin.

## Le témoin, rouge d'abord

Un ADD COLUMN validé pendant un paquet : le paquet se recommence et va au bout ; aucune ligne n'est
perdue ni doublée, et le rapport dit une reprise.

## Ce qu'il faut pour le fermer

Le témoin, rouge sur la lib du cœur sans le lot, puis vert ; les e2e de la synchronisation verts.
