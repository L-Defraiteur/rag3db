# Un lecteur plein texte retient des écritures de blobs

- **État** : ouvert ; ciblé privé P14 qualifié sur a43c475f9 (preuve liée), qualification globale de l’union et livraison attendues.
- **Famille** : produit.
- **Gravité** : tentative de persistance en lecture seule ; la connexion native refuse, mais le lecteur ne doit pas préparer ces écritures.
- **Atteignable en service** : oui, lecteur Catalog BlobBacked ; tentative observée sur fixture synthétique privée, pas de persistance réussie prouvée.
- **Touche rag3weaver** : oui, initialiser_en_lecture, ouverture FTS et CypherBlobStore.
- **Pour** : arbre-principal ; revues mémoire/recherche/orchestration.

Parent : [raccord physique](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).

Preuve P10 e100737d0 : physical-07.log contient BufferedBlobStore dropped with
20 unflushed blobs, refus write dans read-only DB. Le catalogue lecteur monte un
buffer invisible à blob_buffer ; open_with_storage peut retomber sur CREATE et
le handle writable commit à sa fermeture. CypherBlobStore::prepare tente aussi
un ALTER sur toute première lecture. Aucun effet persistant réussi n'est prouvé.

Périmètre autorisé : store RO sans buffer ni DDL, écritures refusées avant query,
handles BlobBacked ouverts par snapshot public lucivy4.3.3 sans écrivain
intermédiaire, aucun fallback CREATE en RO. Fichiers absents/corrompus refusés.
Lecture eager et paquet mémoire transitoire : coût assumé pour ce chemin, sous
limite de qualification32GiB. Pas de modification lucivy/SDK/format persistant.
Modes fichiers/sparse hors preuve native BM25 actuelle restent à qualifier
séparément ; ne pas déduire leur absence d'effets de ce seul témoin.

Qualification attendue : CPU aucune query d'écriture même à Drop ; natif handles
is_read_only, commit refusé, index absent non créé et fichiers DB inchangés après
recherche/fermeture. Le garde de lecture publique sur versions internes reste
strict ; la maintenance interne suit les stockages physiques déjà entretenus.

Qualification ciblée P14 sura43c475f9 : VERTE (voir bilan parent lié).
Créations/écritures/reprise ou lecteur RO réellement éprouvés dans le profil
synthétique BM25. Qualification générale et livraison non faites ; ticket ouvert.
