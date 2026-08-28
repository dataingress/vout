use std::sync::OnceLock;

use config::Config;

pub mod defaults;
pub mod object;

static CONFIG: OnceLock<object::Root> = OnceLock::new();

pub fn get<'a>() -> &'a object::Root {
    CONFIG.get().expect("config not loaded but used")
}

fn create_config_settings(path: Option<String>) -> anyhow::Result<Config> {
    let mut settings = Config::builder().add_source(
        config::Environment::with_prefix("VOUT")
            .prefix_separator("_")
            .separator("-"),
    );

    if let Some(path) = path {
        settings = settings.add_source(config::File::with_name(&path))
    }

    Ok(settings.build()?)
}

pub fn load(path: Option<String>) -> anyhow::Result<()> {
    let settings = create_config_settings(path)?;
    let config: object::Root = settings.try_deserialize()?;

    assert!(CONFIG.set(config).is_ok(), "config already loaded");

    Ok(())
}
