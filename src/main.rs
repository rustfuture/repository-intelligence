use repository_intelligence::Index;
use std::{env, path::Path};

fn main() {
    let mut args = env::args().skip(1);
    let root = args.next().unwrap_or_else(|| ".".into());
    let query = args.collect::<Vec<_>>().join(" ");
    if query.trim().is_empty() {
        eprintln!("usage: repository-intelligence <repo> <query...>");
        std::process::exit(2);
    }
    let index = Index::build(Path::new(&root)).expect("index repository");
    for hit in index.search(&query, 10) {
        println!("{}:{}\t{}", hit.path.display(), hit.line, hit.text.trim());
    }
}
