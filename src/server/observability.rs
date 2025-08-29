use prometheus::{Encoder, Gauge, Opts, Registry, TextEncoder};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use sysinfo::{Pid, RefreshKind, System};
use tokio::time;

use axum::{middleware::Next, response::Response};

use opentelemetry::{global, KeyValue};
use opentelemetry_prometheus::PrometheusExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;

static PROCESS_REGISTRY: OnceLock<Registry> = OnceLock::new();
static CPU_USAGE_GAUGE: OnceLock<Gauge> = OnceLock::new();
static MEMORY_RSS_GAUGE: OnceLock<Gauge> = OnceLock::new();
static MEMORY_VMS_GAUGE: OnceLock<Gauge> = OnceLock::new();
static OTEL_REGISTRY: OnceLock<Registry> = OnceLock::new();
static HTTP_REQ_COUNTER: OnceLock<opentelemetry::metrics::Counter<u64>> = OnceLock::new();
static HTTP_REQ_HIST_MS: OnceLock<opentelemetry::metrics::Histogram<f64>> = OnceLock::new();

pub fn setup_observability() {
    // Initialize OpenTelemetry metrics with Prometheus exporter (standard metrics)
    setup_otel_metrics();
    init_http_instruments();

    let registry = PROCESS_REGISTRY.get_or_init(Registry::new);

    let cpu_gauge = Gauge::with_opts(
        Opts::new(
            "process_cpu_usage",
            "Process CPU usage percentage (can exceed 100% on multi-core)",
        )
            .namespace("rust_observability"),
    )
        .expect("create cpu gauge");

    let rss_gauge = Gauge::with_opts(
        Opts::new(
            "process_memory_rss_bytes",
            "Process resident set size (RSS) in bytes",
        )
            .namespace("rust_observability"),
    )
        .expect("create rss gauge");

    // On macOS, virtual address space can be extremely large and misleading for RAM usage.
    // Default behavior: disable VMS metric on macOS unless explicitly enabled via env var.
    let enable_vms = {
        #[cfg(target_os = "macos")]
        {
            std::env::var("RUST_OBSERVABILITY_VMS")
                .map(|v| {
                    let v = v.trim();
                    v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on")
                })
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            true
        }
    };

    let vms_gauge_opt = if enable_vms {
        let g = Gauge::with_opts(
            Opts::new(
                "process_memory_virtual_bytes",
                "Process virtual memory size (address space) in bytes; note: not resident RAM. On macOS this can be tens of GB due to reserved spaces.",
            )
                .namespace("rust_observability"),
        )
            .expect("create vms gauge");
        registry
            .register(Box::new(g.clone()))
            .expect("register vms gauge");
        Some(g)
    } else {
        None
    };

    registry
        .register(Box::new(cpu_gauge.clone()))
        .expect("register cpu gauge");
    registry
        .register(Box::new(rss_gauge.clone()))
        .expect("register rss gauge");

    CPU_USAGE_GAUGE.set(cpu_gauge).ok();
    MEMORY_RSS_GAUGE.set(rss_gauge).ok();
    if let Some(g) = vms_gauge_opt.clone() {
        MEMORY_VMS_GAUGE.set(g).ok();
    }

    tokio::spawn(async move {
        let mut sys = System::new_with_specifics(RefreshKind::everything());

        let pid = Pid::from_u32(std::process::id());

        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            sys.refresh_all();

            if let Some(proc_) = sys.process(pid) {
                let cpu_pct = proc_.cpu_usage() as f64;
                if let Some(g) = CPU_USAGE_GAUGE.get() {
                    g.set(cpu_pct);
                }

                let rss_bytes = proc_.memory();
                if let Some(g) = MEMORY_RSS_GAUGE.get() {
                    g.set(rss_bytes as f64);
                }

                let mut _vms_bytes_opt: Option<u64> = None;
                if MEMORY_VMS_GAUGE.get().is_some() {
                    let v = proc_.virtual_memory();
                    if let Some(g) = MEMORY_VMS_GAUGE.get() {
                        g.set(v as f64);
                    }
                    _vms_bytes_opt = Some(v);
                }

                #[cfg(debug_assertions)]
                {
                    let rss_mib = (rss_bytes as f64) / (1024.0 * 1024.0);
                    match _vms_bytes_opt {
                        Some(vms_bytes) => {
                            let vms_mib = (vms_bytes as f64) / (1024.0 * 1024.0);
                            println!(
                                "Process metrics => CPU: {cpu_pct:.2}% | RSS: {rss_bytes} bytes ({rss_mib:.2} MiB) | VMS: {vms_bytes} bytes ({vms_mib:.2} MiB)"
                            );
                        }
                        None => {
                            println!(
                                "Process metrics => CPU: {cpu_pct:.2}% | RSS: {rss_bytes} bytes ({rss_mib:.2} MiB) | VMS: disabled (set RUST_OBSERVABILITY_VMS=1 to enable)"
                            );
                        }
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                eprintln!("Could not find current process info for PID {pid}");
            }
        }
    });
}

fn setup_otel_metrics() {
    // Ensure we only set up once
    let registry = OTEL_REGISTRY.get_or_init(Registry::new).clone();

    // Build Prometheus exporter backed by our registry
    let exporter: PrometheusExporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()
        .expect("build otel prometheus exporter");

    // Build and install MeterProvider (no instruments registered here to keep build stable across API changes)
    let provider: SdkMeterProvider = SdkMeterProvider::builder().with_reader(exporter).build();

    global::set_meter_provider(provider);
}

fn init_http_instruments() {
    let meter = global::meter("rust_observability.http");
    let counter = meter
        .u64_counter("http_server_requests_total")
        .with_description("Total number of HTTP requests handled")
        .build();
    let hist = meter
        .f64_histogram("http_server_request_duration_ms")
        .with_description("HTTP server request duration in milliseconds")
        .build();
    let _ = HTTP_REQ_COUNTER.set(counter);
    let _ = HTTP_REQ_HIST_MS.set(hist);
}

pub async fn http_metrics_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().as_str().to_owned();
    // Using raw path; matched route templates aren't available at this layer without extra setup
    let path = req.uri().path().to_owned();
    let res = next.run(req).await;
    let status = res.status().as_u16();

    if let Some(c) = HTTP_REQ_COUNTER.get() {
        c.add(
            1,
            &[
                KeyValue::new("method", method.clone()),
                KeyValue::new("path", path.clone()),
                KeyValue::new("status", status.to_string()),
            ],
        );
    }
    if let Some(h) = HTTP_REQ_HIST_MS.get() {
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        h.record(
            ms,
            &[
                KeyValue::new("method", method),
                KeyValue::new("path", path),
                KeyValue::new("status", status.to_string()),
            ],
        );
    }

    res
}

pub fn render_process_metrics() -> String {
    let registry = PROCESS_REGISTRY
        .get()
        .expect("process registry not initialized");
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        eprintln!("Failed to encode process metrics: {e}");
    }
    String::from_utf8(buffer).unwrap_or_default()
}

pub fn render_otel_metrics() -> String {
    let registry = match OTEL_REGISTRY.get() {
        Some(r) => r,
        None => return String::new(),
    };
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        eprintln!("Failed to encode OpenTelemetry metrics: {e}");
    }
    String::from_utf8(buffer).unwrap_or_default()
}