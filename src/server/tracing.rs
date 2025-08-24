use std::process;
use tracing::Event;
use tracing_loki::url::Url;
use tracing_subscriber::fmt::{
    format::{DefaultFields, FormatEvent, FormatFields, Writer},
    FmtContext,
};
use tracing_subscriber::{fmt, prelude::*, registry::LookupSpan, EnvFilter};

struct AppIdWrapper<F> {
    inner: F,
    app: &'static str,
}

impl<F> AppIdWrapper<F> {
    const fn new(inner: F, app: &'static str) -> Self {
        Self { inner, app }
    }
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

    let (loki_layer, task) = tracing_loki::builder()
        .label("host", "rust-observability-host")
        .unwrap()
        .extra_field("pid", format!("{}", process::id()))
        .unwrap()
        .build_url(Url::parse("http://loki:3100").unwrap())
        .unwrap();

    let fmt_layer = fmt::layer()
        .fmt_fields(DefaultFields::new())
        .event_format(app_format);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(loki_layer)
        .with(fmt_layer)
        .init();

    tokio::spawn(task);
}
