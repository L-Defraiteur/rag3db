//! Rendre un gabarit de résultats sur un payload JSON, avec le moteur de
//! rendu du backend : valider un gabarit écrit à la main avant de le poser là
//! où un backend en service le relit à chaque appel.
//! Usage : render_template <gabarit.md.jinja> <vue.json>
fn main() {
    let mut args = std::env::args().skip(1);
    let (gabarit, vue) = (args.next().expect("gabarit"), args.next().expect("vue.json"));
    let source = std::fs::read_to_string(gabarit).expect("lecture du gabarit");
    let vue: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(vue).expect("lecture de la vue")).expect("json");
    match rag3weaver::dataflow::render_nodes::rendre(&vue, &source) {
        Ok(texte) => print!("{texte}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
