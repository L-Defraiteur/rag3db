//! Le corpus figé de l'empreinte de l'analyse.
pub mod outils;
pub use outils::Outil;

/// Une constante.
pub const LIMITE: usize = 8;

pub struct Catalog {
    pub taille: usize,
}

impl Catalog {
    pub fn new() -> Catalog {
        Catalog { taille: LIMITE }
    }

    pub fn grandir(&mut self) {
        let total = self.taille + 1;
        for i in 0..total {
            self.taille += i;
        }
    }
}

pub fn demarrer() -> usize {
    let mut c = Catalog::new();
    c.grandir();
    Outil::fabrique().valeur() + c.taille
}
