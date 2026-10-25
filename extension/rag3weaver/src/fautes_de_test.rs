//! **Les fautes injectées par les témoins**, portées par le catalogue du test
//! (18 octobre 2026).
//!
//! Avant, deux paires de variables d'environnement
//! (`RAG3WEAVER_TEST_INSERT_FAULT_*`, `RAG3WEAVER_TEST_LINK_FAULT_*`) armaient la
//! faute **pour tout le processus** : depuis que les e2e sont un seul binaire,
//! une suite qui les posait faisait échouer les suites voisines du même
//! processus, au hasard de l'ordre des fils (vu le 18 octobre : un faux rouge
//! imputé à une lib du moteur). Une faute est maintenant un état **du catalogue
//! que le test a ouvert**, posé par le test sur ce catalogue
//! ([`crate::catalog::Catalog::injecter_faute_d_insertion`],
//! [`crate::catalog::Catalog::injecter_faute_de_lien`]) ; le catalogue la prête à
//! ses propres graphes de drain, et aucun autre catalogue ne la voit. Rien n'est
//! jamais lu dans l'environnement.

/// La clé du service prêté aux graphes de drain du catalogue.
pub const SERVICE: &str = "fautes_de_test";

/// Les fautes armées sur un catalogue : `(table ou relation, rang du groupe)`.
/// Sans faute armée, les deux sont `None` et rien ne change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FautesDeTest {
    /// Le n-ième groupe d'insertion de cette table échoue, comme si le moteur
    /// avait refusé l'insertion.
    pub insertion: Option<(String, usize)>,
    /// Le n-ième groupe de liens de cette relation échoue.
    pub lien: Option<(String, usize)>,
    /// La synchronisation d'une source s'arrête après ce paquet (le rang, à
    /// partir de 0), comme sur une demande d'arrêt. Avant le 19 octobre 2026,
    /// `RAG3WEAVER_TEST_STOP_AFTER_BATCH`, pour tout le processus.
    pub stop_after_batch: Option<usize>,
}
