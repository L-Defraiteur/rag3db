# La cause d'arrêt d'un service posé à la main n'est pas dite par l'hôte

- **État** : corrigé (18 novembre 2026, embarquement, branche `posed-stop-cause`, 88b76c27cc) ; ouvert (15 novembre 2026, embarquement)
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

## Correction (18 novembre 2026)

Conception `docs/embarquement/18-11-2026/03-conception-cause-d-arret-d-un-service-pose.md`, acceptée par c7.
- **`models.<capacité>.log`** : le chemin absolu du journal d'un service posé à la main. Refusé au chargement s'il est
  relatif, ou avec une activation (qui a son journal).
- **`ModelSource::stop_cause`** lit la dernière ligne « ✗ arrêt : <cause> » du journal de l'activation locale, sinon de
  `log` ; `cause_d_arret` reste comme alias.
- **La raison d'un service pas joint** dit alors « service d'embarquement arrêté : <cause> (adresse) — il est repris
  dès qu'il répond » ; sans cause, « pas joint » comme avant. Elle va au reçu, à l'état d'index et à la retente.
- Témoin `a_posed_service_stop_cause_is_said_from_its_log` (`src/backend.rs`) : vu rouge (« pas joint »), puis vert.
- **Pour les services de luciepc** : leurs unités écrivent dans
  `~/.cache/rag3weaver-unites-embarquement/journal-<instance>.log` (drop-in `journal.conf`). Ajouter `"log"` à un
  manifeste réel est la décision de sa propriétaire ; rien n'est changé ici.
