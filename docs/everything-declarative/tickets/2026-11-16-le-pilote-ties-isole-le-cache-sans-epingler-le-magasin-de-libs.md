# Le pilote ties isole le cache sans épingler le magasin de libs

- **État** : ouvert ; rouge d'entrée conservé, aucun juge produit joué.
- **Famille** : outillage
- **Gravité** : bloque la qualification de l'ordre stable avant Cargo.
- **Atteignable en service** : non, contrôleur privé de validation seulement.
- **Touche rag3weaver** : oui, résolution de lib dans run_e2e.sh appelée par le pilote.
- **Pour** : everything declarative, coordination C7.
- **Signalé par** : ED, première entrée autorisée sous lib59ac le16novembre.

## Rouge avant correction

APPbd76f557928b5078f98a235f1cb0b5f78e7360d8,
PILOTE6d1e3468a21eb283eedac87cc9460f96ecf3f360. Le driver isole
XDG_CACHE_HOME dans ses preuves mais omet RAG3DB_LIBS_DIR. Le résolveur cherche
donc la bonne lib59ac dans le mauvais magasin privé `.../cache/rag3db-libs`.
Premiercibléexact-0 code2, journal d'une ligne, avant Cargo/ELF/juge.
Premierrouge1173728.265258157, scopevalidation-982fhhua ROUGE/code1,
restes[]/nettoyé, autoritéterminée. Target maintenant marqué sourceEDr2,
aucune .fingerprint ; aucune reprise étrangère ou purge à faire.

Preuves PC `~/.cache/rag3weaver-build/cx/ed-ties-59ac-r2-v1` et
`cx-ed-ties-confinement-20261116-v1/validation-982fhhua`, contrôleur
`cx-ed-ties-controller-20261116-v1/outer.log`. Copie locale
`cx-preuves-ed-ties-v1-local/{client,controller,supervisor}`. Le reçu privé
avec jeton n'est pas publié. Aucun ancien vert1834 transféré.

## Correction et fin attendues

Fixer explicitement RAG3DB_LIBS_DIR sur le magasin réel haché lors de la
préparation, tout en conservant XDG_CACHE_HOME privé. Vérifier le transport
de cette variable dans le profil réel du driver, publier la source du pilote
et un témoin hermétique avant nouveau runtime. Nouveau P/Q/S ; conserver le
target même source avec sa garde commune, sans en inventer une provenance
affa ou en effacer les marqueurs. Aucun retry identique ni reset7200 implicite :
nouvelle entrée seulement après arbitrage du reste/budget par C7.

Fin : preuve d'entrée correcte puis qualification réelle complète ou nouveau
rouge attribué ; ce ticket ne ferme pas le lot produit ni n'autorise master.

## Correctif publié avant test

Profil magasin explicite et témoin du vrai résolveur dans la branche
`cx-ed-stable-tie-qualifier-r2`, [rapport06](../16-11-2026/06-correctif-pilote-ties-magasin.md).
Code et témoin NON JOUÉS lors de cette publication, rouge v1 inchangé.
Admission corrective bornée au RESTE ENDv1 après poste, pas à7200 neufs.

## Qualification corrective acquise

APPbd76/pilote13c8 : passe canonique v3 entièrement VERTE, scopeVERT0/restes[]/busabsent, sous reste de START/ENDinitial sans reset. [Preuves détaillées](../16-11-2026/09-preuves-canon-ties-v3.md),29fichiersPC/localréhash0écart. Anciensrougesconservés. Pas de GOmaster ni retrait déduit ; ticket laissé ouvert pour revue et intégration coordonnées.
