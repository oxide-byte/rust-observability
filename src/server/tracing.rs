pub fn init_tracing_logging() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    // Respect RUST_LOG, default to info if not set
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let fmt_layer = fmt::layer()
        .with_target(false) // omit log target for brevity
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_level(true)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}