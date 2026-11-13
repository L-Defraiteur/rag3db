# Activation et résidence des modèles : ce qui est livré, ce qui manque (13 novembre 2026)

*Page courte avant tout code, pour le ticket
[`gerer-activation-et-residence-des-modeles`](../tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md)
et la [vision du 12 novembre](../../orchestration/visions/2026-11-12-residence-des-modeles-et-prechauffage.md).
Relevé sur master `0977892ce`. Chemins relatifs à `extension/rag3weaver/`.*

## Ce qu'on veut, en une phrase

Un appelant demande un modèle par son identité ; le modèle est chargé s'il ne l'est pas, partagé
entre demandeurs, gardé tant qu'on s'en sert, et déchargé selon une politique déclarée (résident, à
la demande, préchauffé avant un besoin connu), sur le poste qui porte la carte — aujourd'hui
luciepc, card2.

## Livré

| quoi | où | limite |
|---|---|---|
| **Déclaration** : `ModelSource.activation` (programme, args, env, profil, périphérique, dossier de coordination, journal, budgets d'activation et d'attente, `residence`) | `src/model_source/activation.rs` | `Residence` n'a qu'une valeur, `Maintenir` |
| **Acquisition locale unique sous concurrence** : un verrou fichier par adresse déclarée dans le dossier de coordination, un registre par processus ; deux demandeurs, un seul lancement | `src/serveur/activation.rs` (`acquerir`, `Demande::attendre`, `Bail`) | le poste du demandeur seulement |
| **Annulation isolée** : annuler une attente ne tue pas l'activation partagée | `Demande::attendre(annule, budget)` | — |
| **Budgets séparés** : chargement (activation) et attente d'un demandeur | `budget_activation_secs`, `budget_attente_secs` | — |
| **Identité vérifiée avant remise** : modèle, dimension, et l'attestation de recette/profil/périphérique (`IdentiteActivation`, empreinte blake3) ; un profil étranger est refusé | `connecter`, `src/daemon/embeddings.rs` | une attestation, pas une preuve de placement physique |
| **Chauffe observable** : `Identite.chauffe` par capacité, état `non_executee / desactivee / complete / partielle`, durée, formes et erreurs | `src/daemon/embeddings.rs` (`Chauffe`) | décrite par le fournisseur, pas pilotée |
| **Port 0 annoncé** (aujourd'hui) : le fournisseur prend son port, l'adresse est retenue au dossier de coordination et resondée | `00e9fde57` | — |
| **Capacité couverte** : le **dense**, par `connect_embedder` | `src/model_source.rs` | les autres capacités refusent une activation déclarée (« autre capacité non encore prise en charge ») |

## Manquant

1. **L'activation distante.** Les services sont à luciepc, les clients aussi sur le portable (par
   les tunnels 7979-7981). `acquerir` exige une adresse locale et lance un processus sur le poste du
   demandeur ; rien ne démarre un fournisseur sur l'autre poste. Il faut un acteur côté luciepc qui
   reçoive la demande (un « gestionnaire de résidence », ou les unités systemd pilotées), avec un
   propriétaire autorisé à lancer et arrêter.
2. **L'éviction.** Aucun déchargement : pas de délai d'inactivité, pas de budget de VRAM, pas
   d'hystérésis. Il faut savoir quand un modèle n'a **plus d'usage en cours** (requêtes, attentes,
   chargements) avant de l'arrêter.
3. **Les baux d'usage entre processus.** `Bail` ne vit que dans le processus qui l'a pris ; un
   fournisseur partagé par plusieurs processus (l'hôte, les passes, les hooks) ne sait pas combien
   d'usagers il a. Prérequis de l'éviction.
4. **Les politiques** « à la demande » et « préchauffage anticipé » : à déclarer dans `Residence`, à
   mesurer (coût de reprise à froid : granite-278m ~6 s, bge-m3 + relecteur + OCR ~14 s).
5. **Les autres capacités** : creux, dual, relecteur, OCR. Le groupe BGE/relecteur/OCR est un seul
   processus et se décharge d'un bloc tant qu'il n'est pas séparé.
6. **La reprise après plantage** : un fournisseur mort sous un bail (verrou, adresse retenue
   périmée) — le port 0 resonde désormais l'adresse retenue, mais aucun usager attaché n'est prévenu.
7. **Les services de card2 eux-mêmes** : les trois unités systemd sont résidentes en dur
   (`Restart=always`) ; aucune n'est encore déclarée par une `Activation`.

## Ordre proposé

Chacun se mesure avant le suivant ; aucun ne change les services en place sans fenêtre annoncée.

1. **Baux d'usage entre processus** (3) : un compte d'usagers tenu par le fournisseur (acquérir /
   rendre par HTTP, comme `/sante`), visible dans son identité. C'est la mesure qui manque à tout le
   reste ; rien n'est déchargé à cette étape.
2. **Éviction à la demande, locale** (2, 4) : sur un fournisseur lancé par l'activation (jamais sur
   un service rejoint), déchargement après N minutes sans usager, N déclaré ; témoin CPU d'abord.
3. **Activation distante** (1) : décision de conception à prendre avec Lucie (qui lance quoi à
   luciepc : les unités systemd existantes pilotées par `systemctl --user`, ou un gestionnaire à
   part), puis le même contrat de bail traversant le tunnel.
4. **Les autres capacités** (5) et la déclaration des services de card2 (7) ensuite.

Questions à trancher avant le code : le propriétaire des fournisseurs à luciepc (point 3), et si
l'éviction peut toucher les trois services d'aujourd'hui ou seulement de nouveaux fournisseurs
déclarés.
