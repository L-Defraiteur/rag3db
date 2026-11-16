# Les douze sources de l'oracle, résultats compacts disponibles

Oracle indépendant905bf17f36, SHA38f7cf22..., attentes gelées avant sorties.
Snapshot de résultats02 : douze lignes échantillonnées de l'ELFbff1cafb...5565e2,
SHA2563e3ee90db2111430e8bb9f6e27db6bccc92df673346574df7804a0375fc311ff.
Les six langues sont représentées ; la campagne entière n'est pas encore END.

109faits avant sorties :80observed,2missing,1not_exposed,26unjudged. Détail
publié dans oracle-review-02-judgments.json, SHA256
f80db648a2aea7b1730545b9c61ca5e9f39c65900c47f993b793d6d93434472e.
Ces comptes ne forment pas un rappel : observed corrobore les dimensions retenues,
pas tous les qualificatifs de source_kind. Les dimensions perdues par le compact
sont maintenant unjudged, distinctes d'une absence réelle d'API. Le premier
rapport03 conserve son étiquetage historique, plus large, sans absence d'IR inférée.

- C, Python et JavaScript : principales définitions, spans et propriétaires de
  l'échantillon corroborés comme au préfixe01 ; signatures/modifiers et références
  individuelles non retenus restent non jugés.
- C++ : les méthodes ResourceTracker et trois SliceCast restent observées ; le
  forward sans body n'est pas une définition omise par accident selon le contrat
  relu. Namespace internal avec Result,T est un attribut wrong supplémentaire.
- Rust : Empty générique, empty et read_exact_arc ainsi que les trois méthodes
  des impls sont localisés. Le parent de méthode est bien Empty ; l'identité du
  trait Stream/FusedStream n'est pas retenue dans ces records, donc non jugée,
  pas un faux propriétaire. Les impls sont étiquetés Class : cette étiquette ne
  prouve pas la sémantique d'une déclaration struct ni un doublon de définition.
- TypeScript : Point, ses méthodes et les fonctions fléchées/alias MessageContext
  sont localisés et parentés. Le type fonction du paramètre map est conservé.
  x/y sont bien des paramètres constructor, mais manquent aux membres Point.

Trois anomalies confirmées, chacune avec ticket et minimal proposé NON JOUÉ,
aucun correctif pendant la baseline :

1. `docs/codeparsers/tickets/2026-11-16-un-template-forward-cpp-contamine-le-namespace.md` :
   paramètres génériques Result,T attachés au namespace plutôt qu'au forward.
2. `docs/codeparsers/tickets/2026-11-16-un-parametre-generique-rust-garde-sa-contrainte-dans-le-nom.md` :
   GenericParameter.name contient R: AsyncRead plutôt que l'identifiant R.
3. `docs/codeparsers/tickets/2026-11-16-les-parametres-proprietes-typescript-manquent-aux-membres.md` :
   les deux propriétés de constructeur existent seulement comme paramètres.

Les deux premières anomalies d'attributs sont découvertes après sorties, en
complément des109faits initiaux ; elles ne sont pas ajoutées rétroactivement
comme attentes prédites. Les2missing du décompte sont les propriétés x/y.
Les défauts C#/Go restent jugés par C7/Codeparsers, sans tickets dupliqués ici.

Le script d'aide compare ancrages ligne ET octet pour les scopes, ligne seule
pour les membres qui n'ont pas de bornes. La normalisation de parent Rust ne
valide que le nom du type, pas le trait manquant au compact. Les qualificatifs
static/async/readonly ne sont jamais déclarés prouvés par le seul nom.
Aucun reparse ni accès service/DB ; captures02 et annexe584générés conservées.
Le jugement final par langue attend les reçus END et les dénominateurs définitifs.
