# Préparation du transport santé commun et de la politique cycle

État : préparation avec qualification CPU hermétique, pas de GO ni de qualification native. Qv3/b9e5 est figé et
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

## Contrôles CPU après publication

Source d3f2d6acd2 publiée puis suite de 84 fonctions : 80 réussites et quatre
erreurs de setup en 0.430s, rc1. Erreur locale du nouveau témoin : une
substitution textuelle avait construit une clé tuple au lieu des deux pins
observation/transport. Aucun corps des quatre juges policy n'avait été atteint ;
ce rouge reste attaché à d3f2. Les sept contre-exemples initiaux étaient verts.

Correction fixture seule publiée 3f3af87898 : quatre juges policy verts en
0.006s, puis groupe complet 84 fonctions distinctes vertes en 0.431s, rc0.
Les sous-cas ne sont pas comptés comme nouvelles fonctions. Commandes bornées
par timeout (59s+kill1 pour le premier, puis 20s+kill1 pour les contrôles après
correction), toutes terminées en moins d'une seconde réelle chacune.

Commande du groupe : `python3 -B -m unittest test_lifecycle_protocol
test_lifecycle_writer test_lifecycle_admission test_ready_contract
test_ready_consumption test_lifecycle_grant_producer
test_lifecycle_provider_observation test_consumer_health
test_consumer_health_policy test_provider_health_transport`, avec PYTHONPATH
pointant seulement sur le dossier coordination du transport commun.
Aucun test supplémentaire après la correction verte n'est nécessaire sans
nouveau delta de code. Les modifications documentaires ultérieures ne sont pas
présentées comme de nouveaux runs.

Les fakes couvrent le vrai transport worker, le vrai wrapper owner, le vrai
consommateur et son raccord AST après writer, ainsi que la policy cycle. Ils
ne qualifient aucun PID, GET, DB, CLI, service, retrait ou arrêt réel. Aucun
embedding autorisé ; ticket READY ouvert, nouveau paquet et RO totale avant
arbitrage C7 distinct. Aucun ancien GO transféré et aucun GO900 consommé.
