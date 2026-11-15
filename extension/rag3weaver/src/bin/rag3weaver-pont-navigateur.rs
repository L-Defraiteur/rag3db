//! **Le relai du pont navigateur**, que Chrome lance par Native Messaging : il relaie l'extension
//! « rag3weaver pont navigateur » et l'hôte rag3weaver du poste, par son socket Unix. La sortie
//! standard est le canal de Chrome : rien d'autre que les octets de l'hôte n'y va.

fn main() {
    let socket = match rag3weaver::pont_navigateur::relai::chemin_du_socket() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[pont navigateur] {e}");
            std::process::exit(1);
        }
    };
    match rag3weaver::pont_navigateur::relai::relayer(std::io::stdin(), std::io::stdout(), &socket) {
        Ok(fin) => eprintln!("[pont navigateur] fin du relai : {fin:?} a fermé"),
        Err(e) => {
            eprintln!("[pont navigateur] {e}");
            std::process::exit(1);
        }
    }
}
