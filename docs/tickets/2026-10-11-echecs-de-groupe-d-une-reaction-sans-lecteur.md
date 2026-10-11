# Les échecs de groupe d'une réaction ne sont lus par personne, et s'accumulent

- **état** : ouvert (relevé en écrivant le montage des réactions, 11 octobre 2026)
- **gravité** : perte d'information ; croissance sans borne d'un vecteur en mémoire
- **atteignable en service** : oui, dès qu'un manifeste déclare une réaction qui écrit
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

Le canal d'échecs de groupe (`records::SERVICE_ECHECS`, un
`Arc<Mutex<Vec<EchecDeGroupe>>>`) est ouvert par `Catalog::register_search_services`
pour tout graphe que le backend exécute. Les chemins du catalogue qui l'ouvrent
eux-mêmes le **relèvent** après le graphe (`FlushResult::absorber_les_echecs`) ;
le backend, lui, ne le relève jamais.

Pour un appel d'outil c'est une perte bornée : un registre neuf par appel, donc le
vecteur meurt avec l'appel — ce qu'il contenait n'est simplement dit à personne.
**Pour une réaction c'est pire** : le réacteur garde *un* `ServiceRegistry` pour
toute la durée de son service (`Reactor::new(bus, nodes, Arc<ServiceRegistry>)`),
donc le même vecteur reçoit les échecs de toutes les exécutions, sans que rien ne
le vide. Un backend à réaction qui tourne des jours accumule.

## La recette minimale

Pas de recette courte : il faut une réaction qui écrive et dont une écriture de
groupe échoue. La lecture suffit à l'établir — `grep -rn "SERVICE_ECHECS" src/`
donne un seul site d'ouverture hors catalogue (`register_search_services`, par
`ouvrir_le_canal_d_echecs`) et aucun site de relève côté `backend.rs`.

## Le témoin

Aucun, assumé. Pour l'écrire il faudrait un nœud d'écriture qui pousse dans ce
canal sur un chemin que le backend emprunte, puis compter la taille du vecteur
après deux exécutions de la même réaction. C'est faisable mais ce serait un témoin
de plomberie : la vraie correction supprime la question.

## La cause

Deux décisions qui ne se sont pas vues :

1. `execute_plan` construit un registre **neuf par appel** (« services are fresh so
   shutdown owns all handles ») et n'a donc jamais eu à vider quoi que ce soit ;
2. le réacteur prend son registre **une fois** et le prête à chaque exécution — il
   n'a aucun point d'accrochage « après une exécution » où relever un canal.

Le montage des réactions (`Backend::mount_reactions`) partage l'assemblage des
services avec `execute_plan` (`Backend::build_services`), ce qui est la bonne
chose — une réaction validée au chargement doit trouver les mêmes services qu'un
outil — mais le partage a transporté ce canal avec le reste.

## Ce qu'il faut pour le fermer

Au choix, par ordre de préférence :

1. **Que le réacteur relève le canal après chaque exécution** et pousse ce qu'il y
   trouve dans ses erreurs (`ReactorHandle::errors`), déjà lues par
   `Backend::reaction_errors`. Demande un crochet « après une exécution » dans
   `Reactor::fire_due` et un registre que le réacteur sait interroger — c'est la
   correction qui rend l'information *consultable*, pas seulement bornée.
2. Ou que le réacteur construise un registre par exécution, comme `execute_plan` :
   borne la croissance, mais perd toujours les échecs.

Tant que ce n'est pas fait, un échec de groupe survenu dans une réaction n'apparaît
nulle part : ni au journal, ni dans `reaction_errors()`, ni en base.
