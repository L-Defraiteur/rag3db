# Baseline hermétique élargie réellement observée

GO C7-TTY-PROVENANCE-RED-EEB-17nov60, source
 eeb635a07e9bea8495507a832aa58f3a6059ea32, test564393…476aa,
ancien8d998inchangé, driver206970…b951. Une invocation depuis worktree :
`timeout --signal=TERM --kill-after=2s 58s python3 -B docs/tty/17-11-2026/test_qualification_provenance.py`.

**14 tests : 10 réussis, 4 échecs ciblés, 0 erreur, 0 ignoré, rc1**.
Les huit témoins précédents passent dans CE run élargi, dont should_panic et
arrêt avant continuation du fauxbackend ; aucun coupon8vert isolé joué.
Deux nouveaux contrôles positifs passent. Les quatre refus manquants sont
observés : ELF changé au même chemin/noms, chemin restauré après exécution
d'un autre contenu, contenu changé après l'exécution, jeuVERT F8 surnuméraire.
Tous échouent sur « AssertionError not raised », pas sur ERROR de harnais.
Les preuves d'exécution et fichiers ELF de ces témoins sont virtuels : aucune
capture d'ELF natif ni provenance de la passe réelle n'est ainsi qualifiée.

start876895.827184342, finished876895.923017168 :0,095833s englobantes,
unittest0,053s. Sous la borne60secondes, aucun retry/failfast.
[Preuves](preuves-provenance-eeb/manifest.json). Archives RAW copiées puis
réhashées LOCAL et luciepc :
`~/.cache/rag3weaver-build/archives/cx-tty-provenance-red-eeb/evidence.tar`,
SHA256438f1fd61280bb877295ea712c70b251801e9c43c1a21ee6b4fc42470ea9737e.

Ticket anglais toujours OPEN. Aucun nouveau correctif de ces quatre défauts,
CPUgreen séparé, Cargo/no-run/list/native/service/acteur/SQL/slot/3600.
Qv2c003/Qv3cafd, target historique et anciens rouges préservés INACTIFS.
Suite : RO des preuves puis périmètre correctif/pins/RO/GOCPU distincts.
