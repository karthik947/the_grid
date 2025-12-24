use env_logger::Env;
use std::sync::OnceLock;

use crate::Result;

static LOGGER: OnceLock<()> = OnceLock::new();

pub fn initialize_logger() -> Result<()> {
    LOGGER.get_or_init(|| {
        let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();
    });
    Ok(())
}
