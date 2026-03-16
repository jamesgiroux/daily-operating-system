use std::future::Future;
use std::time::Duration;

const MAX_RESTART_DELAY_SECS: u64 = 60;

pub fn spawn_supervised<F, Fut>(name: &'static str, f: F)
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut restart_count: u32 = 0;
        loop {
            match tokio::task::spawn(f()).await {
                Ok(()) => {
                    log::warn!("[supervisor] {name} exited normally, restarting");
                }
                Err(e) if e.is_panic() => {
                    restart_count += 1;
                    let delay = (5 * restart_count as u64).min(MAX_RESTART_DELAY_SECS);
                    log::error!(
                        "[supervisor] {name} panicked (restart #{restart_count}): {e:?}, \
                         restarting in {delay}s"
                    );
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                Err(e) => {
                    log::error!("[supervisor] {name} cancelled: {e:?}");
                    break;
                }
            }
        }
    });
}
