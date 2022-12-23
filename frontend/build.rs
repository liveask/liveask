#![allow(clippy::unwrap_used)]

use std::{
    collections::HashMap,
    env,
    fs::{self, read_to_string},
};

use handlebars::Handlebars;
use konst::eq_str;
use vergen::{Config, ShaKind};

fn get_git_hash() -> String {
    use std::process::Command;

    let commit = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("--verify")
        .arg("HEAD")
        .output();
    if let Ok(commit_output) = commit {
        let commit_string = String::from_utf8_lossy(&commit_output.stdout);

        return commit_string.lines().next().unwrap_or("").into();
    }

    panic!("Can not get git commit: {}", commit.unwrap_err());
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

fn process_html_template(git_hash: &str) {
    const INDEX_FILE: &str = "index.html";

    let mut hb = Handlebars::new();
    hb.register_template_file("template", "index.html.hbs")
        .unwrap();

    let mut data: HashMap<&str, &str> = HashMap::new();
    data.insert("release", git_hash);

    match la_env(env::var("LA_ENV").ok().as_deref()) {
        LiveAskEnv::Prod => {
            println!("cargo:warning=env is prod");
            data.insert("metrical", "xPuojp2x_");
            data.insert("fathom", "XWFWPSUF");
            data.insert("sentry", "production");
        }
        LiveAskEnv::Beta => {
            println!("cargo:warning=env is beta");

            data.insert("metrical", "2LaPi-sYg");
            data.insert("fathom", "OAMRSQQM");
            data.insert("sentry", "beta");
        }
        LiveAskEnv::Local => {
            println!("cargo:warning=env is local");

            data.insert("sentry", "local");
        }
    }

    let content = hb.render("template", &data).unwrap();

    if file_content_changed(INDEX_FILE, &content) {
        use std::io::Write;

        let mut output_file = fs::File::create(INDEX_FILE).unwrap();

        write!(output_file, "{content}").unwrap();
    }
}

fn file_content_changed(path: &str, content: &str) -> bool {
    read_to_string(path)
        .map(|current_content| content != current_content)
        .unwrap_or_default()
}

fn main() -> anyhow::Result<()> {
    let mut config = Config::default();
    *config.git_mut().sha_kind_mut() = ShaKind::Short;

    vergen::vergen(config)?;

    let git = get_git_hash();
    process_html_template(&git);

    Ok(())
}
