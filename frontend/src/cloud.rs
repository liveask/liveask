use image::ImageOutputFormat;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashMap, io::Cursor};
use wordcloud_rs::{Token, WordCloud};

lazy_static! {
    static ref RE_TOKEN: Regex = Regex::new(r"\w+").unwrap();
}

fn tokenize(text: String) -> Vec<(Token, f32)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in RE_TOKEN.find_iter(&text) {
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

//NOTE: how to use in view
// let cloud = if let Some(cloud) = &self.image {
//     html!(<img class="cloud" src={format!("data:image/png;base64,{}",cloud)} />)
// } else {
//     html!()
// };
pub fn create_cloud(text: String) -> String {
    let img = WordCloud::new().generate(tokenize(text));
    let mut mem = Cursor::new(Vec::new());
    img.write_to(&mut mem, ImageOutputFormat::Png).unwrap();
    base64::encode(mem.into_inner())
}
