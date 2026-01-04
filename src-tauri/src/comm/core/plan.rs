//! 通讯地址采集并生成模块：批量读取计划（plan）。
//!
//! 目标（TASK-04）：
//! - 按 `channelName` 分组
//! - 在每个 channel 内按 points 顺序做“地址自动映射”
//! - 对连续地址做聚合，并按最大读长度分批
//! - 输出顺序稳定（按 points 顺序 + pointKey tie-break）

use super::model::{ByteOrder32, CommPoint, ConnectionProfile, DataType, RegisterArea};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanOptions {
    pub max_registers_per_job: u16,
    pub max_coils_per_job: u16,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            // Modbus 常见上限：125 registers / 2000 coils；MVP 保守取值，可后续配置化。
            max_registers_per_job: 120,
            max_coils_per_job: 2000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadPlan {
    pub jobs: Vec<ReadJob>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadJob {
    pub channel_name: String,
    pub read_area: RegisterArea,
    /// 内部 0-based
    pub start_address: u16,
    /// 寄存器/线圈数量
    pub length: u16,
    pub points: Vec<PlannedPointRead>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedPointRead {
    pub point_key: Uuid,
    pub data_type: DataType,
    pub byte_order: ByteOrder32,
    pub scale: f64,
    /// 相对 `ReadJob.startAddress` 的偏移（寄存器/线圈）
    pub offset: u16,
    /// 点位占用的寄存器/线圈数量
    pub length: u16,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanError {
    #[error("duplicate channelName in profiles: {channel_name}")]
    DuplicateChannelName { channel_name: String },

    #[error("missing connection profile for channelName: {channel_name}")]
    MissingProfile { channel_name: String },

    #[error("channelName={channel_name} has mismatched readArea={read_area:?} for dataType={data_type:?}")]
    AreaDataTypeMismatch {
        channel_name: String,
        read_area: RegisterArea,
        data_type: DataType,
    },

    #[error("channelName={channel_name} exceeds configured address range (startAddress={start_address}, length={length})")]
    ExceedsChannelRange {
        channel_name: String,
        start_address: u16,
        length: u16,
    },

    #[error("point requires {point_length} units but max per job is {max_per_job}")]
    PointTooLargeForJob { point_length: u16, max_per_job: u16 },
}

pub fn build_read_plan(
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
    options: PlanOptions,
) -> Result<ReadPlan, PlanError> {
    let profile_index = build_profile_index(profiles)?;

    let mut grouped: std::collections::HashMap<String, ChannelPoints> =
        std::collections::HashMap::new();

    for (index, point) in points.iter().enumerate() {
        let profile = profile_index
            .get(point.channel_name.as_str())
            .ok_or_else(|| PlanError::MissingProfile {
                channel_name: point.channel_name.clone(),
            })?;

        let channel_entry = grouped
            .entry(point.channel_name.clone())
            .or_insert_with(|| ChannelPoints {
                first_index: index,
                points: Vec::new(),
            });

        channel_entry.first_index = channel_entry.first_index.min(index);
        channel_entry.points.push(PointSeed {
            index,
            point_key: point.point_key,
            data_type: point.data_type.clone(),
            byte_order: point.byte_order.clone(),
            scale: point.scale,
            address_offset: point.address_offset,
            unit_length: point_unit_length(
                &point.channel_name,
                profile.read_area.clone(),
                &point.data_type,
            )?,
        });
    }

    let mut channels: Vec<(String, ChannelPoints)> = grouped.into_iter().collect();
    channels.sort_by(|(name_a, a), (name_b, b)| {
        a.first_index
            .cmp(&b.first_index)
            .then_with(|| name_a.cmp(name_b))
    });

    let mut jobs: Vec<ReadJob> = Vec::new();
    for (channel_name, mut channel_points) in channels {
        let profile =
            profile_index
                .get(channel_name.as_str())
                .ok_or_else(|| PlanError::MissingProfile {
                    channel_name: channel_name.clone(),
                })?;

        channel_points.sort_points();
        jobs.extend(build_jobs_for_channel(
            &channel_name,
            profile,
            &channel_points.points,
            &options,
        )?);
    }

    Ok(ReadPlan { jobs })
}

#[derive(Clone, Debug)]
struct ChannelPoints {
    first_index: usize,
    points: Vec<PointSeed>,
}

impl ChannelPoints {
    fn sort_points(&mut self) {
        self.points.sort_by(|a, b| {
            a.index
                .cmp(&b.index)
                .then_with(|| a.point_key.cmp(&b.point_key))
        });
    }
}

#[derive(Clone, Debug)]
struct PointSeed {
    index: usize,
    point_key: Uuid,
    data_type: DataType,
    byte_order: ByteOrder32,
    scale: f64,
    address_offset: Option<u16>,
    unit_length: u16,
}

#[derive(Clone, Debug)]
struct ProfileInfo {
    channel_name: String,
    read_area: RegisterArea,
    start_address: u16,
    total_length: u16,
}

impl ConnectionProfile {
    fn channel_name(&self) -> &str {
        match self {
            ConnectionProfile::Tcp { channel_name, .. } => channel_name,
            ConnectionProfile::Rtu485 { channel_name, .. } => channel_name,
        }
    }

    fn read_area(&self) -> RegisterArea {
        match self {
            ConnectionProfile::Tcp { read_area, .. } => read_area.clone(),
            ConnectionProfile::Rtu485 { read_area, .. } => read_area.clone(),
        }
    }

    fn start_address(&self) -> u16 {
        match self {
            ConnectionProfile::Tcp { start_address, .. } => *start_address,
            ConnectionProfile::Rtu485 { start_address, .. } => *start_address,
        }
    }

    fn total_length(&self) -> u16 {
        match self {
            ConnectionProfile::Tcp { length, .. } => *length,
            ConnectionProfile::Rtu485 { length, .. } => *length,
        }
    }
}

fn build_profile_index<'a>(
    profiles: &'a [ConnectionProfile],
) -> Result<std::collections::HashMap<&'a str, ProfileInfo>, PlanError> {
    let mut index: std::collections::HashMap<&'a str, ProfileInfo> =
        std::collections::HashMap::new();

    for profile in profiles {
        let name = profile.channel_name();
        if index.contains_key(name) {
            return Err(PlanError::DuplicateChannelName {
                channel_name: name.to_string(),
            });
        }

        index.insert(
            name,
            ProfileInfo {
                channel_name: name.to_string(),
                read_area: profile.read_area(),
                start_address: profile.start_address(),
                total_length: profile.total_length(),
            },
        );
    }

    Ok(index)
}

fn point_unit_length(
    channel_name: &str,
    read_area: RegisterArea,
    data_type: &DataType,
) -> Result<u16, PlanError> {
    let length = match (read_area, data_type) {
        (RegisterArea::Coil | RegisterArea::Discrete, DataType::Bool) => 1,
        (RegisterArea::Holding | RegisterArea::Input, DataType::Int16 | DataType::UInt16) => 1,
        (
            RegisterArea::Holding | RegisterArea::Input,
            DataType::Int32 | DataType::UInt32 | DataType::Float32,
        ) => 2,
        (area, dt) => {
            return Err(PlanError::AreaDataTypeMismatch {
                channel_name: channel_name.to_string(),
                read_area: area,
                data_type: dt.clone(),
            });
        }
    };

    Ok(length)
}

fn max_length_per_job(profile: &ProfileInfo, options: &PlanOptions) -> u16 {
    match profile.read_area {
        RegisterArea::Coil | RegisterArea::Discrete => options.max_coils_per_job,
        RegisterArea::Holding | RegisterArea::Input => options.max_registers_per_job,
    }
}

fn build_jobs_for_channel(
    channel_name: &str,
    profile: &ProfileInfo,
    points: &[PointSeed],
    options: &PlanOptions,
) -> Result<Vec<ReadJob>, PlanError> {
    // 地址语义（冻结约束）：
    // - profile：描述 area/start/len（内部 0-based baseStart）
    // - point：可选提供 addressOffset（相对 profile.baseStart 的偏移），用于精确对齐真实地址
    // - 若 point.addressOffset 缺省，则保持兼容旧行为：按 points 顺序从 profile.baseStart 自动顺排
    // - plan：将上述信息汇总为 ReadJob.startAddress/length（内部 0-based）
    let max_per_job = max_length_per_job(profile, options);

    #[derive(Clone, Copy, Debug)]
    struct Segment {
        start: u32,
        end: u32,
    }

    fn overlaps(a: Segment, b: Segment) -> bool {
        a.start < b.end && b.start < a.end
    }

    let channel_start: u32 = profile.start_address as u32;
    let channel_end: u32 = profile.start_address as u32 + profile.total_length as u32;

    // 先收集显式 offset 的占用段，保证 auto 顺排不会与其冲突。
    let mut occupied: Vec<Segment> = Vec::with_capacity(points.len());
    for point in points {
        let len = point.unit_length as u32;
        if len as u16 > max_per_job {
            return Err(PlanError::PointTooLargeForJob {
                point_length: len as u16,
                max_per_job,
            });
        }

        let Some(offset) = point.address_offset else {
            continue;
        };

        let start = channel_start + u32::from(offset);
        let seg = Segment {
            start,
            end: start.saturating_add(len),
        };

        if seg.end > channel_end {
            return Err(PlanError::ExceedsChannelRange {
                channel_name: channel_name.to_string(),
                start_address: profile.start_address,
                length: profile.total_length,
            });
        }

        if occupied.iter().copied().any(|other| overlaps(seg, other)) {
            return Err(PlanError::ExceedsChannelRange {
                channel_name: channel_name.to_string(),
                start_address: profile.start_address,
                length: profile.total_length,
            });
        }

        occupied.push(seg);
    }

    let mut planned: Vec<(u16, PointSeed)> = Vec::with_capacity(points.len());
    let mut cursor: u32 = channel_start;

    for point in points {
        let len = point.unit_length as u32;

        let start: u32 = match point.address_offset {
            Some(offset) => channel_start + u32::from(offset),
            None => {
                let mut candidate = cursor;
                loop {
                    let seg = Segment {
                        start: candidate,
                        end: candidate.saturating_add(len),
                    };

                    if seg.end > channel_end {
                        return Err(PlanError::ExceedsChannelRange {
                            channel_name: channel_name.to_string(),
                            start_address: profile.start_address,
                            length: profile.total_length,
                        });
                    }

                    if let Some(overlap) =
                        occupied.iter().copied().find(|other| overlaps(seg, *other))
                    {
                        candidate = overlap.end;
                        continue;
                    }

                    occupied.push(seg);
                    cursor = seg.end;
                    break seg.start;
                }
            }
        };

        planned.push((start as u16, point.clone()));
    }

    // 显式 offset 可能导致地址不按 points 顺序递增；为确保聚合/分批正确，这里按地址做确定性排序。
    planned.sort_by(|(addr_a, a), (addr_b, b)| {
        addr_a
            .cmp(addr_b)
            .then_with(|| a.index.cmp(&b.index))
            .then_with(|| a.point_key.cmp(&b.point_key))
    });

    let mut jobs: Vec<ReadJob> = Vec::new();
    let mut current_job_start: Option<u16> = None;
    let mut current_job_len: u16 = 0;
    let mut current_points: Vec<(u16, PointSeed)> = Vec::new();

    for (address, point) in planned {
        match current_job_start {
            None => {
                current_job_start = Some(address);
                current_job_len = point.unit_length;
                current_points.push((address, point));
            }
            Some(job_start) => {
                let expected_next = job_start + current_job_len;
                let can_merge_contiguous = address == expected_next
                    && current_job_len.saturating_add(point.unit_length) <= max_per_job;

                if can_merge_contiguous {
                    current_job_len += point.unit_length;
                    current_points.push((address, point));
                } else {
                    jobs.push(finalize_job(
                        channel_name,
                        profile.read_area.clone(),
                        job_start,
                        current_job_len,
                        &current_points,
                    ));

                    current_job_start = Some(address);
                    current_job_len = point.unit_length;
                    current_points = vec![(address, point)];
                }
            }
        }
    }

    if let Some(job_start) = current_job_start {
        jobs.push(finalize_job(
            channel_name,
            profile.read_area.clone(),
            job_start,
            current_job_len,
            &current_points,
        ));
    }

    Ok(jobs)
}

fn finalize_job(
    channel_name: &str,
    read_area: RegisterArea,
    start_address: u16,
    length: u16,
    points: &[(u16, PointSeed)],
) -> ReadJob {
    let planned_points = points
        .iter()
        .map(|(address, point)| PlannedPointRead {
            point_key: point.point_key,
            data_type: point.data_type.clone(),
            byte_order: point.byte_order.clone(),
            scale: point.scale,
            offset: address.saturating_sub(start_address),
            length: point.unit_length,
        })
        .collect();

    ReadJob {
        channel_name: channel_name.to_string(),
        read_area,
        start_address,
        length,
        points: planned_points,
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{ByteOrder32, ConnectionProfile, DataType, RegisterArea};
    use super::*;

    fn tcp_profile(channel_name: &str) -> ConnectionProfile {
        ConnectionProfile::Tcp {
            channel_name: channel_name.to_string(),
            device_id: 1,
            read_area: RegisterArea::Holding,
            start_address: 0,
            length: 100,
            ip: "127.0.0.1".to_string(),
            port: 502,
            timeout_ms: 1000,
            retry_count: 1,
            poll_interval_ms: 500,
        }
    }

    fn tcp_profile_with_base_start(
        channel_name: &str,
        start_address: u16,
        length: u16,
    ) -> ConnectionProfile {
        ConnectionProfile::Tcp {
            channel_name: channel_name.to_string(),
            device_id: 1,
            read_area: RegisterArea::Holding,
            start_address,
            length,
            ip: "127.0.0.1".to_string(),
            port: 502,
            timeout_ms: 1000,
            retry_count: 1,
            poll_interval_ms: 500,
        }
    }

    fn point(channel_name: &str, data_type: DataType, point_key: Uuid) -> CommPoint {
        CommPoint {
            point_key,
            hmi_name: "X".to_string(),
            data_type,
            byte_order: ByteOrder32::ABCD,
            channel_name: channel_name.to_string(),
            address_offset: None,
            scale: 1.0,
        }
    }

    fn point_with_offset(
        channel_name: &str,
        data_type: DataType,
        point_key: Uuid,
        offset: u16,
    ) -> CommPoint {
        CommPoint {
            point_key,
            hmi_name: "X".to_string(),
            data_type,
            byte_order: ByteOrder32::ABCD,
            channel_name: channel_name.to_string(),
            address_offset: Some(offset),
            scale: 1.0,
        }
    }

    #[test]
    fn plan_aggregates_contiguous_and_splits_by_max_registers() {
        let profiles = vec![tcp_profile("tcp-1")];
        let points = vec![
            point("tcp-1", DataType::Int16, Uuid::nil()),
            point("tcp-1", DataType::UInt16, Uuid::from_u128(1)),
            point("tcp-1", DataType::Int32, Uuid::from_u128(2)),
        ];

        let plan = build_read_plan(
            &profiles,
            &points,
            PlanOptions {
                max_registers_per_job: 2,
                ..PlanOptions::default()
            },
        )
        .unwrap();

        assert_eq!(plan.jobs.len(), 2);
        assert_eq!(plan.jobs[0].start_address, 0);
        assert_eq!(plan.jobs[0].length, 2);
        assert_eq!(plan.jobs[0].points.len(), 2);
        assert_eq!(plan.jobs[0].points[0].offset, 0);
        assert_eq!(plan.jobs[0].points[1].offset, 1);

        assert_eq!(plan.jobs[1].start_address, 2);
        assert_eq!(plan.jobs[1].length, 2);
        assert_eq!(plan.jobs[1].points.len(), 1);
        assert_eq!(plan.jobs[1].points[0].offset, 0);
        assert_eq!(plan.jobs[1].points[0].length, 2);
    }

    #[test]
    fn plan_output_order_is_stable_by_first_point_index() {
        let profiles = vec![tcp_profile("A"), tcp_profile("B")];
        let points = vec![
            point("B", DataType::Int16, Uuid::from_u128(10)),
            point("A", DataType::Int16, Uuid::from_u128(11)),
            point("B", DataType::Int16, Uuid::from_u128(12)),
        ];

        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();

        assert_eq!(plan.jobs.len(), 2);
        assert_eq!(plan.jobs[0].channel_name, "B");
        assert_eq!(plan.jobs[1].channel_name, "A");
        assert_eq!(plan.jobs[0].points.len(), 2);
        assert_eq!(plan.jobs[0].points[0].point_key, Uuid::from_u128(10));
        assert_eq!(plan.jobs[0].points[1].point_key, Uuid::from_u128(12));
    }

    #[test]
    fn plan_respects_profile_base_start_for_auto_mapping_when_base_start_is_not_zero() {
        let profiles = vec![tcp_profile_with_base_start("tcp-1", 100, 10)];
        let points = vec![
            point("tcp-1", DataType::UInt16, Uuid::from_u128(1)),
            point("tcp-1", DataType::UInt16, Uuid::from_u128(2)),
        ];

        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
        assert_eq!(plan.jobs.len(), 1);
        assert_eq!(plan.jobs[0].start_address, 100);
        assert_eq!(plan.jobs[0].length, 2);
        assert_eq!(plan.jobs[0].points[0].offset, 0);
        assert_eq!(plan.jobs[0].points[1].offset, 1);
    }

    #[test]
    fn plan_uses_point_address_offset_relative_to_profile_base_start() {
        let profiles = vec![tcp_profile_with_base_start("tcp-1", 100, 20)];
        let points = vec![
            point_with_offset("tcp-1", DataType::UInt16, Uuid::from_u128(1), 5),
            point_with_offset("tcp-1", DataType::UInt16, Uuid::from_u128(2), 6),
        ];

        let plan = build_read_plan(&profiles, &points, PlanOptions::default()).unwrap();
        assert_eq!(plan.jobs.len(), 1);
        assert_eq!(plan.jobs[0].start_address, 105);
        assert_eq!(plan.jobs[0].length, 2);
        assert_eq!(plan.jobs[0].points[0].offset, 0);
        assert_eq!(plan.jobs[0].points[1].offset, 1);
    }
}
