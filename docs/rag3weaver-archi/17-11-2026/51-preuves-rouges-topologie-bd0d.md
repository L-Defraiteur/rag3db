# Trois rouges topologie acquis avant correction

GO **C7-ENTRY-TOPOLOGY-RED-BD0D-17nov60**, publié3215b9ec5d et lu avant entrée.
Source exacte bd0d71dde978f44f57cbf5615c2b3440746d86c7, WT propre, module
8b6de03703a1b2a2de8f30c90a93a01d247d0b3be436e57215b1603981cb0697 inchangé
de cb290 ; module/test/stub/interpréteur réempreintés conformes avant lancement.

UNE invocation de la commande épinglée topology-red-pins.json, unittest-v SANS
failfast sur les trois noms exacts, TERM58/KILL2, Python3.14-B, cwd/PYTHONPATH
épinglés sans module homonyme dans le répertoire de témoins. Résultat exact :
**3 FAIL, 0 ERROR, 0 skip, rc1** ; les trois motifs sont **Refusal not raised**.
Les contre-exemples de collision enfant/owner, enfant/root, root/runner même PID
avec start contradictoire sont donc réellement vus échouer avant correction.
Les48autres fonctions (dont le positif root=runner exact) restent NON JOUÉES.
Aucun retry. Les trois traces et la sortie complète sont préservées.

Temps monotonic local : START888049.618568704, END888049.679376938,
enveloppe **0.06080823403317481s** ; unittest affiche0.002s, mesure distincte.
Aucun reset ni comparaison d'horloges interhôtes. Les durées ne sont pas des
mesures du protocole physique : seule l'API de modèles a tourné.

Racine exclusive hors WT, LOCAL et PC :
`~/.cache/rag3weaver-build/cx-archi-entry-topology-red-bd0d-17nov-r1`.
Neuf fichiers (huit entrées du manifeste et manifeste lui-même) copiés puis
réhashés sur les deux hôtes : **0 écart**, acquis AVANT modification du module.
SHA manifeste :
`3572a359673f698f96ce04a424721f6b4877c726cb88b450d2bbff8505ec22a2`.
Source, pins, argv/environnement déclaré, stdout/stderr, rc et temps complets
font partie des bruts. La preuve baseline nominale f74 reste inchangée à côté.

Cette preuve ne vaut ni51rouges/verts ni admission OS, CP/Common réels, SDK,
stockage, capture d'entrée, quatre appelants, Cargo/native ou service. Aucun
accès de ce type n'a été fait par les témoins ; l'archivage distant ne lance pas
les sources. Seule la PRÉPARATION du correctif pur de topologie est maintenant
autorisée par ce GO. Publication/pins/RO puis GO51 distinct avant tout vert.
