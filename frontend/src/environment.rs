use konst::eq_str;

pub enum LiveAskEnv {
    Prod,
    Beta,
    Local,
}

pub const fn la_env(env: Option<&str>) -> LiveAskEnv {
    match env {
        Some(env) if eq_str(env, "prod") => LiveAskEnv::Prod,
        Some(env) if eq_str(env, "beta") => LiveAskEnv::Beta,
        _ => LiveAskEnv::Local,
    }
}
