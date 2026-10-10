# Mémoire longue — ce que cette session sait

3 octobre 2026, et la journée du 4. Mis à jour sur place. Ce que le [journal des
chantiers](../../../../../docs/journal-des-chantiers.md) ne dit pas.

## 1. Les fichiers

| Où | Quoi |
|---|---|
| `templates/backends/memory/` | `Memory`, `Subject`, `ANCHORED_TO`, `REPLACES`, les machines à états — **tout en déclaration** |
| `scripts/test_banc_memoire.py` | le banc, cinq sessions, cinq mesures, deux régimes |
| `scripts/banc-memoire/paires-de-controle.json` | 72 paires, vérité **par construction**, versionnées |
| `tests/e2e_arret_brutal.rs` | le témoin produit de la reprise après mort brutale |
| `tests/e2e_simple_entity.rs` | la machine à états (27 août) et `EntitiesChanged` (3 octobre) |
| `src/events.rs` | la variante `EntitiesChanged { entity, uuids }` |
| `src/catalog.rs` | son émission, au succès, à côté de `marquer_les_ecritures` |
| `docs/3-octobre-2026-20h41/01-…` | la proposition, avec ses §7.7 à §7.10 de corrections |

## 2. Ce qu'il faut savoir du moteur pour travailler là

**Une relation a un couple `(depuis, vers)` fixe.** `register_relation_with`
refuse durement un ré-enregistrement avec un autre couple. Mais la
**traversée** est déjà polymorphe : `FetchRelatedNode` fait un `MATCH` sur des
nœuds non étiquetés et lit `label(m)` à l'exécution. C'est la déclaration qui
borne, pas le stockage — et le dialecte Postgres ignore même le couple.

**Au plus une relation par couple si l'on veut filtrer d'une entité sur une
autre.** `find_relation` itère une table de hachage et rend la **première** ;
avec deux relations entre les mêmes bouts, laquelle est prise n'est pas
déterministe.

**Une propriété d'arête ne se cherche ni ne se filtre.** Ni
`FetchRelatedNode` (arête anonyme, propriétés non rendues), ni `FilterParser`
(champs de nœuds seulement). Et `RelationBatchNode` **refuse** une relation à
propriétés : « snapshot links require a property-free relation; use an entry
entity for payload ».

**Un graphe-outil n'a pas de conditionnelle.** La valeur neutre en tient lieu
— une liste vide dans un port d'écriture n'écrit rien. Et il a **un seul**
port de résultat.

**Une seule cible de recherche par appel.** Le multi-entité se fait **par
graphe** (cf. `search_related_scoped.mmd`, quatre entités chaînées), pas par
paramètre.

**Un backend exige un service d'embarquement à l'ouverture même si aucune
entité ne déclare de vecteur** — `EmbeddingService::connect` est appelé sans
regarder les signaux. Signalé à la session recherche ; en attendant, le banc
exige le service.

**Le refus d'un index derrière sa table est une constante nommée du
moteur**, pas un message de circonstance :
`HNSWIndexUtils::INDEX_BEHIND_ITS_TABLE = "is behind its table"`, dans
`extension/vector/src/include/index/hnsw_index_utils.h`. Le commentaire
au-dessus dit la suite à faire — « l'appelant le reconnaît à ce fragment,
retire l'index (`DROP_VECTOR_INDEX`) et le rebâtit ». On s'accroche au
fragment, pas à la phrase, qui porte des noms de table.

**`CREATE_VECTOR_INDEX` n'est pas idempotent, malgré `skip_if_exists`.** Sur
un index *détaché* il refuse par ce fragment, `skip_if_exists` ou non. C'est
ce qui rendait `restore_dropped_vector_indexes` incapable de réparer le cas
qu'elle existe pour réparer — et pire, son échec remontait jusqu'à faire
refuser l'ouverture. Remède : `DROP` (qui porte `skip_if_not_exists`) puis
`CREATE`, toujours.

**`Catalog::initialize` rejoue ses DDL d'index à chaque ouverture** (étapes 2
à 4 : `generate_full_schema_with_dialect`, puis `schema.indexes`). Donc un
index **retiré** par un rejeu est déjà recréé à l'ouverture, par un code écrit
pour autre chose ; et un index **détaché** ferait échouer cette étape 4. Mais
ces DDL ne portent que le **schéma déclaré** : l'index d'un modèle
d'embarquement d'avant n'y est pas, et personne ne le recrée.

**`<base>.extensions` existe, à côté de `<base>.wal`.** Le moteur y note les
extensions à recharger au rejeu. **Le supprimer avant une réouverture retrouve
l'état d'index détaché** — c'est la façon propre de fabriquer ce témoin, bien
meilleure que déplacer la bibliothèque d'extension, qui est partagée entre
toutes les sessions du poste. Donné par la session cœur C++ le 4 octobre ; je ne
savais pas que ce fichier existait.

**Et une réserve sur ce témoin, à connaître avant de l'écrire** : si le journal
de la session morte porte encore l'enregistrement de chargement et que
l'extension a disparu, **la base ne s'ouvre pas du tout** — le rejeu relance
l'erreur de chargement. Ce n'est pas un index détaché, c'est une erreur
d'ouverture. Donc ce cas ne s'écrit **qu'avec la garde 2** du moteur, qui fera
continuer le rejeu sans l'extension.

**Un index ne se dit « en retard » que s'il y avait des écritures à rattraper**
dans sa table. Sans elles, rien à rejouer, et l'index reste juste. C'est ce qui
explique que le cas BM25 de `e2e_arret_brutal` ne joue aucune sonde : il n'y a
pas d'index du tout.

**Le nom à attendre quand une extension ne se charge pas au rejeu** :
« At recovery, extension VECTOR could not be loaded from <chemin>: <erreur> »,
**à la suite de** « is behind its table ».

**`CREATE_VECTOR_INDEX … skip_if_exists` refuse exprès sur un index détaché**, et
le cœur C++ refuse de le changer : « faire absorber le cas reviendrait à rebâtir
en silence un index entier derrière un mot qui promet de ne rien faire ». C'est
notre règle sur les avertissements, vue de l'autre côté de la frontière.

**Le moteur refuse une écriture dans une table indexée si l'extension
vectorielle n'est pas chargée**, hors rejeu. Donc on ne fabrique pas un index
détaché en « ouvrant sans l'extension puis en écrivant » : il faut la mort
brutale, avec un `CHECKPOINT` explicite entre le chargement de l'extension et
l'écriture.

**`_absent_since`** existe (schéma v8), est posée sur les lignes
transitionnées par une fin, effacée dès qu'une ligne reparaît par trois
chemins — et **relue par personne en production**. Préfixée `_`, elle n'est
pas adressable par `FilterParser`.

**La mise de côté existe** (`src/catalog/aside.rs`, 3 octobre) et fait plus
que promis : elle garde la ligne **et les vecteurs de ses chunks**, les rend
sans réembarquer si identité et empreinte n'ont pas bougé, purge par lots
bornés, et `undo_snapshot_finish` annule une fin en bloc tant que les copies
vivent.

## 3. La synchronisation par périmètre, ce que j'en ai relu

Trois fenêtres ont été trouvées et fermées, dans cet ordre, chacune par la
relecture de la précédente :

1. **Un plan périmé supprimait une ligne reparue.** Fermé par la relecture à
   l'application plus `check_open_session`.
2. **Le garde de session est par périmètre, l'identité d'une ligne ne l'est
   pas** — une session légitime sur B peut toucher une ligne que le plan de A
   tient. Fermé par la relecture de l'appartenance.
3. **Le garde gouverne le marquage, pas l'écriture** — une ligne recréée dans
   le même périmètre par une ingestion ordinaire porte une marque non
   concordante et serait retirée. Fermé en faisant prendre à toute écriture la
   marque de la session ouverte.

**La racine, utile à retenir** : la marque prouve « la session l'a vue », pas
« la source ne l'a plus ». Retirer sur `mark != session` déduit une absence
d'un négatif.

**Le prix assumé** : un écrivain qui réécrit périodiquement une ligne que la
source n'a plus **empêche définitivement son retrait**, en silence.

## 4. Les mesures, et ce qu'elles valent

| Mesure | Chiffre | Réserve |
|---|---|---|
| banc de la mémoire | 3 mesures jouées vertes, 2 non jouées | les non jouées **ne s'affichent pas en zéro** |
| paires de contrôle | 72, vérité par construction | personne ne les a étiquetées — c'est leur intérêt |
| seuil du modèle de décision sur mes paires | AUC 0,82 (0,97 sur les paires jugées) | les deux distributions se recouvrent |
| avec le pourquoi en plus du titre | 0,80 → 0,92, 5 redites sur 6 | le cosinus fait l'inverse : 0,99 sur les titres, 0,83 avec le pourquoi |
| ranger par le cosinus | 14/16, 19/32, 22/30 | sur le **texte complet** du sujet ; sur les noms seuls, 8/7/15 |
| décider qu'il faut **créer** | rien ne marche, sur aucun jeu | d'où « demander toujours » |
| arrêt brutal | `libvector` au journal ⟺ la base s'ouvre | 4 cas, une variable, aucune exception |
| garde 1 par le chemin du produit | 3 cas verts contre le moteur du 3 oct. 23 h 50 | **le cas vectoriel redéclarait le schéma** : son vert ne distingue pas « la garde a réparé » de « j'ai recréé l'index moi-même » |
| index détaché dans rag3weaver | **jamais observé** | ni détaché ni retiré : je ne sais pas lequel des deux la garde laisse |

**Le texte du juge n'est pas le texte du vecteur.** Le classement veut le
titre court, le verdict veut le titre **et** le pourquoi. Ce n'est pas un
réglage, c'est la forme du nœud — et la tentation est de passer le même objet
aux deux puisque c'est la même mémoire.

## 4 bis. La garde de cycle de vie : ce que « vide » voulait dire

`split_unchanged` rendait une `HashMap` nue pour l'état d'avant d'un lot, et
**une carte vide voulait dire deux choses incompatibles** : « la table a
répondu, aucune de ces lignes n'y est » — des naissances prouvées — et « je n'ai
pas pu regarder ». Comme `lifecycle_verdict` traite une absence d'état d'avant
comme une naissance, le second cas laissait passer **n'importe quelle transition
déclarée**, en silence.

`PreviousState { Read(carte), Unknown(raison) }`. Les cas, qui ne se traitent
pas en bloc :

| Sortie de `split_unchanged` | Ce qu'elle rend | Effet |
|---|---|---|
| relecture impossible | `Unknown` | **refus nommé** |
| entité absente de la configuration | `Unknown` | **refus nommé** (branche défensive, injoignable : `register_entity` insère dans `config.entities`) |
| zéro ligne relue, et première ingestion | `Read(vide)` | passe — naissances **prouvées** |
| chunks illisibles | `Read` | passe — c'est `stored` qui est bon là |

Et une exception : un état d'avant inconnu sur une ligne qui **n'écrit aucun
état** laisse passer l'état initial. Il n'y a pas de transition à vérifier, et
refuser là transformerait toute relecture qui tombe en panne d'ingestion.

**Le chiffre, et ce qu'il vaut.** Batterie complète : **274** occurrences du
chemin permis, **0** des deux chemins durcis. Mais les deux durcis sont des
**chemins d'erreur** — il faut qu'une requête échoue —, et une suite verte n'en
fabrique pas les conditions par construction. Donc le zéro **n'est pas une
mesure de rareté** : c'est une autorisation de durcir sans casser l'existant.

Les 274 par suite, ce que je n'avais pas regardé d'emblée :
`e2e_code_sync` **127**, `e2e_code` 68, `e2e_idempotent_registration` 19,
`e2e_synchronisation` 15, `e2e_agent_loop` 12, onze autres de 1 à 5. Donc la
réingestion — seconde synchronisation des mêmes fichiers, réédition — exerce
massivement la relecture de l'état d'avant, et **jamais son échec**.

## 5. Défauts connus, non corrigés

- **La portée `person` est écrite et jamais rappelée** par le crochet, faute
  d'identité d'appelant au protocole. Dette nommée, avec sa condition de
  sortie : le jour où elle existe, le crochet filtre au lieu d'exclure.
- **Un backend sans vecteur exige quand même le service d'embarquement.**
- **`_absent_since` n'a aucun lecteur** en production, et n'est pas filtrable.
- **`e2e_arret_brutal` est vert sur ses trois cas**, mais son cas vectoriel
  **redéclare le schéma** : son `register_entity` recrée l'index, donc le vert
  ne prouve pas que la garde a réparé. À refaire selon la recette à trois
  processus.
- **`restore_dropped_vector_indexes` ne pouvait pas réparer un index
  détaché** : son `CREATE … skip_if_exists` est l'appel que le moteur refuse.
  Corrigé par un `DROP` devant (lot en cours).
- **Le sujet n'est pas une entité dérivée**, alors que la mesure le demande.

## 6. Ce qui a été essayé sans succès

- **Un chaînage de nœuds existants pour le réacteur** : rien ne consomme le
  port `events` sauf `TraceSinkNode`, et sans conditionnelle la chaîne
  demanderait deux transformations `Rhai` et des ports qui ne s'emboîtent pas.
  Un nœud dédié est la voie.
- **Un lien symbolique pour peupler un sous-module dans un worktree** : git
  refuse (« le chemin du sous-module ne devait pas être un lien
  symbolique ») et `git status` sort en erreur. Un clone local depuis l'autre
  arbre, `--no-checkout` puis la révision exacte : 2 Mo au lieu de 2,6 Go.
- **Le régime sans embarquement du banc** : impossible aujourd'hui, le
  backend joint le service à l'ouverture quels que soient les signaux.
- **Le témoin « sans point de reprise » en une seule session** : muet, parce
  que `CREATE_VECTOR_INDEX` pose son propre point de reprise. Il faut deux
  sessions, la première fermée proprement.

## 7. Les habitudes qui ont payé

- **Vérifier à la source la plus récente avant d'annoncer qu'une chose
  manque.** J'ai annoncé deux fois une absence depuis un endroit où la chose
  ne pouvait pas être : la mise de côté (vue refusée sur une branche
  antérieure) et la fermeture de session (lue en queue de fichier au lieu de
  queue de fonction).
- **Un test porte sa condition de validité à côté de son verdict** : journal
  non vide, `REPLI=non`, âge de la bibliothèque liée. Éprouvé dès la passe
  suivante : trois rouges, et c'est l'imprimé qui les a nommés —
  `extension vector absente : …/rag3db-memoire/extension/vector/build/…`,
  faute de `RAG3DB_ROOT`. Depuis un worktree, **toute** référence au moteur
  passe par cette variable, pas seulement `RAG3DB_BUILD` : un rouge de harnais
  qui se nomme lui-même ne coûte qu'une relance.
- **Un rouge qui se nomme vaut mieux qu'un vert muet, et un vert qui imprime sa
  réserve vaut mieux que les deux.** « le montage ne voit plus d'index en
  retard : rebâti de lui-même ? » est ce qui a empêché d'annoncer une preuve
  qui n'en était pas une.
- **Nommer ce qu'un rouge signifierait**, dans le message d'échec, plutôt que
  « attendu 6, reçu 0 ».
- **Une condition de validité se vérifie aux deux bouts.** Relevée avant le
  travail, elle ne prouve rien de ce qui s'est passé pendant. Le 4 octobre à
  00 h 59, le moteur a été rebâti **pendant** une batterie complète : les
  binaires démarrés avant tenaient l'ancienne bibliothèque, ceux d'après la
  neuve, et la ligne d'âge imprimée en tête de passe était parfaitement exacte
  et parfaitement trompeuse. Batterie jetée. `run_e2e.sh` compare désormais une
  **somme du contenu** du moteur au début et à la fin et refuse de conclure si
  elle a changé.
- **Un contrôle d'identité se fait sur le contenu.** J'avais proposé la date et
  la taille ; l'arbre principal a mis une somme, et pour une raison mesurée :
  deux de ses rebâtis de la même nuit avaient **la même taille à l'octet**. Ma
  version aurait raté un vrai échange tout en criant au faux sur un `touch`.
- **Vingt essais ne valent que ce que vaut le taux cherché.** Mon premier
  résultat d'isolation était 0/20 contre 0/20, et j'ai failli le rendre : à 10 %
  — le taux mesuré ailleurs —, tomber sur zéro en vingt tirages arrive une fois
  sur huit. À cent, une fois sur trente-sept mille. Chaque essai coûtait 0,4 s :
  j'avais pris vingt parce qu'on m'avait dit vingt.
- **Deux côtés d'une comparaison doivent voir la même minute.** Sous une charge
  extérieure qui monte et descend, deux blocs consécutifs diraient la charge au
  lieu de dire le code : les côtés s'alternent.
- **Mesurer plutôt que raisonner, quand les deux sont possibles.** Aucun de
  mes raisonnements sur l'émission de `EntitiesChanged` n'a tenu ; les cinq
  jalons imprimés ont tranché en une exécution.

## 8. Ce que le 10 octobre a ajouté au savoir transférable

**Un chemin de manifeste se trouve par sa clé, pas par sa place.** Toute
réécriture de chemins dans un `backend.json` doit être un parcours **récursif**
par clé (`graph`, `schema`, `input_schema`) plus l'extension `.rhai` pour les
scripts, dont la clé est le *nom* du script. Énumérer les endroits rate
`input_schema` et le `graph` d'un crochet `before` — et la panne ressemble à un
gabarit cassé alors que c'est la réécriture qui l'est. Mesuré : deux gabarits sur
quatre tombaient.

**Un refus se teste depuis une fonction pure, ou il ne se teste pas.** Les
validations de manifeste écrites à l'intérieur de `load` ne s'éprouvent qu'en
fabriquant un manifeste sur le disque — donc on ne les éprouve pas, donc on les
découvre à la première panne. `valider_une_reaction` est séparée de `load` pour
cette seule raison, et ses cinq refus ont un témoin chacun.

**Le compte d'une liste blanche s'épingle.** Une liste de sécurité s'élargit par
accident : on ajoute un nœud « pour un cas » et la surface de tous les outils
grandit sans décision. Un `assert_eq!` sur sa longueur ne juge pas le contenu —
il exige seulement que l'ajout soit **décidé**.

**`poste lourd` prend le verrou en partagé** (`flock -s poste.lock`) : plusieurs
lourds tournent ensemble, c'est son intérêt. Il protège donc la **mémoire** et la
**priorité**, jamais l'**arbre** : aucun `git checkout` n'est sérialisé par lui.
Un worktree partagé entre sessions se fait arracher ses sources. Corollaire
rassurant, vérifié dans le script : le message « porte tenue depuis 600 s, le
lourd entre quand même » ne perturbe **pas** une mesure en cours — la porte ne
parle que des mesures en attente, et une mesure qui tourne tient le verrou en
exclusif.

**Deux faits de `dataflow/reactor.rs` à connaître avant de monter une réaction**
(relus, pas devinés) : `run_tool` instancie avec `json!({})` — les liaisons d'un
manifeste ne l'atteignent pas sans travail ; et il exécute sous
`NodeTypePolicy::All` — la liste blanche réactive est le **seul** garde-fou, et
il est au chargement.

**Le C++ du moteur se rebâtit par cmake dès `rag3db-native`**, sauf si
`RAG3DB_SHARED` est posée. Sur une machine neuve, ce n'est pas `cargo` qu'on
lance mais `run_e2e.sh` (qui lit `RAG3DB_BUILD`) — sinon on paie un build
complet du moteur sans s'en apercevoir.
