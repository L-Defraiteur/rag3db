# Ordonner les entités dérivées à l'ouverture du backend

- **État** : ouvert (11 novembre 2026), capacité à vérifier pour l'intégration déclarative du lot 1a.
- **Famille** : produit
- **Gravité** : planification et risque de refus à l'ouverture d'un manifeste dérivé ; pas de reproduction e2e encore établie.
- **Atteignable en service** : à confirmer avec un manifeste déclarant une racine et sa dérivée.
- **Touche rag3weaver** : oui, src/backend.rs, enregistrement du schéma ; DerivedConfig et Catalog::verifier_la_derivation.
- **Pour** : everything-declarative ; arbre-principal porte le raccordement au lot 1a.
- **Signalé par** : arbre-principal, [ticket du contenu partagé](../../arbre-principal/tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).

## Éléments relus

PreparedBackend conserve les configurations dans un HashMap<String, EntityConfig>. Son ouverture
initialise un catalogue vide, puis appelle register_entity dans l'ordre de cette HashMap ; les
relations déclarées sont enregistrées après les entités. Catalog::verifier_la_derivation exige que
la racine soit déjà enregistrée et que les relations de gather existent. Ces contraintes doivent
être composées à l'ouverture, indépendamment de l'ordre des noms ou de la HashMap.

Repères de lecture sur la base du lot arbre-principal 49caf64c6 : backend.rs vers 961 et 2248–2308,
catalog.rs vers 1935. Les primitives natives du contenu partagé sont éprouvées par API Catalog,
mais cela ne prouve pas encore l'accessibilité depuis un manifeste de backend.

## Travail attendu et critères de fin

Poser un e2e minimal de chargement/ouverture de manifeste avec racine, index dérivé et association
publique search_index, via le chemin normal PreparedBackend. Reproduire ou invalider le risque
d'ordre ; réutiliser l'ordonnancement existant s'il couvre déjà ces contraintes, sinon composer
racines, relations puis dérivées selon leurs dépendances, avec refus explicite d'un cycle ou manque.
Prouver écriture sur la racine et recherche sur sa cible publique sans dépendre des noms ni des
semences de HashMap. Les dérivées avec gather doivent également avoir leurs relations disponibles.

Aucun cas spécial Scope, aucune activation des bases de service. Ce ticket est distinct des preuves
natives du lot 1a et n'autorise pas un nouveau lot de code sans coordination avec l'orchestration.
Le porteur peut invalider le risque avec sa preuve ; il ne doit pas être présenté comme reproduit avant ce témoin.

## Départage ED et choix sémantique

Sur master `fc3ca92aa`, témoins seuls `02948aebb`, trois rouges indépendants
(`0/3/0`, 0,11 s ; `I-luciepc/ouverture-20261111/derivees-rouge.log`) :

- Le chargement refuse l'identité héritée de la dérivée : exigence hashsafe
  alors que la validation locale interdit hashsafe sur une dérivée.
- Après isolation de ce verrou de chargement, l'ouverture refuse la racine
  absente quand la dérivée occupe réellement la première position de la map.
- Avec les racines effectivement premières, l'ouverture refuse la relation
  gather qui n'est enregistrée qu'après les entités.

Les témoins contrôlent l'ordre observé sans supposer une graine et sans répétition
aléatoire. Les deux derniers isolent la phase open en appliquant la déclaration
dérivée à une version préparée de racines ; le premier et le nouveau parcours
complet couvrent le vrai chargement de manifeste.

Plan privé d'initialisation, choix relu et validé par orchestration : entité dérivée
dépend de sa racine et de ses relations gather ; une relation dépend de ses
extrémités. Validation du plan complet au chargement et avant le premier appel
base à l'ouverture, tri stable et refus nommé des dépendances absentes/cycliques.
Le corps pur de Catalog.verifier_la_derivation est partagé dans
DerivedConfig.validate_context (résolveurs de configurations/relations), le
catalogue conserve sa vérification individuelle ; direction, champs et refus de
racine elle-même dérivée restent identiques. Pas de nouvelle chaîne ou cas Scope.
Seules les dérivées sont exemptées de l'exigence hashsafe (identité de la racine).

Source `be6646344` sur master `2e910930d` : **six témoins ouverture verts**
(0,60 s), dont chargement/ouverture réels, écriture racine puis recherche sur
la dérivée rassemblant un voisin, trois permutations des noms et zéro appel base
sur refus après préparation. L'essai précédent 5/6 lisait à tort un UUID ParLot
avant drain (Pending), corrigé dans la fixture, pas dans le produit.
Preuve : `cibles-2.log`. La passe avec/sans code sur `a166474a0` reste en cours.
Lecture croisée avec arbre principal demandée pour l'identité root/content ;
aucune source 1a importée. L'alias public search_index est absent de cette base :
son raccordement et sa preuve finale relèvent de la combinaison avec 1a et restent
à faire avant clôture de ce ticket.

Premier résultat `a166474a0` sans code : ouverture6/0/0 (0,58 s), module
builtin_templates9/0/0 (0,01 s). Lib native entière et ciblés avec code encore
en cours ; preuve `cibles-3.log`, aucun verdict global anticipé.

Ciblés terminés sur `a166474a0` : sans code, ouverture6/0/0 (0,58s),
gabarits9/0/0 (0,01s), lib native entière1575/0/15 ignorés (20,44s).
Avec code, ouverture6/0/0 (0,60s), gabarits9/0/0 (0,01s), aucun ignoré dans
les ciblés. `cibles-3.log`, `lib-native-3.log`, `verdict-cibles-3`.
Ce vert ciblé ne remplace pas la passe globale ni la lecture croisée avec 1a.
