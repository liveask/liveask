#![allow(clippy::unwrap_used)]

use std::{
    collections::HashMap,
    env,
    fs::{self, read_to_string},
};

use handlebars::Handlebars;
use konst::eq_str;

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
        } else {
            panic!("Can not get git commit: {}", commit.unwrap_err());
        }
    } else {
        panic!("Can not get git branch: {}", branch.unwrap_err());
    }
}

enum LiveAskEnv {
    Prod,
    Beta,
    Local,
}

const fn la_env(env: Option<&str>) -> LiveAskEnv {
    match env {
        Some(env) if eq_str(env, "prod") => LiveAskEnv::Prod,
        Some(env) if eq_str(env, "beta") => LiveAskEnv::Beta,
        _ => LiveAskEnv::Local,
    }
}

fn process_html_template(git_hash: String) {
    let mut hb = Handlebars::new();
    hb.register_template_file("template", "index.html.hbs")
        .unwrap();

    let mut data: HashMap<&str, &str> = HashMap::new();
    data.insert("release", &git_hash);

    match la_env(env::var("LA_ENV").ok().as_deref()) {
        LiveAskEnv::Prod => {
            data.insert("metrical", "xPuojp2x_");
            data.insert("fathom", "XWFWPSUF");
            data.insert("sentry", "production");
        }
        LiveAskEnv::Beta => {
            data.insert("metrical", "2LaPi-sYg");
            data.insert("fathom", "OAMRSQQM");
            data.insert("sentry", "beta");
        }
        LiveAskEnv::Local => {
            data.insert("sentry", "local");
        }
    }

    let content = hb.render("template", &data).unwrap();

    const INDEX_FILE: &str = "index.html";

    if file_content_changed(INDEX_FILE, &content) {
        use std::io::Write;

        let mut output_file = fs::File::create(INDEX_FILE).unwrap();

        write!(output_file, "{}", content).unwrap();
    }
}

fn file_content_changed(path: &str, content: &str) -> bool {
    read_to_string(path)
        .map(|current_content| content != current_content)
        .unwrap_or_default()
}

fn main() {
    let git = get_git_hash();
    println!("cargo:rustc-env=GIT_BRANCH={}", git.0);
    println!("cargo:rustc-env=GIT_HASH={}", git.1);

    process_html_template(git.1);
}
