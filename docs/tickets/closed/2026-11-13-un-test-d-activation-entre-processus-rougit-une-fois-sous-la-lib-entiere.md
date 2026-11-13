# Un test d'activation entre processus rougit une fois sous la lib entière

- **État** : fermé (13 novembre 2026, embarquement) : dix tours de la lib entière sans rouge de ce test après le port 0 annoncé (00e9fde57) ; ouvert le 13 novembre, relevé par arbre principal pendant le lot du raccord physique.
- **Pour** : embarquement (décision de c7, 13 novembre 2026).
- **Famille** : produit
- **Gravité** : passe rouge sans cause dans le lot qui la joue ; une fois sur sept tours.
- **Atteignable en service** : à départager.
- **Touche rag3weaver** : oui (`src/model_source/activation.rs`).

## Ce que c'est

`model_source::activation::tests::profil_etranger_entre_processus_est_refuse` a rougi une fois,
au tour 3 de trois tours de la lib entière, joués sur master `73c7f15e1` sur lucie-tablette :

```
thread 'model_source::activation::tests::profil_etranger_entre_processus_est_refuse' panicked at src/model_source/activation.rs:196:55:
test result: FAILED. 2307 passed; 1 failed; 67 ignored
```

- **Les conditions** : lib `207701bb2`, sans `MOTEUR_ANCIEN`, jeu large, 8 jobs, fils de test par
  défaut, sans `RAG3WEAVER_EMBED_SERVICE`, sous `poste lourd`.
- **Les six autres tours** : verts, avec ou sans les services.
- **Ce qui manque** : le message de la panique. Le journal ne gardait que les lignes filtrées.

La ligne 196 est `connect_embedder(&f.source).unwrap()` dans le processus parent, avant le
processus enfant. C'est donc l'activation de la fixture elle-même qui a échoué, pas le refus du
profil étranger.

## Lecture du code (13 novembre, embarquement) : une course de port, plausible, pas prouvée

Le message de la panique du rouge n'a pas été gardé, et le test n'a pas rougi dans les quatre lib
entières suivantes (`~/.cache/rag3weaver-build/ci-physique-passe/` et `ci-physique-rejeu/`). La
cause n'est donc pas établie. Ce qui se lit dans le code :

- L'activation gérée exige une **adresse locale fixe** et refuse le port 0
  (`src/serveur/activation.rs`, « activation gérée : adresse locale fixe requise »).
- Pour en avoir une, `Fixture::nouvelle` (`src/model_source/activation.rs`, module `tests`) lie
  `127.0.0.1:0`, lit le port, **relâche l'écouteur** (`drop(listener)`), puis passe ce port au
  fournisseur (`tests/fixtures/residence/fournisseur.py`, `--adresse`), qui ne s'y lie qu'à son
  démarrage, plus tard.
- Sous la lib entière, beaucoup de tests lient des ports en même temps : un autre peut prendre ce
  port entre le relâchement et la liaison du fournisseur. Le `connect_embedder` du parent (la ligne
  196:55 de la panique) tomberait alors sur un autre serveur ou sur rien. C'est le même motif que la
  course `port_libre` du démon (11 octobre 2026), réglée là par un port 0 annoncé.

Autre piste, non écartée : `budget_activation_secs = 5` et `timeout_secs = 1` pour lancer un python
et le faire répondre, sur un poste chargé.

Le message départage : « identité », refus de connexion ou serveur inattendu pour la course ;
délai ou budget pour la seconde. Garder la sortie entière de toute lib qui le rougirait.

Remèdes possibles pour la course, **à choisir (ils touchent la conception de l'activation)** :
1. le test garde son écouteur et le passe au fournisseur (descripteur hérité) : rien ne change au
   produit, mais la fixture se complique ;
2. l'activation accepte un port 0 si le fournisseur annonce l'adresse prise, comme le démon
   d'embarquement : une adresse fixe reste possible, la fixture n'en réserve plus ;
3. la fixture réessaie quand l'identité trouvée n'est pas la sienne : simple, mais cache la course
   au lieu de l'ôter.

## Remède retenu (c7, 13 novembre) : le port 0 annoncé par le fournisseur

`serveur::activation::acquerir` accepte une adresse locale de port 0. Le fournisseur se lie
lui-même et annonce l'adresse prise (`LIGNE_ADRESSE`, comme le démon d'embarquement) ; elle est
retenue à côté du verrou, au dossier de coordination (`<adresse>.adresse`, écrit puis renommé), et
une activation suivante la sonde sous le verrou avant de lancer : elle ne s'y attache que si le
fournisseur y répond, sinon elle relance. `adresse_retenue(coordination, adresse)` la lit. La
fixture du test déclare le port 0 et ne réserve plus de port. Une adresse fixe reste possible.

Témoin, rouge sur l'ancien code (qui refusait le port 0) :
`model_source::activation::tests::une_activation_sur_port_nul_annonce_son_adresse_et_la_retrouve`
(deux demandes, un seul démarrage du fournisseur, la seconde retrouve l'adresse prise).

Si le rouge de la lib entière revient avec le port 0, la seconde piste (le python lent sous charge,
5 s de budget) reste ouverte, et ce ticket aussi.

## La fermeture (13 novembre, embarquement)

Le remède (le port 0 annoncé par le fournisseur) est sur master : `2d7d9b165` (le témoin, rouge sur
l'ancien code : « adresse locale fixe requise ») et `00e9fde57` (le correctif).

Dix tours de la lib entière sur `00e9fde57`, à luciepc, lib
`rag3db-build-835d5298f80b832377a7caa80ee58d242e3ece23-…`, sans `RAG3WEAVER_EMBED_SERVICE`, sous
`POSTE_MEM_MAX=32G poste lourd`, sortie entière gardée par tour
(`~/.cache/rag3weaver-build/activation/tour-<n>.log` à luciepc) :
`profil_etranger_entre_processus_est_refuse` vert dix fois sur dix. Neuf tours à 2220/0 ; le tour 7 a
rougi un autre test, `dataflow::migrations::physical_index::tests::native_process_death_recovers_same_program_and_lost_ack_once`
(« after-lexical-durability », `left: None, right: Some(91)`, fixture gardée à luciepc en
`/tmp/.tmpGr3L1P`), sans rapport avec l'activation : dit à l'arbre principal.

Avant le correctif, les tests d'activation avaient aussi passé 20 puis 10 fois de suite, et la lib
entière deux fois.

**Ce que ça prouve et ce que ça ne prouve pas** : à un rouge sur sept, dix tours verts arriveraient
encore une fois sur cinq par hasard. La course de port est ôtée par construction (plus aucun port
réservé puis relâché) ; la seconde piste (un python lent sous charge, 5 s de budget) n'est pas
réfutée. Si ce test rougit encore, rouvrir ce ticket avec le message de la panique.

## Ce qu'il faut pour le fermer

Retrouver le message, en rejouant la lib entière avec la sortie complète gardée. Puis départager
une limite de temps sous charge d'une vraie erreur de l'activation.
