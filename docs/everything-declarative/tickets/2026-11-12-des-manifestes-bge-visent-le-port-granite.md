# Des manifestes BGE visent le port actuellement servi par granite

- **État** : ouvert ; gabarits corrigés et intégrés sur master6dd116e8f, activation des copies utilisateur non vérifiée.
- **Famille** : produit
- **Gravité** : configuration ; un fournisseur déclaré peut ne pas servir le modèle requis.
- **Atteignable en service** : à qualifier ; aucune panne de ces backends observée dans cet audit.
- **Touche rag3weaver** : oui, gabarits notebook/validated-result et copies de configuration.
- **Pour** : ED, en coordination avec embarquement pour les identités de service.
- **Signalé par** : mémoire puis relecture orchestration pendant l'audit des services résidents du 12 novembre.
- **Ouvert le** : 12 novembre 2026

## Constat borné

Les gabarits `templates/backends/{notebook,validated-result}/backend.json`
et les copies sous `~/.config/rag3weaver/memoire-longue/backends/` déclarent
bge-m3/1024 avec adresse 127.0.0.1:7878. Le service observé sur ce port à
luciepc est granite-278m/768 ; BGE est sur 7879. Le routage dépend aussi du
poste et des surcharges : ne pas corriger toutes les adresses par substitution
globale. La mémoire réelle déclare BGE avec adresse vide, cas distinct.

## Fin attendue

Qualifier les chemins effectivement activés et la résolution de ModelSource,
puis rendre les gabarits et copies utiles cohérents avec leur fournisseur,
en préservant modèle, dimension et configuration par poste. Aucun index n'a
été ouvert ou réindexé pour ce constat ; aucune configuration utilisateur
modifiée. Une absence d'usage confirmé ne constitue pas une preuve de panne.

## Prise en charge ED — 12 novembre

Lot `cx-ed-bge-routage`, worktree neuf du même nom, base `41497ba4e`.
Périmètre annoncé : les deux manifestes et leurs README, tests dans `model_source.rs`
et `backend.rs`, rapport ED ; aucun changement du résolveur de production prévu.

Lecture du contrat : l'adresse explicite gagne sur l'environnement. Sans adresse, la
variable commune RAG3WEAVER_SERVICE_EMBED gagne sur l'alias RAG3WEAVER_EMBED_SERVICE ;
une liste est parcourue par identité servie. Refus par défaut, repli local seulement déclaré.
La correction envisagée compose ce contrat : retirer le port imposé des deux gabarits,
conserver BGE/1024 et le fournisseur service, laisser chaque poste fournir ses adresses.
Aucune substitution globale de port ni migration de déclaration n'est nécessaire.

Activation à distinguer : notebook déclare du vectoriel, validated-result déclare ses
signaux vides ; le second n'a pas besoin d'embarquement à son ouverture normale. Le raccord
exact est à prouver par chargement isolé. Une erreur de connect_embedder est un refus nommé ;
embedder_or_absent peut néanmoins ouvrir en mots seuls avec raison, selon le contrat existant.
Cela ne doit pas être annoncé comme une ouverture entièrement refusée ou comme du dense prêt.

Préfixe avant lourd : tests de résolution à constructeurs simulés et environnement injecté,
sans toucher l'environnement du processus ; chargement PreparedBackend des deux gabarits
sans base ni service. Vérifier sélection BGE dans un pool mixte, variable commune/alias,
adresse explicite prioritaire et erreur nommée sur granite, absence d'adresse sans repli local,
identité/dimension préservées et besoins vectoriels effectivement déclarés. Aucun modèle,
GPU, service réel, fichier .env/.vault, base ou configuration de Lucie lu ou modifié.

Les copies sous configuration utilisateur et la mémoire réelle BGE sans adresse restent
non qualifiées : leur activation n'est pas déduite de l'existence des gabarits. Ce lot ne
les modifie pas et ne fermera pas ce reste sans preuve et autorisation appropriées.

## Résultat du préfixe

Correction gabarits publiée `691ba0391` ; rouge source `5eef3fe01` : deux échecs
attendus et un succès. Vert sur le jeu exact matrice3 : trois succès, puis le témoin
existant embeddings/models.embed un succès ; aucun ignoré. Hash ELF et limites
dans le rapport ED du 12 novembre, section Gabarits BGE. Aucun service réel appelé.
Ce préfixe ne remplace pas la passe globale, encore à coordonner. Les copies utilisateur
restent à qualifier ; ne pas fermer le ticket au seul atterrissage des gabarits.

## Passe globale de la correction des gabarits

Source efdf90fb9 (production691ba0391), verdict composé VERT : F5check5/5,
check --lib --no-default-features rc0, lib2204/0/65, navigateur40/0/0,
PostgreSQL36/0/0 plus deux filets. Trois ELF/source/native29/220gabarits attestés ;
quiescence[], deadline initiale respectée. Rapport ED du12nov et preuves
I-luciepc/bge-routage-20261112/globale/verdict-compose.json aux deux postes.
Les gabarits livrés pourront être intégrés ; aucune activation des copies existantes
ni migration de configuration utilisateur n'est attestée. Ce reste conserve le ticket ouvert.

Correction des gabarits intégrée sur master `6dd116e8f70fd328acf3b45a89401e06f71365cb`.
Passe composée et nettoyage privés consignés au rapport ED ; aucune copie utilisateur
modifiée ou activée par ce lot. Le reste annoncé ci-dessus conserve ce ticket ouvert.
