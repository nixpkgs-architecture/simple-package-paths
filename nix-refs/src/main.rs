use std::{env, fs};

use rowan::ast::AstNode;
use std::path::Path;
use std::path::PathBuf;
use std::collections::hash_map::HashMap;
use std::collections::hash_set::HashSet;
use std::path::Component;

fn process_file(tree: &mut FileTree, file: &Path) {
    // eprintln!("Processing file {:?}", file);
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
            eprintln!("error parsing file, ignoring: {}", err);
            // std::process::exit(2);
            return
        }
    };

    for node in parse.syntax().descendants() {
        if node.kind() != rnix::SyntaxKind::NODE_PATH {
            continue
        }
        // println!("{:#?}", node);
        if node.children().count() != 0 {
            eprintln!("Warning: Path contains subexpressions: {}", node.text());
            continue
        }
        let target = node.text().to_string();
        if str::starts_with(&target, "<") {
            eprintln!("Warning: Skipping search path: {}", node.text());
            continue
        }

        let reference = Reference { file: file.to_path_buf(), reference: target.to_owned() };
        let mut path_ref = std::path::PathBuf::from(&target);
        
        // Reference is relative to the file's parent
        let mut actual_referred_path = std::path::PathBuf::from(file.parent().unwrap());
        actual_referred_path.push(&target);

        actual_referred_path = match actual_referred_path.canonicalize() {
            Ok(r) => r,
            Err(_) => {
                eprintln!("Warning: Could not canonicalize reference {:?}, ignoring this reference", reference);
                continue
            }
        };

        if actual_referred_path.is_dir() {
            path_ref = path_ref.join("default.nix");
        } else if ! actual_referred_path.is_file() {
            eprintln!("Warning: Path is neither a file nor a directory, ignoring it: {:?}", reference);
            continue
        }


        process_reference(tree, reference, path_ref);
    }
}

fn process_reference(tree: &mut FileTree, reference: Reference, path_ref: PathBuf) {
    // eprintln!("Processing {:?}, or {:?}", reference, path_ref);
    let mut dir = reference.file.parent().unwrap().to_path_buf();
    tree.prevent_move(reference.file.to_path_buf(), reference.clone());
    for component in path_ref.components() {
        match component {
            std::path::Component::ParentDir => {
                tree.prevent_move(dir.to_owned(), reference.clone());
                dir = dir.parent().unwrap().to_path_buf();
            }
            std::path::Component::Normal(osstr) => {
                dir = dir.join(osstr).to_path_buf(); 
                tree.prevent_move(dir.to_owned(), reference.clone());
                tree.prevent_rename(dir.to_owned(), reference.clone());
            }
            std::path::Component::CurDir => {
            }
            other => {
                eprintln!("Warning: reference {:?} contains unsupported component {:?}", reference, other);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct Reference {
    file: PathBuf,
    reference: String,
}

#[derive(Debug)]
struct FileTree {
    children: HashMap<std::ffi::OsString, FileTree>,
    prevents_moves: HashSet<Reference>,
    prevents_renames: HashSet<Reference>,
}

impl FileTree {
    fn new() -> FileTree {
        FileTree {
            children: HashMap::new(),
            prevents_moves: HashSet::new(),
            prevents_renames: HashSet::new(),
        }
    }

    fn from(path: &Path) -> FileTree {
        let mut tree = FileTree::new();
        let metadata = path.metadata().unwrap();
        if ! metadata.is_dir() {
            return tree;
        }
        for entry in path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let subtree = FileTree::from(&path);
            tree.children.insert(entry.file_name(), subtree);
        }
        tree
    }

    fn _prevent_move(&mut self, components: &mut std::path::Components, reference: Reference) {
        match components.next() {
            Some(Component::CurDir) => self._prevent_move(components, reference),
            Some(Component::Normal(osstr)) => {
                if let Some(x) = self.children.get_mut(osstr) {
                    x._prevent_move(components, reference);
                } else {
                    eprintln!("child {:?} does not exist", osstr);
                }
            },
            None => {
                self.prevents_moves.insert(reference);
                ()
            },
            other => {
                eprintln!("Unexpected component {:?}", other);
                std::process::exit(3);
            }
        }
    }

    fn _prevent_rename(&mut self, components: &mut std::path::Components, reference: Reference) {
        match components.next() {
            Some(Component::CurDir) => self._prevent_rename(components, reference),
            Some(Component::Normal(osstr)) => {
                if let Some(x) = self.children.get_mut(osstr) {
                    x._prevent_rename(components, reference);
                } else {
                    eprintln!("child {:?} does not exist", osstr);
                }
            },
            None => {
                self.prevents_renames.insert(reference);
                ()
            },
            other => {
                eprintln!("Unexpected component {:?}", other);
                std::process::exit(3);
            }
        }
    }

    fn prevent_move(&mut self, path: PathBuf, reference: Reference) {
        eprintln!("Preventing the move of path {:?} due to reference {:?}", path, reference);
        self._prevent_move(&mut path.components(), reference);
    }

    fn prevent_rename(&mut self, path: PathBuf, reference: Reference) {
        eprintln!("Preventing the rename of path {:?} due to reference {:?}", path, reference);
        self._prevent_rename(&mut path.components(), reference);
    }

    fn safe_to_move(&self, path: PathBuf) {
        if self.prevents_moves.is_empty() {
            println!("{:?}", path);
        } else if self.prevents_moves.len() == 1
            && self.prevents_moves.iter().next().unwrap().file
                == std::path::PathBuf::from("./pkgs/top-level/all-packages.nix") {
            println!("Only referenced by ./pkgs/top-level/all-packages.nix: {:?}", path);
        }
        for (name, tree) in &self.children {
            tree.safe_to_move(path.join(name));
        };
    }
}

fn find_references(tree: &mut FileTree, path: &Path) {
    let metadata = path.metadata().unwrap();
    if metadata.is_dir() {
        for entry in path.read_dir().unwrap() {
            find_references(tree, &entry.unwrap().path());
        }
    } else if metadata.is_file() && path.extension() == Some(&std::ffi::OsStr::new("nix")) {
        process_file(tree, path);
    }
}

fn main() {
    let mut iter = env::args().skip(1);
    let root = match iter.next() {
        None => {
            eprintln!("Usage: dump-ast <file>");
            return;
        },
        Some(file) => file
    };
    let root_path = Path::new(&root);
    std::env::set_current_dir(root_path).unwrap();
    let cur_dir = Path::new(".");
    let mut tree = FileTree::from(cur_dir);
    find_references(&mut tree, cur_dir);

    // println!("{:#?}", tree);

    tree.safe_to_move(cur_dir.to_path_buf());
    // Print all files that are safe to move
}
