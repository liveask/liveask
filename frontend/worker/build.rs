use vergen::EmitBuilder;

fn main() -> anyhow::Result<()> {
    EmitBuilder::builder()
        .git_sha(true)
        .git_branch()
        .all_build()
        .emit()?;

    Ok(())
}
