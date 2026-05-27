use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Semaphore, SemaphorePermit};

/// Request queue that serializes Modbus operations per connection.
///
/// Enforces:
/// - Maximum concurrency (1 unless `parallel_unit_ids` is enabled)
/// - Minimum delay between consecutive requests (`command_delay`)
pub(crate) struct ModbusRequestQueue {
    semaphore: Semaphore,
    command_delay: Option<Duration>,
    last_request: Arc<Mutex<Option<tokio::time::Instant>>>,
}

impl ModbusRequestQueue {
    pub fn new(parallel: bool, command_delay_ms: Option<u64>) -> Self {
        let max_permits = if parallel { 4 } else { 1 };
        Self {
            semaphore: Semaphore::new(max_permits),
            command_delay: command_delay_ms.map(Duration::from_millis),
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    /// Acquire a permit, waiting for `command_delay` if needed.
    /// Returns a guard that releases the permit on drop.
    #[allow(dead_code)]
    pub async fn acquire(&self) -> crate::Result<SemaphorePermit<'_>> {
        let permit = self.semaphore.acquire().await.map_err(|_| anyhow::anyhow!("Queue closed"))?;

        // Enforce command_delay: wait until enough time has passed since last request
        if let Some(delay) = self.command_delay {
            let last = self.last_request.lock().await;
            if let Some(prev) = *last {
                let elapsed = prev.elapsed();
                if elapsed < delay {
                    tokio::time::sleep(delay - elapsed).await;
                }
            }
        }

        Ok(permit)
    }

    /// Record that a request just completed. Called before releasing the permit.
    #[allow(dead_code)]
    pub async fn record_completion(&self) {
        let mut last = self.last_request.lock().await;
        *last = Some(tokio::time::Instant::now());
    }
}

impl std::fmt::Debug for ModbusRequestQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModbusRequestQueue").field("command_delay", &self.command_delay).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn queue_basic_acquire_release() {
        let queue = ModbusRequestQueue::new(false, None);
        {
            let _permit = queue.acquire().await.unwrap();
            // Permit held
        }
        // Permit released
        let _permit2 = queue.acquire().await.unwrap();
    }

    #[tokio::test]
    async fn queue_command_delay_enforced() {
        let queue = ModbusRequestQueue::new(false, Some(100));
        let start = tokio::time::Instant::now();

        let permit = queue.acquire().await.unwrap();
        queue.record_completion().await;
        drop(permit);

        let _permit2 = queue.acquire().await.unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(90), "Expected ~100ms delay, got {:?}", elapsed);
    }

    #[tokio::test]
    async fn queue_parallel_allows_concurrent() {
        let queue = ModbusRequestQueue::new(true, None);
        let p1 = queue.acquire().await.unwrap();
        let _p2 = queue.acquire().await.unwrap();
        // Both permits held simultaneously
        drop(p1);
    }
}
