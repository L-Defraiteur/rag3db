# Le démon d'un test unitaire « n'a pas répondu » sous la charge du poste

- **État** : fermé le 11 octobre 2026 (optimiseur, branche `demon-port-zero`)
- **Gravité** : test instable (un rouge sans défaut du produit)
- **Atteignable en service** : non (le délai est celui du test, pas du démon)
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs`, tests)

## Ce que c'est

`daemon::embeddings::tests::le_demon_porte_aussi_le_relecteur_et_l_ocr` a rendu « le démon
n'a pas répondu » le 10 octobre au soir. Le test lance un démon dans un fil, sur un port
pris puis relâché par `port_libre()`, et lui laisse 200 × 10 ms pour répondre
(`fn demon`, `src/daemon/embeddings.rs` ~830). Deux batteries `lourd` tournaient en même
temps.

## Recette

Pas de recette sûre. Seul, le module passe : 5 passes sur 5, 18 tests en 0,27 s. Deux causes
possibles, non départagées :
- **le délai** : 2 s, sous la charge de deux batteries ;
- **la course au port** : `port_libre()` relâche le port avant que le démon ne le prenne, et
  un autre processus peut le prendre entre-temps.

## Témoin

Le test lui-même, sous charge. Il n'a pas de témoin à part.

## Pour le fermer

- Lier le démon à `127.0.0.1:0` et rendre l'adresse qu'il a prise : plus de course au port.
- Porter l'attente à un délai qui tienne sous charge (10 s), en gardant la sortie dès la
  première réponse.

## Observé aussi (11 octobre 2026, chantier I)

`daemon::embeddings::tests::un_gros_envoi_est_decoupe_en_lots` a rendu la même
panique (« le démon n'a pas répondu », `fn demon`, ligne 843) pendant une lib
complète sur luciepc, sous une batterie voisine ; rejoué seul, trois fois vert.
Même cause : le délai de 2 s de `fn demon`, pas le démon.

Et `daemon::db::tests::un_blob_traverse_le_demon` (`src/daemon/db.rs:416`), même panique, une fois
sur quatre lib complètes de l'arbre principal (11 octobre, 4 h 35, retrait du filet du COPY) ;
vert à la passe suivante.

Et deux à la fois dans une lib complète de la session embarquements (11 octobre, vers 13 h,
lot 1 du chaînage, après un rebase, sous `poste lourd`) : `daemon::db::tests::plusieurs_clients_sur_la_meme_base`
(`src/daemon/db.rs:416`) et `daemon::embeddings::tests::le_factice_traverse_le_fil`
(`src/daemon/embeddings.rs:843`), même panique ; rejoués seuls (`daemon::`), 27 sur 27 verts,
puis verts aux trois lib complètes suivantes.

## Fermé (11 octobre 2026)

**La cause était la course au port, pas le délai.** `port_libre()` liait un
écouteur au port 0, lisait le port, le **relâchait**, puis le démon se liait à
ce numéro : entre les deux, un autre test (ou un autre processus) pouvait le
prendre. Les trois visages de la même course, vus au témoin :
- « le démon n'a pas répondu » (le démon n'a jamais pu se lier) ;
- le port « mort » d'un test tenu par le démon d'un autre test
  (« 127.0.0.1:43985 sert regle » dans `le_service_se_choisit_par_le_modele…`) ;
- un test qui parle au démon d'un autre test (« un démon ne doit pas blanchir
  ce qu'il transporte » dans `le_factice_traverse_le_fil`).

**Le correctif** : `daemon::servir_lie(service, TcpListener, fils, expose)`
sert un écouteur **déjà lié** (`tiny_http::Server::from_listener`), et
`EmbedDaemon::servir_lie` / `DbDaemon::servir_lie` le relaient. Les tests se
lient au port 0, gardent l'écouteur et le donnent au démon : le port n'est
jamais relâché. Les tests qui veulent une adresse où personne n'écoute
prennent `127.0.0.1:1` (rien ne s'y lie sans privilège) au lieu d'un port
pris puis relâché. `servir(adresse)` est inchangé pour les binaires de
service. L'attente du premier « répond » passe de 2 s à 10 s ; elle ne
couvre plus que le démarrage des fils, le port étant tenu depuis le `bind`.

**Le témoin** (`~/.cache/rag3weaver-build/demon/temoin.sh`) : dix tours de
huit processus du binaire de tests de la lib (`daemon::`, 27 tests,
16 fils chacun) en parallèle, sous `poste lourd`.

| | tours rouges sur 10 | processus rouges sur 80 |
|---|---|---|
| avant (origin/master 56a680169) | 3 | 4 |
| après | **0** | **0** |

**Reste hors de ce lot** : `tests/e2e_rag3daemon.rs` et
`tests/e2e_demon_embeddings.rs` lancent les binaires de service sur un
`port_libre()` ; les corriger demande que le binaire dise le port qu'il a pris
(lié au port 0), donc toucher aux binaires de service — à faire avec leur
propriétaire si ces e2e rougissent sous charge.
