use std::path::PathBuf;

#[derive(Debug)]
pub struct ArchivalNode {
    pub path: PathBuf,
    pub is_leaf: bool,
}

