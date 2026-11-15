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

## À la demande : les deux délais (13 novembre au soir)

Une déclaration `residence: { "a_demande": { "inactivite_secs": N, "rendre_le_bail_secs": M } }` :
- **`inactivite_secs`** (N) — côté fournisseur : sans bail ni calcul pendant N secondes, il se
  décharge (`RAG3WEAVER_EVICTION_INACTIVITE_S`, dit dans son identité `eviction`).
- **`rendre_le_bail_secs`** (M, défaut 30, `RENDRE_LE_BAIL_PAR_DEFAUT_SECS`) — côté client :
  l'embarqueur à la demande rend son client, donc son bail, après M secondes sans appel.
- M doit être non nul et **plus court** que N (sinon refus à la déclaration) : un client qui s'arrête
  d'appeler rend son bail avant que le fournisseur ne compte son inactivité jusqu'au bout.

Rien n'est lancé à l'ouverture ; le premier calcul active ; un calcul qui trouve le fournisseur
parti le relance une fois (identité revérifiée), puis refuse en le disant. `Maintenir` et les
services posés à la main ne changent pas.

*Note de passe (13 novembre) : `target-activation`, d'abord bâti dans le worktree `rag3db-spans`
(effacé), a été repris pour le worktree `rag3db-a-la-demande` par `RAG3WEAVER_TARGET_REPRENDRE=1`,
l'usage prévu de la garde des targets (elle efface les empreintes des paquets locaux et garde les
dépendances) ; sans lui, la garde refuse un target venu d'un autre arbre.*

## Les activations nommées et le groupe (14 novembre)

*Décision de c7, au mot de Lucie : « une abstraction, pas trois configs qui divergent ».*

- **La déclaration.** `activations: { "<nom>": {…} }` au niveau du manifeste. Chaque déclaration de
  `models` la cite par son nom : `"activation": "<nom>"`. Au chargement, le nom cité reçoit le bloc et
  son `nom` (`resoudre_les_activations`, `src/backend.rs`). Un nom absent est refusé en le nommant, avec
  la liste des noms déclarés. Une activation nommée dit le **`modele`** que son processus charge
  (`RAG3WEAVER_EMBED_MODEL`), parce que le modèle d'une déclaration du relecteur n'est pas celui du
  processus.
- **Un seul mécanisme.** Un bloc écrit dans la déclaration reste valable : c'est une activation nommée
  d'après son `model`. Le lot du 13 novembre ne change donc pas de forme.
- **La recette** attestée par le fournisseur devient `hash(nom, modele, programme, args, env, profil,
  peripherique)`. La dimension n'en fait plus partie ; elle se vérifie à part, pour le dense.
- **L'identité, par capacité** : le dense (modèle et dimension), le creux (modèle servi et `sparse`), le
  relecteur (`reranker` au `/sante`), l'OCR (`ocr` au `/sante`). Toutes avec la même recette.
- **Partagés** : deux déclarations qui citent le même nom, à la même adresse, partagent le processus,
  la coordination et l'éviction. Chaque capacité tient son bail ; le fournisseur s'évince quand plus
  aucun bail ne le tient.
- **Le comportement**, pour le creux, le relecteur et l'OCR, comme pour le dense :
  - à la demande, rien à l'ouverture ; le premier appel active ; le bail est rendu après
    `rendre_le_bail_secs` sans appel ;
  - maintenu, l'activation est immédiate ;
  - un appel qui trouve le fournisseur planté le relance une fois, ou se refuse en disant les deux
    temps ; `relances` compte.
- **Ce que ce lot ne fait pas** (tickets) :
  - ~~un creux à la demande n'a pas de dual~~ : le dual se déclare par `dimensions` depuis le
    14 novembre au soir (section « Le dual d'un creux à la demande » plus bas) ;
  - ~~le dense garde son embarqueur à lui à côté de la tenue générique~~ : porté sur `Tenue<C>` le
    14 novembre au soir, un seul mécanisme pour les quatre capacités.

**Le vrai jeu du groupe, 14 novembre, 18 h 46 → 19 h 06** : vert. Hôte isolé 7971 (binaire de la branche
4f43b712d7e61903), fournisseur de groupe à part sur card2, port 7891 (service 12783f97e8dbdd45), à la
demande (60 s, 30 s) ; le dense, le creux, le relecteur et l'OCR citent l'activation `bge-m3`. L'hôte de
Lucie n'a jamais été appelé.
- **L'ouverture** ne lance rien.
- **L'index 1** (80 fichiers, 2219 morceaux) active le groupe pendant l'appel. Identité au /sante :
  bge-m3 1024, creux, dual, relecteur bge-reranker-v2-m3, OCR ppocrv6-tiny, `relances` 0.
- **Une `search_code` relue** (20 candidats) relit dans le même processus.
- **Les baux** : jusqu'à deux tenus ensemble, rendus à +30 s ; l'éviction suit à +60,5 s.
- **L'index 2** réactive le groupe en 6 s ; puis les baux sont rendus et le groupe s'évince de même.
- **L'OCR** n'est pas exercé par l'hôte : aucun outil du backend de code ne l'appelle.

**La marge sur card2** : le groupe a tenu jusqu'à 11,4 Gio (le 7879 en place : 8,3), et card2 est
montée à 30 573 Mio sur 32 624, soit 2 Gio libres au pic. Règle (c7) : un groupe à côté des résidents
de card2 ne part qu'avec 13 Gio libres, ou le 7879 arrêté.

**L'index 1 a pris 904 s** (bge-m3 dense et creux, carte presque pleine, deux lourds à côté). La cause
n'est pas tirée. À faire : une mesure à part, card2 calme, pour départager le modèle et la carte pleine.
Scripts et journaux à luciepc : `~/.cache/rag3weaver-build/groupe-vrai-jeu/`.

## Le dual d'un creux à la demande : conception (14 novembre au soir)

*Ticket `2026-11-14-un-creux-a-la-demande-n-a-pas-de-dual.md`. À relire avant le code.*

**Ce qui est.** Ouvert sur un service posé, `connect_sparse` lit l'identité du fournisseur (`/sante`,
`dual`) et rend le dual s'il le sait. Le backend l'attache au catalogue si le modèle du creux est
celui du dense (`backend.rs`, `set_dual_embedder`). Un creux activé ne connaît pas cette identité
avant son premier appel : `connecter_creux` rend `dual: None`. Une requête dense et creuse sur le
groupe bge-m3 part alors en deux appels (`/embed`, puis `/embed_sparse`) au lieu d'un (`/embed_dual`).
Il en va de même pour l'ingestion.

**La règle proposée : le dual se déclare.**
- **Quand.** Un creux activé rend un dual quand sa déclaration dit sa dimension
  (`models.sparse.dimensions`). C'est la dimension du dense que le dual rend, la seule chose qu'un
  dual doit savoir sans fournisseur (`DualEmbedder::dim`). Sans `dimensions`, pas de dual, comme
  aujourd'hui.
- **Une seule tenue.** Le creux et le dual partagent la même `Tenue<C>` : un seul bail pour les
  deux, une seule activation, une seule éviction. Le dual n'ajoute pas d'usager.
- **L'identité, à l'activation.** Avec un dual déclaré, le fournisseur doit dire `dual: true` et
  une `dim` égale à la dimension déclarée. Sinon, refus nommé : « le fournisseur sert bge-m3 sans
  le dual », ou « en 768 au lieu de 1024 ». Rien n'est deviné.
- **Rien d'autre ne bouge.** Le backend attache le dual comme aujourd'hui, si le modèle du creux
  est celui du dense. Un groupe où le dense et le creux citent la même activation (`bge-m3`)
  embarque alors en un appel. Un dense d'un autre modèle (granite) garde son embarqueur, et le
  creux reste seul.

**Les témoins, rouges d'abord**, sur la fixture : `--dual` lui fait servir `/embed_dual` et dire
`dual: true`.
1. Un creux à la demande qui déclare `dimensions` rend un dual, et l'ouvrir ne lance rien.
2. Un appel au dual lance le groupe une fois (un seul `constructeur_demarre`, **un bail**). Un appel
   au creux ensuite ne relance pas et ne prend pas de second bail.
3. Un fournisseur sans dual, ou d'une autre dimension, se refuse en le disant.
4. Par le catalogue : une recherche dense et creuse sur ce groupe fait **un** appel `/embed_dual`, et
   aucun `/embed` ni `/embed_sparse` séparé.

**Le dual, livré (14 novembre au soir)** : la conception ci-dessus, telle quelle, plus la garde de c7 :
une dimension du creux différente de celle du dense est refusée au chargement du manifeste.

**Le vrai jeu du dense sur `Tenue<C>`, 14 novembre, 21 h 39 → 21 h 43** : vert, par un hôte isolé
(7971, binaire de dense-sur-tenue 1cd3200d5ccd81b8, lib 373b5477b).
- **Le fournisseur** : granite-107m à la demande, déclaré en bloc inline (une activation nommée d'après
  son modèle), sur card2, port 7892 ; jamais 7878.
- **Le déroulé** : rien à l'ouverture ; activation pendant le premier index (13,4 s) ; bail rendu à
  +29,6 s ; éviction à +59,8 s ; réactivation en 6 s au second index.
- **La mémoire** : card2 au plus 26 504 Mio (6,1 Gio libres au pire), le fournisseur au plus 5,7 Gio,
  card0 à 88 Mio. Le dense porté se comporte en vrai comme l'embarqueur d'avant.

## Le target d'une session gonfle (15 novembre)

Le 14 novembre à 23 h 56, `run_e2e.sh` a refusé l'e2e du groupe : `target-activation` pesait **34 Go**,
au-delà du plafond de 32 (`RAG3WEAVER_TARGET_MAX_GO`). Une soirée de rejeux sous plusieurs jeux de
features, sur plusieurs worktrees repris (`RAG3WEAVER_TARGET_REPRENDRE`), suffit à le faire gonfler.
**Le plafond ne s'est pas relevé** : un seuil ne se relâche pas sans preuve. Le target, à moi et sans
aucun processus dessus (vérifié par `/proc/<pid>/exe`), a été effacé, puis l'e2e rejoué sur la même
tête, vert. Règle : **le ménage du target avant une passe entière**, quand il approche du plafond
(`du -sh`), plutôt qu'à son refus.

## La mesure des 904 s, reportée (15 novembre, 02 h 57 et 04 h 03)

La mesure à part du premier index du groupe bge-m3 (904 s le 14 à 18 h 47) n'a pas été lancée. Deux
relevés de luciepc, avec les quatre conditions de c7 : card2 à au moins 13 Gio libres, card2 à 0 %,
aucune mesure tenue ni en attente, une place lourde libre.

| Heure | card2 libres | card2 occupée | `poste --qui` |
|---|---|---|---|
| 02 h 57 | 12 228 Mio | 0 % | trois lourds, une mesure en attente |
| 04 h 03 | 12 228 Mio | 0 % | deux lourds (dont `e2e_code`) |

**12 228 Mio est l'état de repos de card2**, avec les deux services résidents chargés (7878 granite-278m,
7879 bge-m3, environ 20,4 Gio tenus). Le seuil de 13 Gio, fixé pour un groupe de 11,4 Gio à côté
d'eux, ne peut donc jamais être atteint tant qu'ils tournent. Reporter à « une nuit sans file » ne
suffit pas.

**Ce qu'il faut** : le 7879 arrêté le temps de la mesure (un service de Lucie, donc son mot), ou un
seuil relevé, à prouver d'abord. Avec le 7879 arrêté, card2 aurait environ 20 Gio libres et la
comparaison serait propre : le même jeu, une carte calme. Sans cela, la cause des 904 s (le modèle ou
la carte pleine) reste non tirée.

Vu en passant : à 02 h 57 comme à 04 h 03, **card0** (l'affichage de Lucie) était occupée à 100 %, sans
lourd de nos sessions dessus. c7 l'a noté pour Lucie.

## La mesure des 904 s : la séquence de la fenêtre du matin (décision de c7, 15 novembre, 04 h)

Le 7879 est arrêté le temps de la mesure, une quinzaine de minutes, par G qui tient les services. La
règle de Lucie du 14 le permet : relancer un service quand le lot le demande, en l'annonçant. L'heure est
donnée par c7, avec la pose du binaire du matin.

**Ce qui est prêt à luciepc**, dans `~/.cache/rag3weaver-build/groupe-vrai-jeu/`, à l'identique du jeu du
14 à 18 h 46 pour que la comparaison tienne :
- les mêmes binaires : hôte isolé `4f43b712d7e61903`, service `12783f97e8dbdd45` ;
- les gabarits du commit d'alors (`gabarits-904/`, de 73262341b) ;
- le même dépôt (`depot-904/`, codeparsers/src au pointeur f413dc63, 80 fichiers) ;
- `mesure-904.sh` : base neuve, seuil de **13 Gio libres** dans le script, **index 1 seul** puis son
  éviction, `timeout 900`.

**La séquence** :
1. **T−1 min** : je l'annonce aux sessions. Je vérifie à luciepc qu'aucun index ne tourne et que personne
   ne parle au 7879 (`ss -tnp` sur 7879, `poste --qui`). Sinon on ne commence pas.
2. **T0** : G arrête le 7879 (`systemctl --user stop rag3weaver-embeddings@bge-m3`). Je relève card2 :
   `mem_info_vram_used`, environ 20 Gio libres attendus, et `gpu_busy_percent` à 0.
3. **T0 + 1 min** : `poste lourd bash mesure-904.sh` : l'hôte isolé 7971, le fournisseur de groupe à
   part 7891, l'index 1 du même dépôt, puis le bail rendu et l'éviction. La mémoire est relevée toutes les
   2 s.
4. **Fin de la mesure**, au plus quinze minutes : l'hôte isolé arrêté, aucun processus restant, card2
   relevée.
5. **G relance le 7879** (`systemctl --user start …`) et le vérifie : `/sante` (bge-m3, 1024, dual, creux,
   relecteur, OCR) et un embed d'essai. Je relève card2, revenue à environ 12,2 Gio libres. Puis
   l'annonce « relancé » aux sessions.

**Ce que la mesure départage** : un index 1 bien plus court que 904 s, sur une carte calme, désignerait
la carte pleine ; un index 1 voisin de 904 s désignerait le modèle. L'écart est dit tel quel, sans cause
tirée au-delà.
