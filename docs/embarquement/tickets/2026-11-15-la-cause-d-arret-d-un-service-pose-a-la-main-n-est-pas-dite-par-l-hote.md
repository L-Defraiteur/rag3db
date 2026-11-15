# La cause d'arrêt d'un service posé à la main n'est pas dite par l'hôte

- **État** : ouvert (15 novembre 2026, embarquement)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : manque. Pour un service posé à la main (le 7879 et le 7878 de luciepc), l'hôte dit « injoignable »,
  pas « service d'embarquement arrêté : <cause> ».
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/model_source.rs`, `src/backend.rs`)

## Ce que c'est

Depuis le lot du 503 nommé (15 novembre 2026), un service d'embarquement qui s'arrête écrit en dernière ligne de son
journal « ✗ arrêt : <cause> (code N) », et sort avec ce code. Pour un fournisseur lancé par une activation,
l'hôte connaît ce journal (`<activation.journal>/rag3weaver-embeddings.log`). Il y lit la cause avant de relancer, et
la dit : « service d'embarquement arrêté : <cause> ». Un service posé à la main par une unité systemd écrit ailleurs,
dans le fichier que nomme son drop-in `journal.conf` (`~/.cache/rag3weaver-unites-embarquement/journal-<instance>.log`).
La déclaration `models.embed` n'en dit rien. L'hôte ne peut donc dire que « service déclaré mais pas joint ».

## Ce qu'il faudrait

Un champ optionnel de la déclaration, `journal` : le chemin du journal du service posé. L'hôte y lirait la dernière
ligne, comme pour une activation. Ou bien le service garderait sa dernière cause d'arrêt dans un fichier connu, à côté
de son adresse. Témoin : un service posé, arrêté avec sa ligne, et la déclaration qui nomme son journal ;
l'état d'index dit « service d'embarquement arrêté : <cause> ».
