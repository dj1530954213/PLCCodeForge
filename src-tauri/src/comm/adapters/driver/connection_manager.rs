//! Per-run Modbus connection lifecycle management.
//!
//! Goals:
//! - Reuse the same Modbus connection within a single runId (per ConnectionKey)
//! - Invalidate broken connections and reconnect on demand
//! - Keep driver implementations focused on protocol I/O (connect + read)

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::watch;
use uuid::Uuid;

use crate::comm::core::model::ConnectionProfile;

use super::{CommDriver, ConnectedClient, ConnectionKey, DriverError};

pub struct ConnectionManager {
    run_id: Uuid,
    conns: HashMap<ConnectionKey, ConnectedClient>,
}

impl ConnectionManager {
    pub fn new(run_id: Uuid) -> Self {
        Self {
            run_id,
            conns: HashMap::new(),
        }
    }

    pub fn invalidate(&mut self, key: &ConnectionKey, reason: &str) {
        if self.conns.remove(key).is_some() {
            eprintln!(
                "[comm][conn] runId={} invalidate key={} reason={}",
                self.run_id, key, reason
            );
        }
    }

    pub fn get_mut(&mut self, key: &ConnectionKey) -> Option<&mut ConnectedClient> {
        self.conns.get_mut(key)
    }

    pub async fn ensure_connected(
        &mut self,
        driver: &dyn CommDriver,
        profile: &ConnectionProfile,
        stop_rx: &watch::Receiver<bool>,
        timeout: Duration,
    ) -> Result<ConnectionKey, DriverError> {
        let key = driver.connection_key(profile)?;

        if self.conns.contains_key(&key) {
            eprintln!(
                "[comm][conn] runId={} reuse connection key={}",
                self.run_id, key
            );
            return Ok(key);
        }

        eprintln!("[comm][conn] runId={} connect key={}", self.run_id, key);

        let stop = wait_stop(stop_rx.clone());
        let connect = driver.connect(profile);

        let client = tokio::select! {
            _ = stop => {
                return Err(DriverError::Comm { message: "stop requested".to_string() });
            }
            res = tokio::time::timeout(timeout, connect) => {
                match res {
                    Ok(v) => v,
                    Err(_) => Err(DriverError::Timeout),
                }
            }
        }?;

        self.conns.insert(key.clone(), client);
        Ok(key)
    }
}

async fn wait_stop(mut stop_rx: watch::Receiver<bool>) {
    loop {
        if *stop_rx.borrow() {
            return;
        }
        if stop_rx.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use tokio_modbus::prelude::Slave;

    use crate::comm::adapters::driver::{ConnectFuture, DriverFuture};
    use crate::comm::core::model::RegisterArea;
    use crate::comm::core::plan::ReadJob;

    #[derive(Clone)]
    struct TestDriver {
        connects: Arc<AtomicUsize>,
    }

    impl CommDriver for TestDriver {
        fn connection_key(
            &self,
            profile: &ConnectionProfile,
        ) -> Result<ConnectionKey, DriverError> {
            match profile {
                ConnectionProfile::Tcp {
                    ip,
                    port,
                    device_id,
                    ..
                } => Ok(ConnectionKey::Tcp {
                    ip: ip.clone(),
                    port: *port,
                    unit_id: *device_id,
                }),
                _ => Err(DriverError::Comm {
                    message: "tcp profile required".to_string(),
                }),
            }
        }

        fn connect<'a>(&'a self, profile: &'a ConnectionProfile) -> ConnectFuture<'a> {
            let connects = Arc::clone(&self.connects);
            Box::pin(async move {
                let ConnectionProfile::Tcp { device_id, .. } = profile else {
                    return Err(DriverError::Comm {
                        message: "tcp profile required".to_string(),
                    });
                };
                connects.fetch_add(1, Ordering::SeqCst);

                let (stream, _peer) = tokio::io::duplex(64);
                Ok(tokio_modbus::client::tcp::attach_slave(
                    stream,
                    Slave(*device_id),
                ))
            })
        }

        fn read_with_client<'a>(
            &'a self,
            _client: &'a mut ConnectedClient,
            _job: &'a ReadJob,
        ) -> DriverFuture<'a> {
            Box::pin(async move {
                Err(DriverError::Comm {
                    message: "not used in this test".to_string(),
                })
            })
        }
    }

    fn sample_tcp_profile() -> ConnectionProfile {
        ConnectionProfile::Tcp {
            channel_name: "tcp-1".to_string(),
            device_id: 1,
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 10,
            ip: "127.0.0.1".to_string(),
            port: 502,
            timeout_ms: 200,
            retry_count: 0,
            poll_interval_ms: 200,
        }
    }

    #[tokio::test]
    async fn ensure_connected_reuses_connection_within_run() {
        let driver = TestDriver {
            connects: Arc::new(AtomicUsize::new(0)),
        };
        let profile = sample_tcp_profile();
        let (stop_tx, stop_rx) = watch::channel(false);
        let _keep = stop_tx;

        let mut mgr = ConnectionManager::new(Uuid::new_v4());
        let key1 = mgr
            .ensure_connected(&driver, &profile, &stop_rx, Duration::from_millis(200))
            .await
            .unwrap();
        let key2 = mgr
            .ensure_connected(&driver, &profile, &stop_rx, Duration::from_millis(200))
            .await
            .unwrap();

        assert_eq!(key1, key2);
        assert_eq!(driver.connects.load(Ordering::SeqCst), 1);
        assert!(mgr.get_mut(&key1).is_some());
    }

    #[tokio::test]
    async fn invalidate_forces_reconnect_next_time() {
        let driver = TestDriver {
            connects: Arc::new(AtomicUsize::new(0)),
        };
        let profile = sample_tcp_profile();
        let (stop_tx, stop_rx) = watch::channel(false);
        let _keep = stop_tx;

        let mut mgr = ConnectionManager::new(Uuid::new_v4());
        let key1 = mgr
            .ensure_connected(&driver, &profile, &stop_rx, Duration::from_millis(200))
            .await
            .unwrap();
        mgr.invalidate(&key1, "test");

        let _ = mgr
            .ensure_connected(&driver, &profile, &stop_rx, Duration::from_millis(200))
            .await
            .unwrap();

        assert_eq!(driver.connects.load(Ordering::SeqCst), 2);
    }
}
