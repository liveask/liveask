#![allow(clippy::panic, clippy::unwrap_used)]

fn get_git_hash() -> (String, String) {
    use std::process::Command;

    let branch = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output();
    if let Ok(branch_output) = branch {
        let branch_string = String::from_utf8_lossy(&branch_output.stdout);
        let commit = Command::new("git")
            .arg("rev-parse")
            .arg("--short")
            .arg("--verify")
            .arg("HEAD")
            .output();
        if let Ok(commit_output) = commit {
            let commit_string = String::from_utf8_lossy(&commit_output.stdout);

            return (
                branch_string.lines().next().unwrap_or("").into(),
                commit_string.lines().next().unwrap_or("").into(),
            );
        }

        panic!("Can not get git commit: {}", commit.unwrap_err());
    } else {
        panic!("Can not get git branch: {}", branch.unwrap_err());
    }
}

fn main() {
    let git = get_git_hash();
    println!("cargo:rustc-env=GIT_BRANCH={}", git.0);
    println!("cargo:rustc-env=GIT_HASH={}", git.1);
}
