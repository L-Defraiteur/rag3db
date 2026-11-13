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

## Fait depuis : les baux d'usage (master `7042128d0`)

Le fournisseur compte ses baux et ses calculs en cours et le dit dans `/sante`
(`usage: { baux, en_cours }`) ; l'activation prend un bail par client remis ; `/sante` se
recalcule à chaque demande. **Pour les trois services de luciepc, la pose de ce soir ne change
rien tant qu'ils ne sont pas redéployés** : ils tournent sur un binaire d'avant les baux, et les
clients tolèrent un tel fournisseur (pas de compte, pas d'erreur). Décisions de c7, à relire par
Lucie : les fournisseurs à luciepc sont les unités systemd, et l'éviction ne touchera jamais les
services posés à la main (7878, 7879, 7880, 7882).

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

## L'activation distante : les règles (à relire par Lucie)

*13 novembre au soir, avant tout code. C'est la première fois qu'un poste en pilote un autre :
le portable lance un fournisseur sur luciepc. Décision de c7 : par ssh vers `systemctl --user`
de luciepc, rien d'autre à inventer. Fait depuis : l'éviction locale (master `0ee027cf0`).*

**La déclaration.** Une `Activation` qui porte `hote: "luciepc"` (une cible ssh) et
`adresse_distante: "127.0.0.1:<port>"` est distante. Son `address` reste locale : c'est le bout
du tunnel par lequel le client la joint (`127.0.0.1:<port local>`). Le programme, les arguments,
l'environnement, le profil et le périphérique sont ceux du poste distant.

**Les unités.** L'activation lance le fournisseur par
`ssh <hôte> systemd-run --user --unit=rag3weaver-fournisseur-<modèle>-<port> --collect --property=Restart=on-failure --setenv=… -- <programme> --adresse <adresse distante>`.
- Une **unité transitoire** : aucun fichier n'est écrit à luciepc, `--collect` l'efface quand
  elle s'arrête.
- **Son propre préfixe**, `rag3weaver-fournisseur-` : l'activation ne touche, n'arrête ni ne
  remplace jamais une unité `rag3weaver-embeddings@…` (les services posés à la main).
- **`Restart=on-failure`** : un plantage relance (borné par systemd), une éviction (sortie en
  code 0) ne relance pas.
- **Un port déjà pris** sur luciepc, par un fournisseur qui n'est pas celui déclaré (identité ou
  recette différente) : refus nommé, rien n'est lancé. Pris par le même : on s'y attache.

**Le chemin du client.** L'activation ouvre elle-même un tunnel,
`ssh -N -o BatchMode=yes -o ExitOnForwardFailure=yes -L <address>:<adresse distante> <hôte>`,
tenu par le bail local et fermé avec lui. L'identité du fournisseur (`/sante`, son attestation de
recette, `usage`, `eviction`) se lit par ce tunnel, comme pour un fournisseur local ; le port
distant est déclaré, pas annoncé (un port 0 distant demanderait de lire le journal de l'unité).

**Si ssh échoue** (hôte injoignable, clé refusée, `systemd-run` en erreur) : refus net et nommé,
`ssh -o BatchMode=yes -o ConnectTimeout=5`, aucune attente ni nouvel essai ; l'appelant voit
« activation distante : ssh <hôte> : <cause> ». Pas de repli local : le repli reste explicite.

**La carte.** Le service calcule en Vulkan : c'est `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0` qui le
place sur card2 à luciepc (`gpu:0` = card2 en Vulkan) ; `ROCR_VISIBLE_DEVICES` ne vaut que pour
ROCm et ne place rien ici. Vérifié par la mémoire des deux cartes pendant le vrai jeu.

**Ce que Lucie voit et comment elle arrête** (une commande chacune, depuis le portable) :
- voir : `ssh luciepc systemctl --user list-units 'rag3weaver-fournisseur-*'`
- arrêter : `ssh luciepc systemctl --user stop 'rag3weaver-fournisseur-*'`

**Les témoins.** Rouge d'abord sans luciepc : un faux `ssh` (un script qui journalise ses
arguments et exécute la commande en local) donné par une variable de test ; il prouve l'unité
transitoire, son préfixe, `Restart=on-failure`, le refus net d'un ssh en échec, et qu'aucune unité
`rag3weaver-embeddings@` n'est nommée. Puis le vrai jeu, une fois, à luciepc sous `poste lourd` :
granite-107m (plus résident depuis le 12 novembre, 7880 libre, ~4 Go sur card2 qui en a 13 de
libres), port distant 7890, tunnel local 7990, bail pris puis rendu, éviction à 60 s observée,
l'unité effacée ensuite.
