# Qualification ciblée du raccord physique neuf

Le ciblé P14 est **VERT** sur la source exacte `a43c475f9ce1f7cbd6c1c2a0ee56719b521678f1`,
branche privée `cx-arbre-raccord-physique-r2`, base master798ab1aa7.
Client : 44.321345s dans une portée distincte599s autorisée C7.
Superviseur `validation-gllfe1kq` : VERT/code0/restes[], autorité terminée,
bus `/tmp/r3v-26lgxvot` absent et scan indépendant des processus/cwd du contexte
vide après fin. Aucun GO global/master n'est déduit de ce résultat.

## Ce qui a réellement tourné

- Compilation native+code, sans features par défaut :23,791s, puis inventaire réel.
- 10 scénarios CPU et10 fonctions natives : **20 fonctions,19 scénarios juges**,
  zéro ignoré. Le helper enfant est une fonction brute, pas un juge autonome.
- 9 juges natifs, dont6 morts/reprises du lot et2 morts de bootstrap réellement
  atteintes par8 enfants avec sortie91 exigée ; sept témoins PHYSICAL_WITNESS
  (réouverture normale et6 fenêtres du lot), deux BOOTSTRAP_WITNESS.
- Valeurs exactes :8occurrences,6versions de contenu,8références,2cellules ; UUID,
  rendus, champs source y compris content composite, chunks et leurs bornes dans
  CE rendu, multiplicité et absence de liens entre cellules. Pas de projection
  des offsets du rendu sur body brut ou sur les lignes du fichier revendiquée.
- Vraies requêtes BM25 projetées vers occurrences, cellules et noms/docstrings ;
  aucun modèle configuré, aucun vecteur/sparse, pas de modèle factice.
- Préfixes0/1/2 et cardinalités persistées relus AVANT reprise, sources Mermaid
  retirées puis même programme durable repris ; seconde cellule incluse.
- Publication/journal appliqué liés, corruption/refus et identité d'opération,
  bootstrap incomplet quarantiné avant ouvertures ordinaires. Instantanés et
  octets complets inchangés par les refus concernés.
- Relations natives sans propriétés :0/1/2arêtes mêmes extrémités, schéma stable,
  doublons préservés et3empreintes logiques différentes.
- Lecteur BlobBacked réellement RO : handles présents/nonvides dans Alpha ET
  Beta, is_read_only, commit refusé, index absent non créé ; racine entière
  inchangée après recherches/fermeture. Magasin RO sans buffer/DDL/write queries,
  témoin CPU d'absence d'écriture jusque Drop.

## Provenance et ressources

SDK natif29 fixé, stockage moteur43 ; Lucivy4.3.3 inchangé. CPU seulement,
-j8, BURN CPU, tampon de chaque base536870912octets choisi par politique commune
et recette explicite ; cette valeur n'est pas un minimum universel démontré.
MemoryMax32GiB/MemoryHighmax vérifiés dans le vrai cgroup, bus privé unique pour
la portée complète. Target unique conservé, somme de tailles recette
1672334004octets ; du -sb 1607881558octets (comptages distincts),
tous deux sous32GiB. Pas de rebuild SDK/C++ ni service/modèle/GPU/PG réel.

ELF libtest `rag3weaver-9d6f28c3a8a55881` trouvé dans les messages Cargo, tag source
réel attesté dans ses octets, original/copie SHA256 :
`93d384762e938b944fdb3945bb47a444eb7a3b17fc9871d4e05b8035efe18846`.
223gabarits inventoriés/rehasés ; octets embarqués des deux gabarits requis par
ces features vérifiés : tools/search_base.mmd et search_expansion.mmd.
Maps natives observées pour26processus, toutes sur le SDK épinglé, cgroups
vérifiés ; pas une simple variable d'environnement prise pour preuve de chargement.

PC : `~/.cache/rag3weaver-build/cx-raccord-physique-cible-v14/` et
`cx-raccord-physique-confinement-v14/validation-gllfe1kq/`.
Copie locale complète : `~/.cache/rag3weaver-build/cx-preuves-raccord-physique-cible-v14-local/`.
**1653 fichiers** client/superviseur identiques SHA256 aux deux postes, dont ELF,
logs, inventaire, maps et témoins. Manifeste local inventory-sha256.json :
`e778b4c8d539ce70752027483021111e1cdb3fece2d9fe756808d06b95923c78`.
Le contrôle indépendant est aussi copié dans quiescence.json.

## Limites et suite

Les rouges P1–P13, le complément P3 et l'attestation RO restent conservés
séparément avec leurs reçus, délais, causes et copies ; P14 ne les repeint pas.
Chaque correctif a été publié et relu, puis sa nouvelle portée explicitement
coordonnée. P10–P13 ont conservé leurs fixtures en panique pour diagnostic RO ;
P14 n'a aucune fixture en échec. Aucun ancien lot/NAV ni retrait réel rejoué.

Qualification ciblée privée seulement : suite générale/minima/F8/CLI/daemon/PG
non rejouée sur cette source, union avec master ultérieur (notamment ED b876881)
non qualifiée. Pas de livraison master, activation DB réelle, migration legacy,
transfert dense/sparse, couches de vues, rollback général ou GC. Les tickets
restent ouverts jusqu'à qualification générale et décision de livraison C7.
Le chemin lecteur BlobBacked empaquette l'inventaire visible en snapshot public,
eager avec copie RAM transitoire et potentiels fichiers obsolètes ; pas un export
minimal des seuls segments vivants. Les modes fichiers/sparse restent hors de la
preuve BM25 ciblée ci-dessus.
