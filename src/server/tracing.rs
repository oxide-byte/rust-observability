use tracing::Event;
use tracing_subscriber::{EnvFilter, fmt, prelude::*, registry::LookupSpan};
use tracing_subscriber::fmt::{format::{DefaultFields, FormatEvent, FormatFields, Writer}, FmtContext};

struct AppIdWrapper<F> {
    inner: F,
    app: &'static str,
}

impl<F> AppIdWrapper<F> {
    const fn new(inner: F, app: &'static str) -> Self { Self { inner, app } }
}

impl<S, N, F> FormatEvent<S, N> for AppIdWrapper<F>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        // Inject the application tag before the regular formatting
        write!(writer, "application={} ", self.app)?;
        // Delegate to the inner formatter for the rest
        self.inner.format_event(ctx, writer, event)
    }
}

pub fn init_tracing_logging() {
    // Respect RUST_LOG, default to info if not set
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // Build a compact default formatter and wrap it to inject the app tag
    let default_format = fmt::format().compact();
    let app_format = AppIdWrapper::new(default_format, "rust-observability");

    let fmt_layer = fmt::layer()
        .fmt_fields(DefaultFields::new())
        .event_format(app_format);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}