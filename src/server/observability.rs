use axum_prometheus::PrometheusMetricLayer;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use prometheus::{Encoder, Gauge, Opts, Registry, TextEncoder};

static PROCESS_REGISTRY: OnceLock<Registry> = OnceLock::new();
static CPU_USAGE_GAUGE: OnceLock<Gauge> = OnceLock::new();
static MEMORY_RSS_GAUGE: OnceLock<Gauge> = OnceLock::new();
static MEMORY_VMS_GAUGE: OnceLock<Gauge> = OnceLock::new();

pub fn setup_observability() -> (PrometheusMetricLayer<'static>, axum_prometheus::Handle) {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

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
            std::env::var("RUST_OBSERVABILITY_VMS").map(|v| {
                let v = v.trim();
                v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on")
            }).unwrap_or(false)
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
    if let Some(g) = vms_gauge_opt.clone() { MEMORY_VMS_GAUGE.set(g).ok(); }

    tokio::spawn(async move {
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );

        let pid = Pid::from_u32(std::process::id());

        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            sys.refresh_processes_specifics(ProcessRefreshKind::everything());

            if let Some(proc_) = sys.process(pid) {
                let cpu_pct = proc_.cpu_usage() as f64;
                if let Some(g) = CPU_USAGE_GAUGE.get() {
                    g.set(cpu_pct);
                }

                // sysinfo >= 0.30 returns memory in bytes already
                let rss_bytes = proc_.memory() as u64;
                if let Some(g) = MEMORY_RSS_GAUGE.get() {
                    g.set(rss_bytes as f64);
                }

                // Virtual memory size in bytes
                let mut vms_bytes_opt: Option<u64> = None;
                if MEMORY_VMS_GAUGE.get().is_some() {
                    let v = proc_.virtual_memory() as u64;
                    if let Some(g) = MEMORY_VMS_GAUGE.get() { g.set(v as f64); }
                    vms_bytes_opt = Some(v);
                }

                #[cfg(debug_assertions)]
                {
                    let rss_mib = (rss_bytes as f64) / (1024.0 * 1024.0);
                    match vms_bytes_opt {
                        Some(vms_bytes) => {
                            let vms_mib = (vms_bytes as f64) / (1024.0 * 1024.0);
                            println!(
                                "Process metrics => CPU: {:.2}% | RSS: {} bytes ({:.2} MiB) | VMS: {} bytes ({:.2} MiB)",
                                cpu_pct, rss_bytes, rss_mib, vms_bytes, vms_mib
                            );
                        }
                        None => {
                            println!(
                                "Process metrics => CPU: {:.2}% | RSS: {} bytes ({:.2} MiB) | VMS: disabled (set RUST_OBSERVABILITY_VMS=1 to enable)",
                                cpu_pct, rss_bytes, rss_mib
                            );
                        }
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                eprintln!("Could not find current process info for PID {}", pid);
            }
        }
    });

    (prometheus_layer, axum_prometheus::Handle(metric_handle))
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