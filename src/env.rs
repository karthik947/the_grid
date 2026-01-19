use std::env;

pub struct Environment {
    pub app_env: String,
    pub rust_log: String,
}

pub fn init_defaults() -> Environment {
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "prod".to_string());
    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "warn".to_string());

    Environment { app_env, rust_log }
}
