//! 通讯地址采集并生成模块：执行引擎（engine）。
//!
//! 本文件提供“执行一次计划/后台 run”的能力；底层通讯由 driver（Modbus TCP/RTU）实现。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::comm::adapters::driver::connection_manager::ConnectionManager;
use crate::comm::adapters::driver::{CommDriver, ConnectionKey, DriverError, RawReadData};
use crate::comm::core::codec::{
    decode_from_bits, decode_from_registers, DecodeError, DecodedValue,
};
use crate::comm::core::model::{
    CommPoint, CommWarning, ConnectionProfile, Quality, RunStats, SampleResult,
};
use crate::comm::core::plan::{PlannedPointRead, ReadJob, ReadPlan};

pub async fn execute_plan_once(
    driver: &dyn CommDriver,
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
    plan: &ReadPlan,
) -> (Vec<SampleResult>, RunStats) {
    let run_id = Uuid::nil();
    let mut conn_mgr = ConnectionManager::new(run_id);
    let (stop_tx, stop_rx) = watch::channel(false);
    let _keep_sender_alive = stop_tx;

    execute_plan_once_with_manager(driver, &mut conn_mgr, &stop_rx, profiles, points, plan)
        .await
        .unwrap_or_else(|| {
            let now = Utc::now();
            let results: Vec<SampleResult> = points
                .iter()
                .map(|p| SampleResult {
                    point_key: p.point_key,
                    value_display: "".to_string(),
                    quality: Quality::ConfigError,
                    timestamp: now,
                    duration_ms: 0,
                    error_message: "cancelled".to_string(),
                })
                .collect();
            let stats = calc_stats(&results);
            (results, stats)
        })
}

async fn execute_plan_once_with_manager(
    driver: &dyn CommDriver,
    conn_mgr: &mut ConnectionManager,
    stop_rx: &watch::Receiver<bool>,
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
    plan: &ReadPlan,
) -> Option<(Vec<SampleResult>, RunStats)> {
    let profiles_by_channel = build_profile_map(profiles);
    let now = Utc::now();

    let mut results_by_key: HashMap<Uuid, SampleResult> = HashMap::new();
    let mut groups: HashMap<ConnectionKey, Vec<(&ConnectionProfile, &ReadJob)>> = HashMap::new();
    let mut group_order: Vec<ConnectionKey> = Vec::new();

    for job in &plan.jobs {
        let profile = match profiles_by_channel.get(job.channel_name.as_str()) {
            Some(profile) => *profile,
            None => {
                mark_job_failure(
                    &mut results_by_key,
                    job,
                    now,
                    Quality::ConfigError,
                    "missing connection profile",
                    0,
                );
                continue;
            }
        };

        let key = match driver.connection_key(profile) {
            Ok(k) => k,
            Err(DriverError::Timeout) => {
                mark_job_failure(
                    &mut results_by_key,
                    job,
                    now,
                    Quality::Timeout,
                    "timeout",
                    0,
                );
                continue;
            }
            Err(DriverError::Comm { message }) => {
                mark_job_failure(
                    &mut results_by_key,
                    job,
                    now,
                    Quality::CommError,
                    &message,
                    0,
                );
                continue;
            }
        };

        if !groups.contains_key(&key) {
            group_order.push(key.clone());
        }
        groups.entry(key).or_default().push((profile, job));
    }

    let mut reconnected_keys: HashSet<ConnectionKey> = HashSet::new();

    for key in group_order {
        if *stop_rx.borrow() {
            return None;
        }
        let Some(items) = groups.get(&key) else {
            continue;
        };
        let Some((profile0, _)) = items.first() else {
            continue;
        };

        let timeout = Duration::from_millis(profile_timeout_ms(profile0) as u64);
        if let Err(err) = conn_mgr
            .ensure_connected(driver, profile0, stop_rx, timeout)
            .await
        {
            if *stop_rx.borrow() {
                return None;
            }
            let (quality, message) = match err {
                DriverError::Timeout => (Quality::Timeout, "timeout".to_string()),
                DriverError::Comm { message } => (Quality::CommError, message),
            };
            for (_, job) in items {
                mark_job_failure(&mut results_by_key, job, now, quality.clone(), &message, 0);
            }
            continue;
        };

        for (profile, job) in items {
            let (raw, duration_ms) = read_with_retry(
                driver,
                conn_mgr,
                stop_rx,
                &key,
                profile,
                job,
                &mut reconnected_keys,
            )
            .await?;
            match raw {
                Ok(data) => decode_job_data(&mut results_by_key, job, data, now, duration_ms),
                Err(err) => {
                    let (quality, message) = match err {
                        DriverError::Timeout => (Quality::Timeout, "timeout".to_string()),
                        DriverError::Comm { message } => (Quality::CommError, message),
                    };
                    mark_job_failure(
                        &mut results_by_key,
                        job,
                        now,
                        quality,
                        &message,
                        duration_ms,
                    );
                }
            }
        }
    }

    // 输出顺序稳定：按 points 列表顺序输出；缺失则补 ConfigError。
    let mut ordered_results: Vec<SampleResult> = Vec::with_capacity(points.len());
    for point in points {
        let result = results_by_key
            .remove(&point.point_key)
            .unwrap_or_else(|| SampleResult {
                point_key: point.point_key,
                value_display: "".to_string(),
                quality: Quality::ConfigError,
                timestamp: now,
                duration_ms: 0,
                error_message: "missing result".to_string(),
            });
        ordered_results.push(result);
    }

    let stats = calc_stats(&ordered_results);
    Some((ordered_results, stats))
}

fn build_profile_map<'a>(
    profiles: &'a [ConnectionProfile],
) -> HashMap<&'a str, &'a ConnectionProfile> {
    let mut map: HashMap<&'a str, &'a ConnectionProfile> = HashMap::new();
    for profile in profiles {
        let channel_name = match profile {
            ConnectionProfile::Tcp { channel_name, .. } => channel_name.as_str(),
            ConnectionProfile::Rtu485 { channel_name, .. } => channel_name.as_str(),
        };
        map.insert(channel_name, profile);
    }
    map
}

fn profile_timeout_ms(profile: &ConnectionProfile) -> u32 {
    match profile {
        ConnectionProfile::Tcp { timeout_ms, .. } => *timeout_ms,
        ConnectionProfile::Rtu485 { timeout_ms, .. } => *timeout_ms,
    }
}

fn profile_retry_count(profile: &ConnectionProfile) -> u32 {
    match profile {
        ConnectionProfile::Tcp { retry_count, .. } => *retry_count,
        ConnectionProfile::Rtu485 { retry_count, .. } => *retry_count,
    }
}

async fn read_with_retry(
    driver: &dyn CommDriver,
    conn_mgr: &mut ConnectionManager,
    stop_rx: &watch::Receiver<bool>,
    key: &ConnectionKey,
    profile: &ConnectionProfile,
    job: &ReadJob,
    reconnected_keys: &mut HashSet<ConnectionKey>,
) -> Option<(Result<RawReadData, DriverError>, u32)> {
    let timeout = Duration::from_millis(profile_timeout_ms(profile) as u64);
    let max_retries = profile_retry_count(profile);

    let mut attempt: u32 = 0;
    loop {
        if *stop_rx.borrow() {
            return None;
        }

        let started = Instant::now();

        if conn_mgr.get_mut(key).is_none() {
            let connect = conn_mgr
                .ensure_connected(driver, profile, stop_rx, timeout)
                .await;
            if let Err(e) = connect {
                let duration_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
                if *stop_rx.borrow() {
                    return None;
                }
                return Some((Err(e), duration_ms));
            }
        }

        let Some(client) = conn_mgr.get_mut(key) else {
            let duration_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
            return Some((
                Err(DriverError::Comm {
                    message: "missing connected client".to_string(),
                }),
                duration_ms,
            ));
        };

        let read_fut = driver.read_with_client(client, job);
        let stop_fut = wait_stop(stop_rx.clone());
        let result = tokio::select! {
            _ = stop_fut => {
                return None;
            }
            res = tokio::time::timeout(timeout, read_fut) => {
                match res {
                    Ok(inner) => inner,
                    Err(_) => Err(DriverError::Timeout),
                }
            }
        };

        let duration_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;

        match &result {
            Ok(_) => return Some((result, duration_ms)),
            Err(DriverError::Timeout) | Err(DriverError::Comm { .. }) => {
                // Per-connection, per-poll: at most one reconnect. Subsequent failures
                // keep the connection (to avoid reconnect storms) but invalidate on final failure.
                if !reconnected_keys.contains(key) {
                    conn_mgr.invalidate(key, "read failed; will reconnect once");
                    reconnected_keys.insert(key.clone());
                    continue;
                }

                if attempt >= max_retries {
                    conn_mgr.invalidate(key, "read failed after reconnect/retries");
                    return Some((result, duration_ms));
                }

                attempt += 1;
                let stop_sleep = wait_stop(stop_rx.clone());
                tokio::select! {
                    _ = stop_sleep => {
                        return None;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                }
            }
        }
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

fn decode_job_data(
    results_by_key: &mut HashMap<Uuid, SampleResult>,
    job: &ReadJob,
    data: RawReadData,
    timestamp: DateTime<Utc>,
    duration_ms: u32,
) {
    match data {
        RawReadData::Coils(bits) => {
            for point in &job.points {
                let bit_slice = bits.get(point.offset as usize..).unwrap_or(&[]);
                let decoded = decode_from_bits(point.data_type.clone(), bit_slice);
                insert_decoded_result(results_by_key, point, decoded, timestamp, duration_ms);
            }
        }
        RawReadData::Registers(registers) => {
            for point in &job.points {
                let reg_slice = registers.get(point.offset as usize..).unwrap_or(&[]);
                let decoded = decode_from_registers(
                    point.data_type.clone(),
                    point.byte_order.clone(),
                    reg_slice,
                );
                insert_decoded_result(results_by_key, point, decoded, timestamp, duration_ms);
            }
        }
    }
}

fn insert_decoded_result(
    results_by_key: &mut HashMap<Uuid, SampleResult>,
    point: &PlannedPointRead,
    decoded: Result<DecodedValue, DecodeError>,
    timestamp: DateTime<Utc>,
    duration_ms: u32,
) {
    let (quality, value_display, error_message) = match decoded {
        Ok(value) => (
            Quality::Ok,
            value.to_value_display(point.scale),
            "".to_string(),
        ),
        Err(err) => (Quality::DecodeError, "".to_string(), err.to_string()),
    };

    results_by_key.insert(
        point.point_key,
        SampleResult {
            point_key: point.point_key,
            value_display,
            quality,
            timestamp,
            duration_ms,
            error_message,
        },
    );
}

fn mark_job_failure(
    results_by_key: &mut HashMap<Uuid, SampleResult>,
    job: &ReadJob,
    timestamp: DateTime<Utc>,
    quality: Quality,
    message: &str,
    duration_ms: u32,
) {
    for point in &job.points {
        results_by_key.insert(
            point.point_key,
            SampleResult {
                point_key: point.point_key,
                value_display: "".to_string(),
                quality: quality.clone(),
                timestamp,
                duration_ms,
                error_message: message.to_string(),
            },
        );
    }
}

fn calc_stats(results: &[SampleResult]) -> RunStats {
    let mut stats = RunStats {
        total: 0,
        ok: 0,
        timeout: 0,
        comm_error: 0,
        decode_error: 0,
        config_error: 0,
    };

    for result in results {
        stats.total += 1;
        match result.quality {
            Quality::Ok => stats.ok += 1,
            Quality::Timeout => stats.timeout += 1,
            Quality::CommError => stats.comm_error += 1,
            Quality::DecodeError => stats.decode_error += 1,
            Quality::ConfigError => stats.config_error += 1,
        }
    }

    stats
}

#[derive(Clone, Debug)]
struct LatestSnapshot {
    results: Vec<SampleResult>,
    stats: RunStats,
    updated_at_utc: DateTime<Utc>,
    run_warnings: Vec<CommWarning>,
}

struct RunHandle {
    stop_tx: watch::Sender<bool>,
    join: JoinHandle<()>,
    latest: Arc<Mutex<LatestSnapshot>>,
}

/// 后台采集引擎：负责 start/stop/latest/stats。
///
/// 约束（执行要求）：
/// - start 必须 spawn 后台任务，不阻塞调用方
/// - stop 需在 1 秒内生效（MVP 目标）
/// - latest 只读缓存，不触发采集
pub struct CommRunEngine {
    runs: Mutex<HashMap<Uuid, RunHandle>>,
}

impl CommRunEngine {
    pub fn new() -> Self {
        Self {
            runs: Mutex::new(HashMap::new()),
        }
    }

    pub fn start_run(
        &self,
        driver: Arc<dyn CommDriver>,
        profiles: Vec<ConnectionProfile>,
        points: Vec<CommPoint>,
        plan: ReadPlan,
        poll_interval_ms: u32,
    ) -> Uuid {
        let run_id = Uuid::new_v4();
        let (stop_tx, mut stop_rx) = watch::channel(false);

        let now = Utc::now();
        let initial_results: Vec<SampleResult> = points
            .iter()
            .map(|p| SampleResult {
                point_key: p.point_key,
                value_display: "".to_string(),
                quality: Quality::ConfigError,
                timestamp: now,
                duration_ms: 0,
                error_message: "not started".to_string(),
            })
            .collect();
        let initial_stats = calc_stats(&initial_results);
        let latest = Arc::new(Mutex::new(LatestSnapshot {
            results: initial_results,
            stats: initial_stats,
            updated_at_utc: now,
            run_warnings: Vec::new(),
        }));

        let latest_for_task = Arc::clone(&latest);
        let interval = Duration::from_millis(poll_interval_ms as u64);

        let join = tokio::spawn(async move {
            let mut conn_mgr = ConnectionManager::new(run_id);
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    changed = stop_rx.changed() => {
                        if changed.is_err() || *stop_rx.borrow() {
                            break;
                        }
                    }
                    _ = ticker.tick() => {
                        if *stop_rx.borrow() {
                            break;
                        }

                        if let Some((results, stats)) = execute_plan_once_with_manager(
                            driver.as_ref(),
                            &mut conn_mgr,
                            &stop_rx,
                            &profiles,
                            &points,
                            &plan
                        ).await {
                            let updated_at_utc = results.first().map(|r| r.timestamp).unwrap_or_else(Utc::now);
                            let run_warnings = build_run_warnings(&results, &stats);
                            let mut guard = latest_for_task.lock();
                            guard.results = results;
                            guard.stats = stats;
                            guard.updated_at_utc = updated_at_utc;
                            guard.run_warnings = run_warnings;
                        } else if *stop_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });

        self.runs.lock().insert(
            run_id,
            RunHandle {
                stop_tx,
                join,
                latest,
            },
        );

        run_id
    }

    pub fn latest(
        &self,
        run_id: Uuid,
    ) -> Option<(Vec<SampleResult>, RunStats, DateTime<Utc>, Vec<CommWarning>)> {
        let latest = {
            let guard = self.runs.lock();
            guard.get(&run_id).map(|h| Arc::clone(&h.latest))
        }?;

        let snapshot = latest.lock().clone();
        Some((
            snapshot.results,
            snapshot.stats,
            snapshot.updated_at_utc,
            snapshot.run_warnings,
        ))
    }

    pub async fn stop_run(&self, run_id: Uuid) -> bool {
        let handle = self.runs.lock().remove(&run_id);
        let Some(handle) = handle else {
            return false;
        };

        let _ = handle.stop_tx.send(true);
        match tokio::time::timeout(Duration::from_secs(1), handle.join).await {
            Ok(join_result) => join_result.is_ok(),
            Err(_) => false,
        }
    }
}

fn build_run_warnings(results: &[SampleResult], stats: &RunStats) -> Vec<CommWarning> {
    let mut warnings: Vec<CommWarning> = Vec::new();

    if stats.timeout > 0 {
        warnings.push(CommWarning {
            code: "RUN_TIMEOUT".to_string(),
            message: format!("timeout count: {}", stats.timeout),
            point_key: None,
            hmi_name: None,
        });
    }

    if stats.comm_error > 0 {
        warnings.push(CommWarning {
            code: "RUN_COMM_ERROR".to_string(),
            message: format!("comm error count: {}", stats.comm_error),
            point_key: None,
            hmi_name: None,
        });
    }

    if stats.decode_error > 0 {
        warnings.push(CommWarning {
            code: "RUN_DECODE_ERROR".to_string(),
            message: format!("decode error count: {}", stats.decode_error),
            point_key: None,
            hmi_name: None,
        });
    }

    if stats.config_error > 0 {
        warnings.push(CommWarning {
            code: "RUN_CONFIG_ERROR".to_string(),
            message: format!("config error count: {}", stats.config_error),
            point_key: None,
            hmi_name: None,
        });
    }

    if results.is_empty() {
        warnings.push(CommWarning {
            code: "RUN_NO_RESULTS".to_string(),
            message: "engine returned no results".to_string(),
            point_key: None,
            hmi_name: None,
        });
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use crate::comm::adapters::driver::mock::MockDriver;
    use crate::comm::core::model::{ByteOrder32, ConnectionProfile, DataType, RegisterArea};
    use crate::comm::core::plan::{build_read_plan, PlanOptions};

    fn tcp_profile(channel_name: &str) -> ConnectionProfile {
        ConnectionProfile::Tcp {
            channel_name: channel_name.to_string(),
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

    fn point(channel_name: &str, point_key: Uuid) -> CommPoint {
        CommPoint {
            point_key,
            hmi_name: format!("HMI_{point_key}"),
            data_type: DataType::UInt16,
            byte_order: ByteOrder32::ABCD,
            channel_name: channel_name.to_string(),
            address_offset: None,
            scale: 1.0,
        }
    }

    #[tokio::test]
    async fn engine_mock_driver_emits_expected_qualities() {
        let driver = MockDriver::new();
        let profiles = vec![
            tcp_profile("mock-ok"),
            tcp_profile("mock-timeout"),
            tcp_profile("mock-decode"),
        ];
        let points = vec![
            point("mock-ok", Uuid::from_u128(1)),
            point("mock-timeout", Uuid::from_u128(2)),
            point("mock-decode", Uuid::from_u128(3)),
        ];

        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
        let (results, stats) = execute_plan_once(&driver, &profiles, &points, &plan).await;

        assert_eq!(stats.total, 3);
        assert_eq!(stats.ok, 1);
        assert_eq!(stats.timeout, 1);
        assert_eq!(stats.decode_error, 1);

        let by_key: HashMap<Uuid, Quality> =
            results.into_iter().map(|r| (r.point_key, r.quality)).collect();
        assert_eq!(by_key.get(&Uuid::from_u128(1)), Some(&Quality::Ok));
        assert_eq!(by_key.get(&Uuid::from_u128(2)), Some(&Quality::Timeout));
        assert_eq!(by_key.get(&Uuid::from_u128(3)), Some(&Quality::DecodeError));
    }
}
