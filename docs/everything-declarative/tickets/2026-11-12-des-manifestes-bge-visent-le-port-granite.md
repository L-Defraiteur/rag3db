# Des manifestes BGE visent le port actuellement servi par granite

- **État** : ouvert ; contradiction de déclaration relue, activation des copies concernées non vérifiée.
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
