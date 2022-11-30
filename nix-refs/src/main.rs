use std::{env, fs};

use rowan::ast::AstNode;

fn main() {
    let mut iter = env::args().skip(1);
    let file = match iter.next() {
        None => {
            eprintln!("Usage: dump-ast <file>");
            return;
        },
        Some(file) => file
    };
    let content = match fs::read_to_string(file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("error reading file: {}", err);
            std::process::exit(1);
        }
    };
    let parse = match rnix::Root::parse(&content).ok() {
        Ok(parse) => parse,
        Err(err) => {
            eprintln!("error parsing file: {}", err);
            std::process::exit(2);
        }
    };

    for node in parse.syntax().descendants() {
        if node.kind() == rnix::SyntaxKind::NODE_PATH {
            if node.children().count() != 0 {
                eprintln!("Warning: Path contains subexpressions: {}", node.text())
            } else {
                println!("{}", node.text());
            }
        }
    }
}
