use crate::{app::settings, outputln};

pub async fn stage2() -> anyhow::Result<()> {
    outputln!("starting", stage: 2);

    settings::load().await?;

    Ok(())
}
