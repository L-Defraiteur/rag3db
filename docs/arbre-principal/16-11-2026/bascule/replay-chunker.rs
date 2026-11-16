//! Diagnostic CPU only: actual Git chunker modules, no database/model/writer code.
//! Config shim supplies types; both Chunker implementations remain byte-identical Git sources.
use std::{io::{self,Read},process};
mod config {
    #[derive(Debug,Clone,PartialEq,Eq,Hash)]
    pub enum ChunkStrategy { Semantic, Fixed, Sentence, Lines, Markdown }
    pub struct ChunkingConfig {
        pub max_size:usize,pub overlap:usize,pub strategy:ChunkStrategy,
        pub max_lines:usize,pub overlap_lines:usize,
    }
}
#[path="chunker-before.rs"] mod before;
#[path="chunker-after.rs"] mod after;
fn main() {
    let mut text=String::new();
    if io::stdin().read_to_string(&mut text).is_err(){eprintln!("invalid UTF8 input");process::exit(1)}
    let args:Vec<_>=std::env::args().collect();
    if args.len()!=6 {eprintln!("expected max_size overlap max_lines overlap_lines strategy(lines)");process::exit(64)}
    let number=|i:usize|args[i].parse::<usize>().unwrap_or_else(|_|{eprintln!("invalid config number");process::exit(64)});
    if args[5]!="lines" {eprintln!("unexpected strategy");process::exit(64)}
    let c=config::ChunkingConfig{max_size:number(1),overlap:number(2),max_lines:number(3),overlap_lines:number(4),strategy:config::ChunkStrategy::Lines};
    println!("INPUT\t{}\t{}",text.len(),blake3::hash(text.as_bytes()).to_hex());
    for n in before::Chunker::new(before::ChunkerConfig::from(&c)).chunk(&text) {
        println!("BEFORE\t{}\t{}\t{}\t{}\t{}\t{}",n.index,n.start_byte,n.end_byte,n.core_start_byte,n.core_end_byte,blake3::hash(n.text.as_bytes()).to_hex());
    }
    for n in after::Chunker::new(after::ChunkerConfig::from(&c)).chunk(&text) {
        println!("AFTER\t{}\t{}\t{}\t{}\t{}\t{}",n.index,n.start_byte,n.end_byte,n.core_start_byte,n.core_end_byte,blake3::hash(n.text.as_bytes()).to_hex());
    }
}
