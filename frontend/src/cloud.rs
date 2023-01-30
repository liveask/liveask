#![allow(clippy::expect_used)]

use image::ImageOutputFormat;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashMap, io::Cursor};
use wordcloud_rs::{Colors, Token, WordCloud};
use yew::html;

lazy_static! {
    static ref RE_TOKEN: Regex = Regex::new(r"\w+").expect("regex error");
}

#[allow(clippy::cast_precision_loss)]
fn tokenize(text: &str) -> Vec<(Token, f32)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in RE_TOKEN.find_iter(text) {
        let token = token.as_str().to_ascii_lowercase();

        if token.len() <= 3
            || token == "what"
            || token == "where"
            || token == "your"
            || token == "with"
            || token == "kind"
            || token == "does"
            || token == "like"
            || token == "have"
            || token == "would"
        {
            continue;
        }

        *counts.entry(token).or_default() += 1;
    }

    counts
        .into_iter()
        .map(|(k, v)| (Token::Text(k), v as f32))
        .collect()
}

// Cargo.toml
//
// wordcloud-rs = "0.1"
// image="0.24"
// regex = "1.7"
// lazy_static = "1.4"
// base64 = "0.20"

pub fn create_cloud(text: &str) -> anyhow::Result<String> {
    let img = WordCloud::new()
        .colors(Colors::Rainbow {
            luminance: 100.,
            chroma: 0.,
        })
        .dim(600, 400)
        .generate(tokenize(text));
    let mut mem = Cursor::new(Vec::new());
    img.write_to(&mut mem, ImageOutputFormat::Png)?;

    Ok(base64_encode(&mem.into_inner()))
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine as _};

    general_purpose::STANDARD.encode(data)
}

pub fn cloud_as_yew_img(b64: &str) -> yew::Html {
    html! {
        <div class="cloud">
         <img src={format!("data:image/png;base64,{b64}")} />
        </div>
    }
}
