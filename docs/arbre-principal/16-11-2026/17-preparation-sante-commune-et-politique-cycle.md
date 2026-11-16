# Préparation du transport santé commun et de la politique cycle

État : préparation, pas de GO ni de qualification native. Qv3/b9e5 est figé et
retiré des candidats au lancement après contrelecture C7. Son manifeste et ses
preuves ne sont pas modifiés. Les anciens rouges restent rouges.

Le ticket READY reçoit le défaut avant modification de code (7f8ea6a24c).
Six vrais appels de `qualify.health` hermétiques sont publiés dans 8640f74873,
puis vus rouges : égalité de booléens et entiers, dimension float, durée totale,
et absence de bruts lors des refus JSON, statut et taille. f7bb59662e ajoute le
raccord AST réel après writer : une phase terminée atteint encore le GET fictif.
Sept échecs en 0.019s après publication ; aucun réseau, processus enfant,
signal réel, /proc, DB ou CLI.

La frontière commune est `provider_health_transport.py`, sous coordination.
Le mécanisme fait uniquement un GET local `/sante` sur l'adresse explicitement
admise, sans proxy/redirection. Il charge la policy dont chemin, propriétaire
et SHA sont contrôlés, puis l'appelle juste avant contact. La borne absolue
couvre le worker complet et sa récolte ; les bruts, statuts, sorties partielles
et rc sont conservés avant refus. Les traces attestent seulement les octets
effectivement reçus et leur préfixe borné ; un timeout ne prouve jamais un corps
complet. La validation commune exige les neuf champs
avec types exacts et les deux compteurs usage entiers zéro. Usage zéro est un
constat ponctuel, jamais un compteur de RPC cumulés.

Les politiques restent distinctes : le global garde son admission PG ; le
cycle garde le READY exact, son nonce, la provenance et l'interdiction
embedding. Son consommateur s'appuie sur les pins owner canoniques de la même
observation, recoupés avec les copies Q préparées. Avant writer, le grant doit
encore passer la fenêtre d'admission READY. Après writer, la même liaison et
la lease active sont contrôlées sous autorité vivante, sans rouvrir la fenêtre.
Le protocole factorise cette liaison dans `validate_admitted_grant_phase` ;
`validate_grant_phase` ajoute la fenêtre READY. Les checks sont refaits après
la réponse et avant acceptation du résultat.

Destination proposée : lifecycle-copy-v4 / lifecycle-control-v4 /
lifecycle-scopes-v4 ; 19 sources prévues (17 précédentes, policy cycle,
transport commun). Aucun dossier de runtime, allocation, grant ou contact
n'est créé par cette préparation. Le budget proposé reste 900, avec mêmes
bornes de phases et réserve RO ; aucun ancien GO n'est transféré. La source
finale, ses contrôles CPU et le nouveau paquet devront recevoir une RO totale
puis un arbitrage C7 distinct avant tout lancement.

Portage groupé : quatre fichiers byte-identiques à
bcfd1934ac44153b26ea5832f2bd7415ea83ded2, dont le transport commun strict
443b5a039c. SHA transport edf45f73bf97d21f2a0caab4d6e07a7d01f807fba8310c9006f442159e2bc2a6 ;
SHA owner 83139ebb329a59f19aa881f7f9678a7e6bf87c39abd3ff0ab7d20ecc15c0ba00.
Aucun fichier global/PG n'est porté. Le manifeste v4 est recoupé avec BASE
par un garde et un témoin. Le contrôle CPU du groupe suit sa publication ;
aucun résultat vert du consommateur n'est encore revendiqué ici.
