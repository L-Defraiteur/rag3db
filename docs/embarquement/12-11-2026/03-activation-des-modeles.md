# Première acquisition gérée d'un modèle

Ticket : [activation et résidence](../tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md).
Cette tranche est en qualification, pas encore intégrée à master. Elle compose
`ModelSource`, `Serveur` et `Attache` ; aucun service installé n'est modifié.

## Déclaration et appelant

Un manifeste peut porter `models.embed` avec `provider: service`, une adresse
locale numérique fixe, le modèle et sa dimension. Le champ optionnel
`activation` déclare explicitement le lancement :

```json
{
  "provider": "service",
  "model": "granite-107m",
  "dimensions": 384,
  "address": "127.0.0.1:7895",
  "timeout_secs": 60,
  "activation": {
    "programme": "/chemin/vers/rag3weaver-embeddings",
    "args": [],
    "env": {"RAG3WEAVER_BURN_DEVICE_EMBEDDER": "gpu:0"},
    "profil": "granite-107m-flex32-prive",
    "peripherique": "card2-pci-0000:04:00.0",
    "coordination": "/chemin/prive/activation",
    "journal": "/chemin/prive/journaux",
    "budget_activation_secs": 300,
    "budget_attente_secs": 310,
    "residence": "maintenir"
  }
}
```

Exemple de forme, pas une commande à lancer sur les services de Lucie. Le
profil et le périphérique sont des identités déclarées : le placement réel
reste à vérifier par les allocations et le calcul du processus.
Les secrets ne se déclarent pas dans `activation.env` ; utiliser leur voie de
fournisseur déjà prévue. Les champs réservés d'identité ne sont pas remplaçables.

`Backend::connect_embedder` utilise déjà `model_source::connect_embedder`.
Ce connecteur prend l'activation déclarée ; sans elle il garde son comportement
antérieur de connexion seule. `connect_embedder_annulable` reçoit l'annulation
d'un demandeur. Les autres capacités refusent une déclaration d'activation
encore non prise en charge. Un service distant ou une adresse de tunnel ne
constitue pas une recette d'activation distante.

## Garanties et frontières

Le programme reçoit `--adresse H:P` puis les arguments déclarés. Les demandes
d'un même processus rejoignent le chargement commun ; un verrou fichier
sérialise les activations entre processus utilisant le même répertoire. Chaque
activation re-sonde sous ce verrou avant de lancer. Ne pas supprimer ces fichiers
pendant les usages : recréer un inode contournerait le verrou existant.

Le worker possède le chargement, pas le demandeur. Annuler une attente ne tue
pas le fournisseur d'un autre. Le budget de démarrage comprend la coordination
et les sondes HTTP avec une échéance absolue. Le budget d'attente est propre au
demandeur ; son restant borne la sonde finale d'identité. Le délai des appels
au modèle vient ensuite de `timeout_secs` ou du défaut du connecteur.

Avant de rendre le client dense, le connecteur vérifie modèle, dimension et
l'attestation de recette publiée par le fournisseur courant. Cette attestation
porte profil, périphérique déclaré et empreinte de la recette ; un fournisseur
ancien sans attestation ou un autre profil sont refusés, sans être arrêtés.
Un fichier administratif persistant ne certifie pas le processus qui a repris
un port. L'empreinte ne remplace ni une preuve de placement ni une authentification.

Le client garde un bail local. La politique initiale est uniquement `maintenir` :
la fin d'un client ne tue pas le processus. Il n'y a pas de TTL caché, d'éviction,
de comptage global des usages ni de contrôle des unités distantes dans cette
tranche. Le porteur d'un processus privé conserve sa recette d'arrêt.

## États et chauffe

`model_source::activation::demander` expose une `Demande` avec les phases
coordination, activation, disponible ou échec. `attendre` rend un `Bail` ou une
erreur d'annulation/délai. Disponible signifie que le service HTTP répond ; le
connecteur dense contrôle ensuite l'identité. Cela ne signifie pas « tout chaud ».

La santé du fournisseur porte séparément `chauffe`, une table par capacité.
Chaque entrée a un état (`non_executee`, `desactivee`, `complete`, `partielle`),
une durée et les formes réellement tentées (`lot`, `mots`, erreur éventuelle).
Une réponse de chauffe vide, de mauvaise forme ou non finie reste une erreur.
OCR et relecteur présents ne deviennent pas chauds par la chauffe du dense.
L'absence du champ chez un ancien fournisseur est une information inconnue.

## Preuves disponibles

- Validation confinée `validation-jqwupheq` : 17 témoins standalone,
  5 tests du connecteur réel avec fournisseur CPU factice et un de chauffe
  partielle, tous passés sans ignoré. Deux processus distincts partagent
  un départ ; l'annulation d'un demandeur préserve l'autre.
- Banc réel granite-107m : `validation-bvdguacm`, acquisitions concurrentes,
  annulation indépendante, réattachement et nouveau départ après arrêt privé.
  Deux activations, chauffe16/16, vecteurs384 non factices ; six captures
  avec résidence et calcul PCI04, aucun FD PCI07 observé. Le détail et les
  limites de latence figurent dans le [rapport du banc](../../banc/12-11-2026/03-resultats-residence-privee.md).
- Fournisseurs BGE et granite-278m : `validation-tbwulyl4`, deux tests réels
  sans ignoré, observateur attaché avant les départs et actif jusqu'à leur fin.
  80 captures complètes, résidence et calcul PCI04 pour les deux processus ;
  aucun client PCI07 observé. Une disparition `/proc` reste une donnée inconnue.
- Union avec la navigation8f604 : `validation-qdm6acnb`, cinq jeux de features,
  lib2258 passés/65 ignorés structurels non qualifiés, navigateur40/40 ;
  fournisseurs privés2/2 et physique2/2 sous `validation-uu18g72q`, sparse
  réel1/1 sous `validation-twt84wn7`. Aucun service installé relancé.
- Union publication/ED : F5 verts puis rouge `validation-hex7oup7` sur
  la carte show_progress (lib2272/1/66), diagnostic confié à ED. Les
  fichiers du producteur, connecteur, activation et observateur sont identiques
  à ceux déjà qualifiés. Aucune extension de preuve physique à un composant modifié.

Les recettes, refus et échéances restent consignés dans le ticket. Celui-ci
reste ouvert pour les politiques distante/éviction, baux distribués et reprise
après crash ; la tranche dense locale `maintenir` ne les implémente pas.
