//! 通讯地址采集并生成模块：执行引擎（engine）。
//!
//! 本文件在 TASK-05 阶段先提供最小“执行一次计划”的能力，用于 mock 验收与后续 TASK-08 的后台 run 演进。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::comm::adapters::driver::{CommDriver, DriverError, RawReadData};
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
    let profiles_by_channel = build_profile_map(profiles);
    let now = Utc::now();

    let mut results_by_key: HashMap<Uuid, SampleResult> = HashMap::new();
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

        let (raw, duration_ms) = read_with_retry(driver, profile, job).await;
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
    (ordered_results, stats)
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
    profile: &ConnectionProfile,
    job: &ReadJob,
) -> (Result<RawReadData, DriverError>, u32) {
    let timeout = Duration::from_millis(profile_timeout_ms(profile) as u64);
    let max_retries = profile_retry_count(profile);

    let mut attempt: u32 = 0;
    loop {
        let started = Instant::now();

        let result = match tokio::time::timeout(timeout, driver.read(profile, job)).await {
            Ok(inner) => inner,
            Err(_) => Err(DriverError::Timeout),
        };

        let duration_ms = started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
        match &result {
            Ok(_) => return (result, duration_ms),
            Err(DriverError::Timeout) | Err(DriverError::Comm { .. }) => {
                if attempt >= max_retries {
                    return (result, duration_ms);
                }

                attempt += 1;
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
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
            loop {
                if *stop_rx.borrow() {
                    break;
                }

                tokio::select! {
                  _ = stop_rx.changed() => {
                      continue;
                  }
                    output = execute_plan_once(driver.as_ref(), &profiles, &points, &plan) => {
                        let (results, stats) = output;
                        let updated_at_utc = results.first().map(|r| r.timestamp).unwrap_or_else(Utc::now);
                        let run_warnings = build_run_warnings(&results, &stats);
                        let mut guard = latest_for_task.lock();
                        guard.results = results;
                        guard.stats = stats;
                        guard.updated_at_utc = updated_at_utc;
                        guard.run_warnings = run_warnings;
                    }
                }

                tokio::select! {
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                    _ = tokio::time::sleep(interval) => {}
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::comm::driver::mock::MockDriver;
    use crate::comm::model::{ByteOrder32, DataType, RegisterArea};
    use crate::comm::plan::{build_read_plan, PlanOptions};

    fn tcp_profile(channel_name: &str) -> ConnectionProfile {
        ConnectionProfile::Tcp {
            channel_name: channel_name.to_string(),
            device_id: 1,
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 10,
            ip: "127.0.0.1".to_string(),
            port: 502,
            timeout_ms: 50,
            retry_count: 0,
            poll_interval_ms: 500,
        }
    }

    fn point(channel_name: &str, point_key: uuid::Uuid) -> CommPoint {
        CommPoint {
            point_key,
            hmi_name: "X".to_string(),
            data_type: DataType::UInt16,
            byte_order: ByteOrder32::ABCD,
            channel_name: channel_name.to_string(),
            address_offset: None,
            scale: 1.0,
        }
    }

    #[tokio::test]
    async fn mock_driver_produces_ok_timeout_and_decode_error_stats() {
        let profiles = vec![
            tcp_profile("tcp-ok"),
            tcp_profile("tcp-timeout"),
            tcp_profile("tcp-decode"),
        ];

        let points = vec![
            point("tcp-ok", uuid::Uuid::from_u128(1)),
            point("tcp-timeout", uuid::Uuid::from_u128(2)),
            point("tcp-decode", uuid::Uuid::from_u128(3)),
        ];

        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
        let driver = MockDriver::new();

        let (results, stats) = execute_plan_once(&driver, &profiles, &points, &plan).await;

        assert_eq!(results.len(), 3);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.ok, 1);
        assert_eq!(stats.timeout, 1);
        assert_eq!(stats.decode_error, 1);
        assert_eq!(stats.comm_error, 0);

        assert_eq!(results[0].quality, Quality::Ok);
        assert_eq!(results[1].quality, Quality::Timeout);
        assert_eq!(results[2].quality, Quality::DecodeError);
    }

    #[tokio::test]
    async fn run_engine_stop_within_1s_and_latest_is_ordered_by_points() {
        let engine = CommRunEngine::new();
        let profiles = vec![tcp_profile("tcp-ok")];
        let points = vec![
            point("tcp-ok", uuid::Uuid::from_u128(100)),
            point("tcp-ok", uuid::Uuid::from_u128(101)),
            point("tcp-ok", uuid::Uuid::from_u128(102)),
        ];
        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
        let driver = Arc::new(MockDriver::new());

        let run_id = engine.start_run(driver, profiles, points.clone(), plan, 50);

        // 等待至少一次采集写入缓存（最多 1s）。
        let mut latest: Option<(Vec<SampleResult>, RunStats, DateTime<Utc>)> = None;
        for _ in 0..20 {
            if let Some((results, stats, updated_at_utc, _run_warnings)) = engine.latest(run_id) {
                if results.len() == points.len() && stats.total == points.len() as u32 {
                    latest = Some((results, stats, updated_at_utc));
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let (results, stats, _updated_at_utc) =
            latest.expect("run should produce at least one tick");
        assert_eq!(stats.total, 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].point_key, points[0].point_key);
        assert_eq!(results[1].point_key, points[1].point_key);
        assert_eq!(results[2].point_key, points[2].point_key);

        let started = Instant::now();
        let stopped = engine.stop_run(run_id).await;
        assert!(stopped);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn run_engine_latest_contains_ok_timeout_and_decode_error_when_using_mock() {
        let engine = CommRunEngine::new();
        let profiles = vec![
            tcp_profile("tcp-ok"),
            tcp_profile("tcp-timeout"),
            tcp_profile("tcp-decode"),
        ];

        let points = vec![
            CommPoint {
                point_key: uuid::Uuid::from_u128(1),
                hmi_name: "OK_U16".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-ok".to_string(),
                address_offset: None,
                scale: 1.0,
            },
            CommPoint {
                point_key: uuid::Uuid::from_u128(2),
                hmi_name: "OK_F32_CDAB".to_string(),
                data_type: DataType::Float32,
                byte_order: ByteOrder32::CDAB,
                channel_name: "tcp-ok".to_string(),
                address_offset: None,
                scale: 0.1,
            },
            CommPoint {
                point_key: uuid::Uuid::from_u128(3),
                hmi_name: "OK_I32_DCBA".to_string(),
                data_type: DataType::Int32,
                byte_order: ByteOrder32::DCBA,
                channel_name: "tcp-ok".to_string(),
                address_offset: None,
                scale: 1.0,
            },
            CommPoint {
                point_key: uuid::Uuid::from_u128(4),
                hmi_name: "TIMEOUT_U16".to_string(),
                data_type: DataType::UInt16,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-timeout".to_string(),
                address_offset: None,
                scale: 1.0,
            },
            CommPoint {
                point_key: uuid::Uuid::from_u128(5),
                hmi_name: "DECODE_U32".to_string(),
                data_type: DataType::UInt32,
                byte_order: ByteOrder32::ABCD,
                channel_name: "tcp-decode".to_string(),
                address_offset: None,
                scale: 1.0,
            },
        ];

        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
        let driver = Arc::new(MockDriver::new());

        let run_id = engine.start_run(driver, profiles, points.clone(), plan, 50);

        let mut snapshot: Option<(Vec<SampleResult>, RunStats, DateTime<Utc>)> = None;
        for _ in 0..40 {
            if let Some((results, stats, updated_at_utc, _run_warnings)) = engine.latest(run_id) {
                if results.len() == points.len() && stats.total == points.len() as u32 {
                    // 等待至少一次真实采集（不是初始 ConfigError）。
                    if results.iter().any(|r| r.quality != Quality::ConfigError) {
                        snapshot = Some((results, stats, updated_at_utc));
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        let (results, stats, updated_at_utc) =
            snapshot.expect("run should produce at least one tick");

        // 用于 TASK-12 文档验收的可读日志（默认被 cargo test 捕获；可用 -- --nocapture 查看）。
        println!("runId={run_id} updatedAtUtc={updated_at_utc} stats={stats:?}");
        for (i, r) in results.iter().enumerate() {
            println!(
                "row[{i}] pointKey={} quality={:?} valueDisplay='{}' errorMessage='{}' durationMs={}",
                r.point_key, r.quality, r.value_display, r.error_message, r.duration_ms
            );
        }

        assert_eq!(stats.total, 5);
        assert!(results.iter().any(|r| r.quality == Quality::Ok));
        assert!(results.iter().any(|r| r.quality == Quality::Timeout));
        assert!(results.iter().any(|r| r.quality == Quality::DecodeError));

        let stopped = engine.stop_run(run_id).await;
        assert!(stopped);
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
