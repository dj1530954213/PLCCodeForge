//! 通讯采集模块：run_start 前的配置校验（纯校验，不做 IO/通讯）。

use std::collections::HashSet;

use uuid::Uuid;

use crate::comm::core::model::{ByteOrder32, CommPoint, ConnectionProfile, DataType};
use crate::comm::error::CommMissingField;

fn profile_channel_name(profile: &ConnectionProfile) -> &str {
    match profile {
        ConnectionProfile::Tcp { channel_name, .. } => channel_name,
        ConnectionProfile::Rtu485 { channel_name, .. } => channel_name,
    }
}

pub fn validate_run_inputs(
    profiles: &[ConnectionProfile],
    points: &[CommPoint],
) -> Vec<CommMissingField> {
    let mut out: Vec<CommMissingField> = Vec::new();

    let mut seen_channels: HashSet<String> = HashSet::new();
    for profile in profiles {
        let channel_name = profile_channel_name(profile).trim();
        if channel_name.is_empty() {
            out.push(CommMissingField {
                point_key: None,
                hmi_name: None,
                field: "profiles.channelName".to_string(),
                reason: Some("empty".to_string()),
            });
            continue;
        }
        if !seen_channels.insert(channel_name.to_string()) {
            out.push(CommMissingField {
                point_key: None,
                hmi_name: None,
                field: "profiles.channelName".to_string(),
                reason: Some(format!("duplicate channelName: {channel_name}")),
            });
        }
    }

    let mut seen_point_keys: HashSet<Uuid> = HashSet::new();
    for point in points {
        if !seen_point_keys.insert(point.point_key) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "pointKey".to_string(),
                reason: Some("duplicate".to_string()),
            });
        }

        if point.hmi_name.trim().is_empty() {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: None,
                field: "hmiName".to_string(),
                reason: Some("empty".to_string()),
            });
        }

        let channel_name = point.channel_name.trim();
        if channel_name.is_empty() {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "channelName".to_string(),
                reason: Some("empty".to_string()),
            });
        } else if !seen_channels.contains(channel_name) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "channelName".to_string(),
                reason: Some(format!("unknown channelName: {channel_name}")),
            });
        }

        if matches!(point.data_type, DataType::Unknown) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "dataType".to_string(),
                reason: Some("Unknown".to_string()),
            });
        }

        if matches!(point.byte_order, ByteOrder32::Unknown) {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "byteOrder".to_string(),
                reason: Some("Unknown".to_string()),
            });
        }

        if !point.scale.is_finite() {
            out.push(CommMissingField {
                point_key: Some(point.point_key.to_string()),
                hmi_name: Some(point.hmi_name.clone()),
                field: "scale".to_string(),
                reason: Some("not finite".to_string()),
            });
        }
    }

    out
}
