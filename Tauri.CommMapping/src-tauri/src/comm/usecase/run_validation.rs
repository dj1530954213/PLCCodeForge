//! 通讯采集模块：run_start 前的配置校验（纯校验，不做 IO/通讯）。

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::comm::core::model::{
    ByteOrder32, CommDeviceV1, CommPoint, ConnectionProfile, DataType, RegisterArea,
};
use crate::comm::error::CommMissingField;

fn profile_channel_name(profile: &ConnectionProfile) -> &str {
    match profile {
        ConnectionProfile::Tcp { channel_name, .. } => channel_name,
        ConnectionProfile::Rtu485 { channel_name, .. } => channel_name,
    }
}

#[derive(Clone, Debug)]
struct ProfileInfo {
    read_area: RegisterArea,
    start_address: u16,
    length: u16,
}

fn profile_info(profile: &ConnectionProfile) -> ProfileInfo {
    match profile {
        ConnectionProfile::Tcp {
            read_area,
            start_address,
            length,
            ..
        } => ProfileInfo {
            read_area: read_area.clone(),
            start_address: *start_address,
            length: *length,
        },
        ConnectionProfile::Rtu485 {
            read_area,
            start_address,
            length,
            ..
        } => ProfileInfo {
            read_area: read_area.clone(),
            start_address: *start_address,
            length: *length,
        },
    }
}

fn point_unit_length(read_area: RegisterArea, data_type: &DataType) -> Option<u16> {
    match (read_area, data_type) {
        (RegisterArea::Coil | RegisterArea::Discrete, DataType::Bool) => Some(1),
        (RegisterArea::Holding | RegisterArea::Input, DataType::Int16 | DataType::UInt16) => {
            Some(1)
        }
        (
            RegisterArea::Holding | RegisterArea::Input,
            DataType::Int32 | DataType::UInt32 | DataType::Float32,
        ) => Some(2),
        (
            RegisterArea::Holding | RegisterArea::Input,
            DataType::Int64 | DataType::UInt64 | DataType::Float64,
        ) => Some(4),
        _ => None,
    }
}

fn push_point_error(out: &mut Vec<CommMissingField>, point: &CommPoint, field: &str, reason: &str) {
    out.push(CommMissingField {
        point_key: Some(point.point_key.to_string()),
        hmi_name: if point.hmi_name.trim().is_empty() {
            None
        } else {
            Some(point.hmi_name.clone())
        },
        field: field.to_string(),
        reason: Some(reason.to_string()),
    });
}

#[derive(Clone, Debug)]
struct AddressSegment {
    point_key: Uuid,
    hmi_name: String,
    start: u32,
    end: u32,
}

fn overlaps(a: &AddressSegment, b: &AddressSegment) -> bool {
    a.start < b.end && b.start < a.end
}

fn validate_channel_addresses(
    profile: &ProfileInfo,
    points: &[&CommPoint],
) -> Vec<CommMissingField> {
    let mut out: Vec<CommMissingField> = Vec::new();

    let channel_start = profile.start_address as u32;
    let channel_end = channel_start + profile.length as u32;

    let mut explicit_segments: Vec<AddressSegment> = Vec::new();
    let mut explicit_points: HashMap<Uuid, &CommPoint> = HashMap::new();

    for point in points {
        let Some(offset) = point.address_offset else {
            continue;
        };

        let Some(unit_len) = point_unit_length(profile.read_area.clone(), &point.data_type) else {
            push_point_error(
                &mut out,
                point,
                "dataType",
                "数据类型与读取区域不匹配",
            );
            continue;
        };

        let start = channel_start + u32::from(offset);
        let end = start.saturating_add(unit_len as u32);

        if start < channel_start || end > channel_end {
            push_point_error(&mut out, point, "modbusAddress", "地址超出连接范围");
            continue;
        }

        explicit_segments.push(AddressSegment {
            point_key: point.point_key,
            hmi_name: point.hmi_name.clone(),
            start,
            end,
        });
        explicit_points.insert(point.point_key, point);
    }

    let mut conflict_keys: HashSet<Uuid> = HashSet::new();
    for i in 0..explicit_segments.len() {
        for j in (i + 1)..explicit_segments.len() {
            if overlaps(&explicit_segments[i], &explicit_segments[j]) {
                conflict_keys.insert(explicit_segments[i].point_key);
                conflict_keys.insert(explicit_segments[j].point_key);
            }
        }
    }

    for key in conflict_keys {
        if let Some(point) = explicit_points.get(&key) {
            push_point_error(&mut out, point, "modbusAddress", "地址冲突");
        }
    }

    let mut occupied: Vec<AddressSegment> = explicit_segments;
    let mut cursor = channel_start;

    for point in points {
        if point.address_offset.is_some() {
            continue;
        }

        let Some(unit_len) = point_unit_length(profile.read_area.clone(), &point.data_type) else {
            push_point_error(
                &mut out,
                point,
                "dataType",
                "数据类型与读取区域不匹配",
            );
            continue;
        };

        let mut candidate = cursor;
        loop {
            let seg = AddressSegment {
                point_key: point.point_key,
                hmi_name: point.hmi_name.clone(),
                start: candidate,
                end: candidate.saturating_add(unit_len as u32),
            };

            if seg.end > channel_end {
                push_point_error(&mut out, point, "modbusAddress", "地址超出连接范围");
                break;
            }

            if let Some(overlap) = occupied.iter().find(|other| overlaps(other, &seg)) {
                candidate = overlap.end;
                continue;
            }

            occupied.push(seg);
            cursor = candidate.saturating_add(unit_len as u32);
            break;
        }
    }

    out
}

pub fn validate_hmi_uniqueness_points(points: &[CommPoint]) -> Vec<CommMissingField> {
    let mut out: Vec<CommMissingField> = Vec::new();
    let mut seen: HashMap<String, Uuid> = HashMap::new();
    let mut flagged: HashSet<Uuid> = HashSet::new();

    for point in points {
        let name = point.hmi_name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(prev_key) = seen.get(name).copied() {
            if flagged.insert(point.point_key) {
                push_point_error(&mut out, point, "hmiName", "重名");
            }
            if flagged.insert(prev_key) {
                out.push(CommMissingField {
                    point_key: Some(prev_key.to_string()),
                    hmi_name: Some(name.to_string()),
                    field: "hmiName".to_string(),
                    reason: Some("重名".to_string()),
                });
            }
        } else {
            seen.insert(name.to_string(), point.point_key);
        }
    }

    out
}

pub fn validate_global_hmi_uniqueness(devices: &[CommDeviceV1]) -> Vec<CommMissingField> {
    let mut out: Vec<CommMissingField> = Vec::new();
    let mut seen: HashMap<String, (Uuid, String)> = HashMap::new();
    let mut flagged: HashSet<Uuid> = HashSet::new();

    for device in devices {
        let device_label = format!(
            "设备Id={} 设备名={}",
            device.device_id, device.device_name
        );
        for point in &device.points.points {
            let name = point.hmi_name.trim();
            if name.is_empty() {
                continue;
            }
            if let Some((prev_key, prev_device)) = seen.get(name).cloned() {
                if flagged.insert(point.point_key) {
                    push_point_error(
                        &mut out,
                        point,
                        "hmiName",
                        &format!("与其他设备（{prev_device}）重名"),
                    );
                }
                if flagged.insert(prev_key) {
                    out.push(CommMissingField {
                        point_key: Some(prev_key.to_string()),
                        hmi_name: Some(name.to_string()),
                        field: "hmiName".to_string(),
                        reason: Some(format!("与其他设备（{device_label}）重名")),
                    });
                }
            } else {
                seen.insert(name.to_string(), (point.point_key, device_label.clone()));
            }
        }
    }

    out
}

pub fn validate_run_inputs(
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
) -> Vec<CommMissingField> {
    let mut out: Vec<CommMissingField> = Vec::new();

    let mut profiles_by_channel: HashMap<String, ProfileInfo> = HashMap::new();
    for profile in profiles {
        let channel_name = profile_channel_name(profile).trim();
        if channel_name.is_empty() {
            out.push(CommMissingField {
                point_key: None,
                hmi_name: None,
                field: "profiles.channelName".to_string(),
                reason: Some("不能为空".to_string()),
            });
            continue;
        }
        if profiles_by_channel.contains_key(channel_name) {
            out.push(CommMissingField {
                point_key: None,
                hmi_name: None,
                field: "profiles.channelName".to_string(),
                reason: Some(format!("通道名称重复: {channel_name}")),
            });
            continue;
        }
        profiles_by_channel.insert(channel_name.to_string(), profile_info(profile));
    }

    let mut seen_point_keys: HashSet<Uuid> = HashSet::new();
    for point in points {
        if !seen_point_keys.insert(point.point_key) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "pointKey".to_string(),
                reason: Some("重复".to_string()),
            });
        }

        if point.hmi_name.trim().is_empty() {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: None,
                field: "hmiName".to_string(),
                reason: Some("不能为空".to_string()),
            });
        }

        let channel_name = point.channel_name.trim();
        if channel_name.is_empty() {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "channelName".to_string(),
                reason: Some("不能为空".to_string()),
            });
        } else if !profiles_by_channel.contains_key(channel_name) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "channelName".to_string(),
                reason: Some(format!("未知通道名称: {channel_name}")),
            });
        }

        if matches!(point.data_type, DataType::Unknown) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "dataType".to_string(),
                reason: Some("未知".to_string()),
            });
        }

        if matches!(point.byte_order, ByteOrder32::Unknown) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "byteOrder".to_string(),
                reason: Some("未知".to_string()),
            });
        }

        if !point.scale.is_finite() {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "scale".to_string(),
                reason: Some("不是有效数字".to_string()),
            });
        }
    }

    let mut points_by_channel: HashMap<&str, Vec<&CommPoint>> = HashMap::new();
    for point in points {
        let channel_name = point.channel_name.trim();
        if channel_name.is_empty() {
            continue;
        }
        if !profiles_by_channel.contains_key(channel_name) {
            continue;
        }
        points_by_channel
            .entry(channel_name)
            .or_default()
            .push(point);
    }

    for (channel_name, channel_points) in points_by_channel {
        if let Some(profile) = profiles_by_channel.get(channel_name) {
            out.extend(validate_channel_addresses(profile, &channel_points));
        }
    }

    out
}
