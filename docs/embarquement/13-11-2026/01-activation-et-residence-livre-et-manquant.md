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

## La mesure des 904 s, faite (15 novembre, 11 h 35 → 11 h 39)

**L'index 1 du groupe bge-m3 prend 33,3 s sur card2 calme, contre 904 s le 14 à 18 h 47.** Le même jeu :
les mêmes binaires (hôte 4f43b712d7e61903, service 12783f97e8dbdd45), les gabarits de 73262341b, le même
dépôt (80 fichiers, 2219 morceaux, bge-m3 dense et creux), une base neuve. **Ce n'est donc pas le
modèle** : les 904 s venaient de la carte pleine et chargée du 14 au soir, avec 2 Gio libres au pic, à
côté du 7879 et de deux autres lourds. Une seule mesure ne sépare pas la part de la mémoire de celle de
la concurrence pour le moteur.

| Pas | Mesuré |
|---|---|
| Avant (11 h 33) | card2 12 228 Mio libres, 0 % ; cinq connexions au repos sur 7879 (les tunnels du portable) ; annonce aux onze sessions |
| Le 7879 arrêté par G (11 h 35 min 41 s) | card2 à 23 043 Mio libres, 0 % |
| La mesure (11 h 36) | ouverture sans rien lancer ; activation pendant l'index 1, même identité qu'au 14 ; **index et vecteurs en 33,3 s** ; le fournisseur au plus 11,3 Gio ; card2 au pire 11,7 Gio libres |
| Bail, éviction | rendu à +29,4 s, évincé à +59,5 s ; l'hôte isolé arrêté, rien ne reste |
| Le 7879 relancé par G (11 h 38 min 49 s) | répond à 11 h 39 ; /sante complet, un embed et un embed_dual d'essai ; « relancé » aux sessions |

L'arrêt du 7879 a duré 3 min 19 s. Sondes et journaux à luciepc : `~/.cache/rag3weaver-build/groupe-vrai-jeu/`
(`mesure-904.out`, `sondes-904.log`).

## Le repli vers un service quand la carte est pleine : conception (15 novembre)

*À relire par c7 avant le code. La cause vient de la mesure des 904 s (33,3 s sur carte calme).*

**Ce qui s'est passé le 14.** Le fournisseur de groupe bge-m3 a été lancé sur card2, qui portait déjà le
7879. Celui-ci sert ce même modèle, et la carte avait 2 Gio libres au pic : l'index a été vingt-sept fois
plus lent. Le bon geste aurait été de **s'attacher au service qui sert déjà le modèle**, au lieu de
charger une seconde copie sur une carte pleine.

**Le bon endroit : `fallback` de la déclaration.** Il dit déjà « quoi faire quand le service ne répond
pas » (`refuse`, `local`). Une activation n'accepte aujourd'hui que `refuse`. Il gagne une troisième
forme :
```json
"fallback": { "service": "127.0.0.1:7879" }        ou        "fallback": { "activation": "<nom>" }
```
C'est un service déclaré, posé ou activé. **Sans cette déclaration, le repli n'existe pas** (garde 3).
Un seul champ couvre les deux replis : le fournisseur qui ne répond pas, et la carte pleine.

**« Carte pleine » se mesure, jamais ne se devine** (garde 1). Deux mesures, réglées au manifeste avec
leurs défauts, dans le bloc d'activation :
- **avant d'activer**, la VRAM libre lue sur la carte que l'activation nomme (`peripherique`, un nom
  drm comme `card2`, lu dans `/sys/class/drm/<peripherique>/device/mem_info_vram_*`) contre
  `memoire_requise_mio + marge_mio`. `marge_mio` vaut 2048 par défaut ; sans `memoire_requise_mio`,
  pas de mesure, et l'activation se fait comme aujourd'hui. Un `peripherique` qui ne nomme pas une
  carte lisible (`cpu`, un nom inconnu) n'est jamais mesuré, et c'est dit ;
- **pendant l'index**, le débit observé (textes par seconde des appels du fournisseur) sous
  `debit_min` pendant `fenetre_secs` (défaut : 60 s), une fois l'activation chauffée. Sans
  `debit_min`, pas de mesure de débit.

**Le repli se dit, et revient** (garde 2) :
- **le repli** : l'embarqueur passe au service de repli, identité vérifiée (modèle et dimension ; pas de
  recette, puisque c'est un service posé à la main ou une autre activation). Il le dit dans l'état
  d'index et dans le journal d'index : « replié vers 127.0.0.1:7879 à 18 h 47, cause : card2 à
  2 051 Mio libres, 11 500 requis + 2 048 de marge » (ou « débit 0,4 texte/s sous 5 pendant 60 s »).
  Le fournisseur activé, s'il l'était, rend son bail ; l'éviction fait le reste ;
- **le retour** : tant qu'il est replié, l'embarqueur remesure la carte (la VRAM libre, toutes les
  `fenetre_secs`). Quand elle a la place, il réactive le fournisseur déclaré et le dit : « revenu à la
  carte card2 à 19 h 02, 20 Gio libres ». Un index commencé replié ne reste pas lent toute la nuit ;
- **sans repli déclaré**, rien ne change : l'activation se fait, et l'état dit « carte pleine (card2 :
  2 051 Mio libres, 13 548 requis), pas de repli déclaré ». C'est tout.

**Ce qu'il ne fait jamais** : deviner une carte (seule la carte nommée par `peripherique` est lue) ;
lancer sur une autre carte de son propre chef ; toucher card0 ou un service posé à la main autrement
qu'en client.

**Les témoins, rouges d'abord**, sur la fixture. Une carte factice est un dossier `mem_info_vram_*`
donné par une variable de test, `RAG3WEAVER_DRM_RACINE`, défaut `/sys/class/drm` :
1. **la carte a la place** : le fournisseur se lance, comme aujourd'hui ;
2. **la carte est pleine, avec un repli déclaré** : rien n'est lancé ; l'embarqueur sert par le repli,
   et l'état et le journal disent le repli et sa cause ;
3. **elle se libère** : au calcul suivant après la fenêtre, le fournisseur est activé, et le retour est
   dit ;
4. **la carte est pleine, sans repli** : l'activation se fait, et l'état dit « pas de repli déclaré » ;
5. **un repli qui ne sert pas le modèle** : refus nommé à l'activation ;
6. **le débit sous le seuil** pendant la fenêtre (la fixture ralentie par `--lent-ms`) : repli, et sa
   cause dite.

**Le vrai jeu**, après le vert : le jeu du groupe du 14, avec `memoire_requise_mio: 11500` et
`fallback: { "service": "127.0.0.1:7879" }`, sur card2 et ses 12,2 Gio libres. Le repli doit se dire, et
l'index doit tourner au débit du 7879, jamais 904 s.

### Livré (15 novembre, branche `repli-carte-pleine`)

c7 a tranché trois points à la relecture :
- **la recette du repli** : elle porte le périphérique, elle ne peut donc pas se comparer à un repli.
  Sous `{service}`, modèle et dimension sont vérifiés, toujours. L'état dit « repli sans recette
  vérifiée », toujours ;
- **la forme `{activation: <nom>}`** est renvoyée à un lot suivant (ticket
  `2026-11-15-le-repli-par-une-activation-nommee`) ;
- **les clés sont en anglais** : `required_mib`, `margin_mib`, `min_throughput`, `window_secs`, et
  `fallback_hold_secs`, la tenue ajoutée à la relecture du vert.

Ce que le code fait :
- **la mesure** : l'activation mesure la carte qu'elle nomme, avant chaque prise neuve. Pendant un
  repli, elle la remesure après chaque fenêtre ;
- **les deux causes du repli** : la VRAM, ou le débit tenu par fenêtre sur la carte. La première
  fenêtre s'ouvre après le premier appel réussi : l'activation est alors chauffée ;
- **les cas sans mesure** : une activation distante n'est pas mesurée, car sa carte est à l'autre
  poste. Une carte illisible est dite une fois, et l'activation se fait ;
- **ce qui se dit** : la provenance de chaque prise (« carte card2 », « service 127.0.0.1:7879 »)
  compte les textes calculés. Le journal d'index dit, après chaque passe, ce que l'embarqueur a dit,
  puis la provenance des morceaux ;
- **l'état** : `embedding.fallback` dans l'état d'index, lu au suivi d'index quand le catalogue est
  occupé, et à l'embarqueur sinon.

Les témoins, tous rouges d'abord puis verts à luciepc, sous la lib 373b5477b :
- **b25c373af**, les sept rouges : les six ci-dessus, et la provenance au journal d'index ;
- **f26dc306d**, le correctif ;
- **5d4c8193f**, le témoin de la tenue, puis son correctif.

**Limite connue, au ticket.** Une carte qui a la place en VRAM mais calcule lentement alterne d'une
tenue à l'autre. Le doublement de la tenue est renvoyé au lot `{activation}`.

### Le vrai jeu sur card2 (15 novembre, 15 h 03, après le verdict du banc)

Le jeu du groupe du 14 :
- le gabarit de code de 73262341b, et le dépôt de 80 fichiers au pointeur f413dc63a ;
- l'activation nommée `bge-m3` sur card2, port 7891, avec `required_mib: 11500` ;
- `fallback: {"service": "127.0.0.1:7879"}` sur les quatre capacités ;
- l'hôte isolé de la branche (00828f25, lib 373b5477b), sur le port 7972.

Les scripts sont à luciepc, dans `~/.cache/rag3weaver-build/groupe-repli-jeu/` (`manifeste.py`,
`mesure.py`, `mesure.out`, `journal-index.txt`).

**Un écart à la demande, dit.** Quand le banc a rendu la carte, card2 avait 15 190 Mio libres, plus que les
13 548 du réglage par défaut. Le cas joué aurait alors été celui de la carte libre, avec une seconde copie de
bge-m3 à côté du 7879. Pour jouer la carte pleine, la marge a été relevée à `margin_mib: 4096`, soit 15 596
requis. C'est un réglage du manifeste, le code n'a pas changé.

**Ce que ce jeu prouve, et ce qu'il ne prouve pas** (c7, 15 h 10). Avec card2 à 15 190 Mio libres et la
marge montée à 4096 pour forcer le repli, le jeu prouve **le mécanisme** : le repli est dit, les 2 284
morceaux passent par le 7879, l'index dure 34 s, et rien ne se lance sur card2. Il ne prouve **pas** le cas
d'une carte réellement pleine. Ce vrai cas, le 7891 chargé à côté du 7879, se jouera une nuit où la carte
est prise : ticket `2026-11-15-mesurer-le-repli-sur-une-carte-reellement-pleine`.

**Résultat du mécanisme : VERT.**
- **Rien de lancé sur card2** : aucun processus du fournisseur, et le 7891 ne répond jamais ;
- **l'index en 34,1 s**, « indexation terminée ». C'est le 33,3 s de la carte calme, au lieu des 904 s du 14 ;
- **au journal d'index** : « replié vers 127.0.0.1:7879 à 15 h 03, cause : card2 à 15190 Mio libres, 11500
  requis + 4096 de marge ». Viennent ensuite cinq lignes de provenance, soit 2 284 morceaux par le 7879, et
  aucun par card2 ;
- **l'état d'index** porte `embedding.fallback` : « replié vers 127.0.0.1:7879 depuis 15 h 03, jusqu'à
  15 h 07 au plus tôt (cause : …) — repli sans recette vérifiée ».

Le créneau a été pris au « à toi » du banc, et le « fini » lui a été rendu à 15 h 04.

## Les réglages de l'embarquement, en un seul endroit (15 novembre 2026, au soir)

Tous se règlent au manifeste, ou par l'environnement du binaire du service. Aucun n'est une constante cachée.

**Dans un bloc d'activation** (`activations.<nom>`, ou le bloc écrit dans une déclaration) :

| clé | défaut | rôle |
|---|---|---|
| `required_mib` | absent : la carte n'est pas mesurée | la mémoire que le fournisseur prend sur sa carte |
| `margin_mib` | 2048 | la marge exigée en plus de `required_mib` |
| `min_throughput` | absent : le débit n'est pas mesuré | le débit, en textes par seconde, sous lequel l'embarqueur se replie |
| `window_secs` | 60 | la fenêtre du débit, et l'intervalle des remesures pendant un repli |
| `fallback_hold_secs` | 4 × `window_secs` | le temps minimal avant qu'un repli ne revienne à la carte |
| `fallback_hold_max_factor` | 16 | le plafond du doublement de la tenue après une rechute |
| `address` | absent | l'adresse du fournisseur quand le bloc est cité en repli : exigée alors, port 0 admis |

**Dans une déclaration** (`models.<capacité>`) :

| clé | défaut | rôle |
|---|---|---|
| `fallback` | `refuse` | `{"service": "<adresse>"}`, ou `{"activation": "<nom>"}` |
| `query_max_chars` (sur `models.embed`) | 6000 | le plafond d'une requête embarquée, au plus `context_tokens × 3` |

**Dans l'environnement du binaire du service** :

| variable | défaut | rôle |
|---|---|---|
| `RAG3WEAVER_EMBED_MAX_CHARS` | 12000 | le plafond d'un texte (413 nommé au-dessus) ; une valeur mal écrite refuse le démarrage |

**Pour les témoins et la mesure** : `RAG3WEAVER_DRM_RACINE` (défaut `/sys/class/drm`), la racine où se lisent les
cartes.

Les constantes qui restent sont des ratios ou des plafonds dits, pas des réglages : `chars_per_token: 3` (dit dans
l'identité du service), et le code de sortie 70 d'un arrêt sur un serveur de carte empoisonné.

## La tolérance de parité entre lots (mesurée le 19 novembre 2026, 9 h 16)

Ticket `embarquement/tickets/2026-11-16-la-preuve-de-parite-des-services-n-a-pas-de-tolerance-mesuree.md`.

**La mesure.** Elle est faite par `scripts/embed_parity.py`, un client seul (`/sante`, `/embed`). Le jeu fixe est
`tests/fixtures/embed_parity/texts.json` : 100 textes, 50 de prose et 50 de code, de 55 à 1 379 caractères. Chaque
texte est embarqué seul (c'est la référence), puis seul une seconde fois, puis en lots de 8, 32 et 128, dans trois
ordres (par longueur, longueur inverse, mélange à graine 18).

**Le contexte de la mesure.** Le binaire des services est b3764125fba4cc73 (master cae66528ae, avec `token-cap`), posé
par G le 19 novembre à 9 h 16. Les deux services calculent en Flex32, sur `gpu:0` (card2). Le 7879 tourne avec
`RAG3WEAVER_EMBED_MAX_TOKENS=4096`. Les journaux bruts sont à luciepc, dans
`~/.cache/rag3weaver-build/parity-measure-19nov/` (`parity-787x.json`, `sante-787x.json`, `binary.sha256`).

| Service | Même forme de lot (seul, deux fois) | Entre lots : écart absolu max | Entre lots : cosinus min | Au bit près entre lots |
|---|---|---|---|---|
| 7878, granite-278m (768) | 0, 100 % au bit près | **5,51e-5** | 0,99999993 | 0 à 2 % |
| 7879, bge-m3 (1 024) | 0, 100 % au bit près | **1,34e-3** | 0,99992861 | 0 à 1 % |

- **L'écart de bge-m3 est 25 fois celui de granite**, à précision égale (Flex32). C'est un modèle plus profond (24
  couches contre 12). Le cosinus reste au-dessus de 0,9999 ; son effet sur un classement n'est pas mesuré ici. Le
  pire écart vient des lots de 8 en ordre mélangé. Dès 32, toutes les compositions donnent le même écart (1,22e-3) :
  probablement le rembourrage à la plus longue séquence du lot, non vérifié.
- **granite** confirme la sonde du 16 novembre (3,77e-5 pour un texte seul puis au milieu de 7).
- **Le bit dépend du lot** : un texte seul rend les mêmes bits deux fois de suite, sur les deux services. L'écart entre
  deux binaires, lui, se prouve par le vecteur témoin de chaque pose (règle 1 ci-dessous).

## La règle de preuve d'une pose (19 novembre 2026)

1. **Même texte, même forme de lot, entre l'ancien et le nouveau binaire : au bit près.** C'est le vecteur témoin qui
   se fait déjà à chaque pose. Un écart, même de 1e-7, est un rouge : le calcul a changé.
2. **Entre lots, sur le nouveau binaire : sous la tolérance mesurée, avec une marge d'un facteur 2.** Pour le 7878 :
   écart ≤ **1,1e-4** et cosinus ≥ **0,99999987**. Pour le 7879 : écart ≤ **2,7e-3** et cosinus ≥ **0,99985**. Elle se
   prouve en rejouant `scripts/embed_parity.py --address 127.0.0.1:787x`, sans client sur le port pendant la
   mesure, et en lisant `between_batches`.
3. **Un nouveau modèle, ou une autre précision, se mesure avant d'avoir sa tolérance** : ces chiffres ne valent que
   pour granite-278m et bge-m3 en Flex32.
