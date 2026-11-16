# Un worker déborde sa pile sur un fichier OpenSSL

- **État** : ouvert — panne isolée observée dans le premier corpus, aucune relance du cas
- **Famille** : produit
- **Gravité** : un fichier de 149934 octets fait avorter le worker
- **Atteignable en service** : à déterminer ; le banc appelle les workers de production mais ajoute un inventaire compact récursif
- **Touche rag3weaver** : potentiellement, dépendance codeparsers ; aucune ingestion dans le banc
- **Pour** : codeparsers, réduction et localisation après la campagne
- **Signalé par** : campagne FULL02b, source384165760b / parser1b35cfb / base bibliothèque bd13385

## Témoin observé et provenance

Séquence2258 de `language-corpus-02b/campaign/results.jsonl`, langage c/façade scope :
`github.com/openssl/openssl@899d0ca02559e105b9cb30964e3478dfbd8c4578`,
`ssl/s3_lib.c`, 149934 octets,
SHA256 `38036ad0cc36536c11f1580d79206239d378c20aa9806e670cbb3b8ee3d27d47`.
Licence Apache-2.0, preuve LICENSE.txt dans le gel de Banc. Corpus public non exécuté.

Le contrôleur enregistre worker_exit, code -6, durée contrôleur0.185673863s.
Stderr : `thread 'main' (2792790) has overflowed its stack`, puis
`fatal runtime error: stack overflow, aborting`.
Aucune récidive provoquée, aucun retry ; le contrôleur a remplacé le worker et
poursuivi les autres fichiers selon le contrat exploratoire de C7.
ELF SHA `bff1cafb72e8c6662d2d595457b0ca6da4718686ae64b91ae1bde7a4e95565e2`.
Enveloppe globale 32GiB, quota5 CPU ; RLIMIT_AS worker2GiB, timeout10s.
Le code -6 et stderr établissent ici un débordement de pile, pas une échéance
ni une preuve d'OOM du cgroup.

## Ce qui reste à établir

La pile peut déborder dans le parsing, la collecte récursive des faits du banc,
ou une autre étape ; aucun étage précis n'est démontré par ce journal.
Après gel des preuves : localiser l'étape sous borne neuve, réduire le fichier
sans exécuter le projet, observer le rouge minimal puis corriger à la bonne
frontière. Qualifier ce qui est atteignable via les API de production et préserver
les faits du fichier. Aucun stack overflow ne devient un parsing réussi.
Parent : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.
