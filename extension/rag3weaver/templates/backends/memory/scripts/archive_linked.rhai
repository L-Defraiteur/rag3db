// archive_linked — le rapport de RelationBatchNode reçu par le port : l'apport est rangé sur sa fiche,
// sinon l'appel échoue en le disant (un refus de lien n'est pas une erreur du nœud, il se lit ici).
// input : #{ element, value: le rapport {processed, submitted, verified, refused?, warnings} }
let r = input.value;
if r.refused != () && r.refused.len() > 0 { throw `archive : le lien de « ${input.element.title} » est refusé : ${r.refused}`; }
if r.processed == 0 && r.unchanged == 0 { throw `archive : aucun lien posé pour « ${input.element.title} » : ${r}`; }
true
