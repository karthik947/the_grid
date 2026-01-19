use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, LoggerHandle, Naming, detailed_format};
use std::{env, path::PathBuf, sync::OnceLock};

use crate::Result;

static LOGGER: OnceLock<LoggerHandle> = OnceLock::new();

pub fn initialize_logger() -> Result<()> {
    if LOGGER.get().is_some() {
        return Ok(());
    }

    let is_dev = matches!(
        env::var("APP_ENV").as_deref(),
        Ok("dev") | Ok("development")
    );

    let file_spec = if is_dev {
        FileSpec::default().directory("logs")
    } else {
        FileSpec::default().directory(default_log_dir())
    };

    let mut logger = Logger::try_with_env_or_str("off")
        .map_err(|err| err.to_string())?
        .log_to_file(file_spec);

    if is_dev {
        logger = logger.rotate(
            Criterion::Size(5_000_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(3),
        );
    }

    let handle = logger
        .format(detailed_format)
        .start()
        .map_err(|err| err.to_string())?;
    let _ = LOGGER.set(handle);
    Ok(())
}

fn default_log_dir() -> PathBuf {
    let app_name = env!("CARGO_PKG_NAME");

    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Logs")
            .join(app_name)
    }

    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA")
            .or_else(|| env::var_os("APPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(app_name)
            .join("Logs")
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(dir) = env::var_os("XDG_STATE_HOME") {
            PathBuf::from(dir).join(app_name).join("logs")
        } else if let Some(dir) = env::var_os("XDG_DATA_HOME") {
            PathBuf::from(dir).join(app_name).join("logs")
        } else if let Some(home) = env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("state")
                .join(app_name)
                .join("logs")
        } else {
            PathBuf::from("logs")
        }
    }
}
