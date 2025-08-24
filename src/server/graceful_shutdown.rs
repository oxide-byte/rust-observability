use tokio::{
    select,
    signal::{
        ctrl_c,
        unix::{signal, SignalKind},
    },
};

#[cfg(not(target_os = "windows"))]
pub async fn graceful_shutdown() {
    let mut signal = signal(SignalKind::terminate()).unwrap();
    select! {
        _ = signal.recv() => {
            println!("Received SIGTERM, shutting down");
        }
        _ = ctrl_c() => {
            println!("Received SIGINT, shutting down");
        }
    }
}

#[cfg(target_os = "windows")]
pub async fn graceful_shutdown() {
    ctrl_c().await;
    println!("Received CTRL+C, shutting down");
}
