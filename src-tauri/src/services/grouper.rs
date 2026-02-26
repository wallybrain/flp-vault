use crate::matcher::{propose_groups, ProposedGroup};
use crate::store::files::list_all_files;
use rusqlite::Connection;
use std::sync::Mutex;

pub fn run_grouper(db: &Mutex<Connection>, threshold: f32) -> Vec<ProposedGroup> {
    let files = list_all_files(db);
    propose_groups(&files, threshold)
}
