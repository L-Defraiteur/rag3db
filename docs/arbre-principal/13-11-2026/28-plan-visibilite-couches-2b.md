# Préparation2b : visibilité avant recherche et lecture cohérente

Source relue : masterdb687edf63f4857e654ce6f158d1ae21056ae09b, après livraison2a d17 et fin des ressources. Branche/WT neufs cx-arbre-couches-2b-plan-r1/cx-wt-couches-2b-plan. [Ticket fils ouvert](../tickets/2026-11-13-appliquer-la-visibilite-des-couches-avant-les-requetes-des-vues.md), créé avant ce document ; [parent ancres ouvert](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md). Autorisation C7 : documents/contrats/testspec seulement. **Aucun nouveau Rust, test exécuté, Cargo, target, index, base, service ou modèle.** La garde Recherche est un lot indépendant prioritaire avant tout code2b.

## Ce qui est livré et ce qui manque

2a résout une forêt immuable en mémoire, avec parents exacts, tombstones, valeurs opaques et provenance gagnante par occurrence. `LayerGeneration` réutilise les publications existantes ; sa construction depuis une déclaration ne forge pas un certificat physique. `PinnedLayers<T>` n'ouvre aucun index, ne charge aucune révision durable et ne traduit pas les occurrences en offsets. Sa présence ne rend pas une vue accessible aux outils. Les usages de `PinnedLayers` relus sont dans son module et ses témoins.

Le raccord physique livré auparavant qualifie une publication d'un index neuf et son journal, avec ouverture contrôlée. Il ne fournit pas à lui seul un registre durable de couches, leurs parents, leurs valeurs/tombstones, ni la liaison complète d'une sélection de vue aux stockages de toutes ses révisions. `QualifiedPhysicalState` atteste son périmètre physique ; recopier ses chaînes origine/P dans une révision en mémoire n'atteste pas le contenu opaque de celle-ci.

Un premier test d'admission peut donc être isolé sans inventer ce stockage. Une requête réelle sur une vue en couches doit rester indisponible tant que son chargeur qualifié et ses ressources épinglées ne sont pas fournis et prouvés. Le ticket2b reste ouvert après un incrément CPU seul.

## Chemins réels relus

| Point | État actuel observé | Conséquence pour2b |
|---|---|---|
| [layers.rs](../../../extension/rag3weaver/src/dataflow/migrations/publication/layers.rs), `PinnedLayers::resolve` | valeurs empruntées et provenance ; `OccurrenceKey(entity,uuid)`, `FileKey` logique fourni | réutiliser le résolveur ; l'adaptateur doit qualifier la liaison de ces identités aux lignes physiques, sans confondre H et occurrence |
| [physical_index.rs](../../../extension/rag3weaver/src/dataflow/migrations/physical_index.rs), `open_owned`/`observe_published` | contrôle du registre/journal appliqué de l'index possédé | conserver cette admission physique ; aucune porte par simple nom ou déclaration de publication |
| [read_view.rs:612](../../../extension/rag3weaver/src/read_view.rs#L612), `ReadView`/`Forme` | forme immuable, cellule(org,project), connexion et services ; pas de forêt/révision de couche | porter un contexte de lecture commun à l'appel, distinct de la cellule et du nom d'entité |
| [backend.rs:4421](../../../extension/rag3weaver/src/backend.rs#L4421), `poser_l_instantane` | pour graphes `snapshot_safe`, remplace connexion/texte natif par ceux du pool ; pool absent/plein peut poursuivre sans instantané | une requête2b exigeant cette cohérence ne peut hériter silencieusement de cette dégradation ; refus explicite, ancien comportement hors2b préservé |
| [read_pool.rs:244](../../../extension/rag3weaver/src/read_pool.rs#L244), `ConnDInstantane::avant` | ouverture à la première requête ; échec d'ouverture ou dépassement du plafond conduit à `Phase::Fini`, lectures suivantes possibles hors transaction | spécifier un mode strict pour l'appel2b, ou une garantie équivalente démontrée ; expiration/refus ne doit jamais produire une réponse mélangeant deux états |
| [generic_search_nodes.rs:2980](../../../extension/rag3weaver/src/dataflow/generic_search_nodes.rs#L2980), `occurrence_mask_options` | résout des racines admises, puis versions dérivées et condition UUID ; les trois signaux réemploient cette préparation | point de composition commun du masque2b avec l'admission existante, avant budget de recherche ; ne pas créer trois moteurs parallèles |
| [generic_search_nodes.rs:3081](../../../extension/rag3weaver/src/dataflow/generic_search_nodes.rs#L3081), `allowed_ids_in` | filtre vers offsets parents/chunks ; erreurs strictes pour projection d'occurrences, certains chemins ordinaires avertissent puis continuent | un masque2b obligatoire échoue fermé pour toutes les cibles, indépendamment de la politique historique des filtres utilisateur |
| [search.rs:1058](../../../extension/rag3weaver/src/search.rs#L1058), `search_vector_via_backend` et [search_backend.rs](../../../extension/rag3weaver/src/search_backend.rs) | backend filtré reçoit condition/jointure avant sa limite ; `honore_le_filtre=false` émet actuellement un avertissement | la capacité2b exige un refus avant appel si le masque n'est pas garanti ; le booléen existant seul ne prouve pas l'admission par occurrence |
| [rag3db_search_backend.rs:78](../../../extension/rag3weaver/src/rag3db_search_backend.rs#L78) / [postgres_search_backend.rs:244](../../../extension/rag3weaver/src/postgres_search_backend.rs#L244) | chemins vectoriels filtrés : graphe projeté natif / clause filtrée pgvector | qualifier chaque chemin réel, modèle/index/colonne et contexte ; aucune preuve M3–M5 ou de rappel nouveau déduite du code |
| [search.rs:1778](../../../extension/rag3weaver/src/search.rs#L1778), `search_bm25_chunked`, et chemin creux | IDs autorisés transmis à lucivy ou `search_filtered` avant limite | conserver la distinction masque absent / masque vide et qualifier ce préfiltrage avec les vrais handles |
| [generic_search_nodes.rs:3192](../../../extension/rag3weaver/src/dataflow/generic_search_nodes.rs#L3192), `finish_signal` | projection vers occurrences admises après candidats ; exige les racines pour une projection déclarée | l'admission avant top-k ET la projection finale doivent utiliser le même ensemble visible ; ne pas projeter vers tous les parents d'un contenu |
| [dataflow/catalog_read.rs:272](../../../extension/rag3weaver/src/dataflow/catalog_read.rs#L272), `read_for_tool` | prend la vue de la cellule, sinon repli catalogue | aucune perte d'une sélection2b explicite dans ce repli ; cellule, scope de requête et vue en couches restent trois concepts distincts |
| [usage_nodes.rs](../../../extension/rag3weaver/src/dataflow/usage_nodes.rs), [search_nodes.rs](../../../extension/rag3weaver/src/dataflow/search_nodes.rs) | usages/hops et `FetchRelatedNode` passent par lecteurs/plans/connexion ; filtres de portée existants | ajouter l'admission des deux extrémités dans les constructeurs déclarés de requêtes/plans, avant limite et enrichissement ; aucune réécriture de Cypher par substitution textuelle |

Cette lecture ne déclare pas les chemins historiques défectueux pour une capacité2b qui n'est pas encore branchée. Elle identifie les endroits où leurs garanties actuelles ne suffiraient pas au nouveau contrat.

## Frontière commune proposée

Réutiliser `ReadViewCell`, `Forme`, les services d'appel et la préparation commune des signaux. Proposer un contexte de sélection épinglée, construit UNE fois pour l'appel depuis une origine et des publications exactes admises. Son nom définitif est à choisir avec les propriétaires ; sa nécessité tient aux invariants suivants, pas à un nouvel habillage des noms :

- origine/révision demandée et parents exacts, résolution2a et provenance gagnante par occurrence ; aucune consultation ultérieure d'une tête courante ;
- ensemble des occurrences visibles et leur identité logique complète, conversion attestée vers versions/chunks/offsets de chaque stockage/modèle ;
- ressources de lecture et d'index correspondant à ces mêmes générations, conservées jusqu'à la fin de l'appel ;
- masque obligatoire en intersection avec la cellule et le filtre effectif existant, sans changer la précédence domaine/utilisateur ;
- sélection inconnue, ambiguë, ressource absente ou capacité insuffisante : erreur nommée avant candidats, jamais masque omis ni changement de vue.

Scope de2a désigne une occurrence `Scope` par UUID ; `scope::Scope` désigne ici la cellule(org,project). `FileKey` doit inclure le domaine/cellule/dépôt requis par le montage : deux `a.rs` de cellules différentes ne deviennent pas un même fichier logique. Ni H ni un chemin seul ne remplace cette identité.

Le contexte sert également les outils de relations : source ET cible visibles, avec leur provenance gagnante. Chaque traversée et chaque enrichissement appartient à ce contexte. Les outils composent les mêmes services/contrats, sans une nouvelle logique métier par client. `view` reste un futur paramètre typé commun, et `provenance` un booléen séparé ; sans ce dernier, le rendu ordinaire doit rester stable.

## Prérequis physiques et de cohérence à ne pas sauter

1. **Chargeur de révisions.** Définir puis qualifier où résident parents exacts, valeurs, tombstones et liaisons aux publications physiques ; ouverture stricte, données manquantes refusées, reprise durable. Aucun format/table de stockage supplémentaire n'est choisi par cette préparation. Une carte en mémoire de l'appelant n'est pas une preuve de ce chargeur.
2. **Liaison occurrence–stockage.** Fournir depuis la publication admise les lignes d'occurrences et les références canoniques aux contenus/chunks, avec cellule et modèle. Vérifier qu'un offset n'est pas réutilisé à travers deux générations. Les permissions physiques existantes ne sont pas élargies.
3. **Instantané composé.** L6 fige une transaction DB, mais `ReadView::sur_connexion` superpose des services et remplace connexion/texte natif ; cela ne démontre pas une immobilité de tous les handles lucivy/creux hérités. Il faut épingler leurs snapshots/générations ou refuser un montage qui ne les garantit pas. Le snapshot readonly physique existant est une primitive à examiner, pas une qualification automatique de plusieurs couches.
4. **Cycle de vie strict.** Pool indisponible, backend sans connexion de lecture, expiration ou changement de génération pendant l'appel : réponse d'erreur cohérente, jamais poursuite avec un demi-instantané. Ne pas étendre globalement la politique L6 hors2b sans décision distincte.
5. **Accès complet.** Une fois ce contexte branché, vérifier les graphes d'outils effectivement accessibles, leurs paramètres et rendus ; une fonction appelée seulement par un test ne suffit pas à livrer une vue utilisateur. Les lectures directes/plans non capables d'admission doivent refuser la sélection2b plutôt que contourner le masque.

## Premier incrément proposé et ordre de livraison

**2b-A : admission et propagation communes, bornées CPU avant index réel.** Après autorisation produit distincte, dériver de `PinnedLayers` et d'une liaison de stockage explicitement fournie l'ensemble d'occurrences admises ; le composer au point commun des signaux et des lecteurs de relations. Un backend témoin enregistre le masque reçu AVANT sa sélection k ; un lecteur témoin enregistre les deux extrémités et la génération de l'appel. La liaison de stockage du témoin est une fixture nommée, jamais présentée comme le chargeur physique manquant. Inclure les refus d'absence de contexte/snapshot/capacité et le rendu optionnel de provenance. Cet incrément ne ferme pas2b et n'ouvre pas de vue publique durable par défaut.

**2b-B : chargeur et index natifs sur fixture physique privée.** Dépend des prérequis1–4, à arbitrer avant son diff ; utiliser les racines/publications réellement qualifiées et leurs handles. Jouer les mêmes contre-exemples sur les signaux déclarés capables ; un signal sans capacité nécessaire refuse la requête, même si `strict_signals=false`. Aucun filtrage après un top-k non masqué, aucun sur-fetch arbitraire ne sert de preuve. Les autres backends restent non qualifiés tant que leurs propres chemins ne sont pas jugés.

**2b-C : graphes accessibles.** À produit et stockage qualifiés, faire passer lecture/recherche/usages/relations par le même contexte, puis prouver le parcours d'outil réel. Ne pas annoncer C comme livré par A. Le choix automatique de la vue courante selon contexte doit alors être résolu par le fournisseur de contexte ; une sélection explicite inconnue refuse avant toute recherche. Aucun défaut vers une ancre ou le disque.

Arbitrage recommandé à C7 : autoriser d'abord la spécification et les témoins CPU de A, en gardant B/C bloqués sur leurs prérequis explicites ; choisir le porteur et le contrat de stockage avant tout code de chargeur. Aucun code A n'est autorisé par ce plan. Les suites de validation concrètes, leur fenêtre et leurs ressources attendront les diff/SHA exacts et la fin de l'intégration de la garde Recherche.

## Tests contractuels à écrire avant l'implémentation

Les lignes ci-dessous sont des tests SPECIFIÉS, **non écrits et non joués** dans ce lot documentaire. Le nombre de fonctions/juges/enfants sera arrêté dans la recette du futur SHA, pas déduit du nombre de lignes de cette table.

| Contre-exemple | Attendu et faux correctif détecté |
|---|---|
| k=2, au moins20 candidats invisibles mieux scorés et deux visibles moins scorés, pour dense/creux/FTS | les deux visibles arrivent ; backend reçoit le masque avant k ; attrape filtrage tardif et sur-fetch fixe |
| masque absent / masque vide / erreur de conversion distincts | absent sous sélection2b = erreur ; vide = résultat vide sans recherche large ; erreur propagée, jamais `None` signifiant tout |
| H partagé entre Aocc(a.rs,pos10) et Bocc(b.rs,pos90), A masquée seulement | contenu/chunk peut rester candidat grâce à B, résultat public B seul et sa position/provenance ; attrape dédup par H ou réexpansion vers A |
| même révision : tombstone fichier et nouvelle occurrence locale ; autre cellule avec même chemin | ancienne occurrence masquée, nouvelle et autre cellule conservées ; attrape clé fichier non domainée et précédence locale erronée |
| utilisateur demande précisément une occurrence tombstonée, domaine plus large | intersection vide ; le filtre utilisateur ne ressuscite rien et sa précédence historique n'est pas réécrite |
| arêtes A→B et C→A, A invisible ; doublons d'une arête entièrement visible | les deux premières absentes dans les deux sens, multiplicité des arêtes visibles conservée ; attrape filtre d'un seul bout ou dédup fautive |
| commit/publication avancée entre admission, candidats, enrichissement et provenance | tout reste sur P épinglé ou erreur cohérente ; aucune combinaison masqueP0/résultatP1 ; parent EXACT inchangé |
| pool plein, ouverture snapshot refusée, snapshot expiré, handle d'une autre génération | refus sans résultat partiel ni repli catalogue non épinglé ; attrape dégradation L6 silencieuse dans2b |
| backend déclare ne pas savoir filtrer, erreur de filtre, mode scan/projection sans admission | refus AVANT candidats, même en mode signaux souple ; aucun appel non filtré ni warning transformé en réussite |
| vue inconnue/ambiguë et courant automatique impossible faute de contexte ; branche sans worktree | erreur nommée, zéro lecture de repli d'une autre vue ; branche explicite lisible seulement via ressources blob/composite réellement admises, pas disque présumé |
| provenance désactivée puis activée, occurrences héritées et locales | rendu ordinaire stable ; provenance optionnelle gagnante par occurrence, jamais un P unique collé à tous les résultats |
| offset/H identique dans deux générations ou modèles distincts | liaison exacte ou erreur avant recherche ; aucun alias d'index par identifiant nu |

Le classement actuel travaille avec budgets de candidats/chunks, projection et fusion/pagination. Cette préparation n'invente pas une garantie de top-k exact des occurrences pour un ANN approximatif, ni une garantie de rappel/temps constant. La place exacte de k par signal et de la pagination après projection doit être relue avec Recherche ; les tests de saturation ci-dessus prouvent le contrat de visibilité en amont, pas M3–M5 ni une métrique de rappel globale.

## Vues courantes et histoire : contrats futurs maintenus

La vue courante se choisit automatiquement par contexte ; la provenance reste par occurrence. Pour une ancienne version explicitement demandée : si déjà indexée, pas d'ingestion ; sinon matérialiser l'état ciblé sans parcourir/indexer chaque commit intermédiaire. Ces comportements exigent le fournisseur physique qualifié : aucune implémentation n'est annoncée ici.

Checkpoints3a/3b : étudier un état logique COMPLET (visibilité, occurrences, relations, provenance et références aux contenus/index partagés) puis deltas, sans clone d'index. Un checkpoint delta reste dépendant de ses parents ; ne pas collecter leurs ressources tant qu'il en dépend. Chargement RAM à la demande et rétention historique sont des décisions distinctes, sans coût constant promis.

## Fin de cette préparation

Livrable : ce plan, le ticket ouvert et les ancrages de code figés dans le relevé joint. Aucun changement produit/README, aucun nouveau test exécuté ou index créé. Soumission à RO/C7 pour choisir les prérequis et l'incrément, sans GO runtime ni master produit. Les preuves/archives2a conservées ne qualifient pas2b.
