# Un service d'embarquement injoignable à l'ouverture fige le backend en plein texte pour la vie de l'hôte

- **État** : **corrigé** par le lot `embarqueur-revenu` (25 octobre 2026, G) : un backend ouvert sans son service le retente au plus une fois par 30 s, à chaque état et avant chaque appel ; le service revenu, l'hôte (et la porte autonome) rouvre le backend avant de servir l'appel. Témoin : `e2e_demon_hote::un_service_revenu_apres_l_ouverture_est_repris`, rouge sans le correctif, vert avec.
- **Gravité** : réponse dégradée sans fin, et une dette qui grossit en silence
- **Atteignable en service** : oui, dès qu'un hôte démarre avant le service d'embarquement
  qu'il déclare. C'est le cas d'un démarrage de luciepc, où l'hôte et les services partent
  ensemble.
- **Touche rag3weaver** : oui, et seulement (`src/daemon_host.rs`, `src/backend.rs`)

## Ce que c'est

Depuis le lot `embarqueur-absent-portes`, un backend dont le service d'embarquement est déclaré
mais injoignable à l'ouverture s'ouvre en plein texte seul, avec l'embarqueur absent. Le reçu le
dit : « service déclaré mais pas joint (…) — démarrez-le ». Avant, l'ouverture échouait, et
l'appel suivant réessayait. Maintenant, rien ne retente. Un service lancé après l'hôte n'est
jamais repris, et les vecteurs restent en dette jusqu'à une réouverture du backend.

## La recette

Un hôte qui ouvre un gabarit de code dont `models.embed.address` désigne un port où rien
n'écoute encore. On démarre ensuite le service à cette adresse, puis on appelle `search_code` :
le signal dense reste indisponible, et `index_state` garde l'avertissement.

## Le témoin

Il manque. Il faudrait un e2e du cycle de vie qui lance l'hôte avant un démon d'embarquement de
test au port déclaré, démarre le démon, puis exige au plus tard le second appel servi par un
backend rouvert, l'avertissement retiré du reçu.

## La forme probable (arbre principal)

L'hôte retente `connect_embedder` au prochain appel quand l'avertissement est « pas joint »,
pas quand rien n'est déclaré. Si le service répond, il marque le backend à rouvrir, par le
`must_reopen` qui existe déjà. La réouverture pose le vrai embarqueur, et la dette se paie.

## Ce qu'il faut pour le fermer

La reprise ci-dessus et son témoin.

## Revu le 25 octobre 2026

Vu en service sur l'hôte de Lucie, réveillé par sa socket avant ses tunnels d'embarquement : un
index sans aucun vecteur, la recherche dense en repli, et l'avertissement resté faux dans
`daemon_status`. Seule une réouverture du backend l'a réparé. Le récit et un second défaut, l'index
vectoriel retiré pour rattrapage qui ne revient pas sans index, sont dans
`2026-10-25-un-embarqueur-absent-a-l-ouverture-le-reste-et-son-index-vectoriel-retire-ne-revient-pas.md`.

