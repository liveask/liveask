use vergen::EmitBuilder;

fn main() -> anyhow::Result<()> {
    EmitBuilder::builder().git_sha(true).git_branch().emit()?;

    Ok(())
}
