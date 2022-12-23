use vergen::{Config, ShaKind};

fn main() -> anyhow::Result<()> {
    let mut config = Config::default();
    *config.git_mut().sha_kind_mut() = ShaKind::Short;

    vergen::vergen(config)?;

    Ok(())
}
