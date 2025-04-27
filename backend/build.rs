#![allow(clippy::unwrap_used)]

use vergen_gitcl::{Emitter, GitclBuilder};

// build.rs main func
fn main() {
    let gitcl = GitclBuilder::default()
        .branch(true)
        .sha(true)
        .build()
        .unwrap();
    Emitter::default()
        .add_instructions(&gitcl)
        .unwrap()
        .emit()
        .unwrap();
}
