//! `rag3db-check-lib <binaire> <lib>` : le binaire et la lib du moteur
//! viennent-ils du même bâti ? Lu dans leurs sections ELF, **sans rien
//! charger** : ce binaire-ci ne lie pas la lib, il démarre même quand le
//! binaire contrôlé ne démarrerait pas (BIND_NOW, symbole manquant). Pour
//! l'`ExecStartPre` d'une unité. Sort à 0 en écrivant l'identifiant commun,
//! à 2 en nommant les deux identifiants, à 1 sur une erreur d'emploi.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [binary, lib] = args.as_slice() else {
        eprintln!("usage : rag3db-check-lib <binaire> <librag3db.so>");
        std::process::exit(1);
    };
    match rag3weaver::build_check::check_pair(std::path::Path::new(binary), std::path::Path::new(lib)) {
        Ok(id) => println!("même bâti : {id}"),
        Err(e) => {
            eprintln!("rag3db-check-lib : {e}");
            std::process::exit(2);
        }
    }
}
