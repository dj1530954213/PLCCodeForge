use std::env;

use tauri_app_lib::comm::driver::modbus_rtu::ModbusRtuDriver;
use tauri_app_lib::comm::driver::modbus_tcp::ModbusTcpDriver;
use tauri_app_lib::comm::driver::CommDriver;
use tauri_app_lib::comm::engine::execute_plan_once;
use tauri_app_lib::comm::model::{
    ByteOrder32, CommPoint, ConnectionProfile, DataType, Quality, RegisterArea, SerialParity,
};
use tauri_app_lib::comm::plan::{build_read_plan, PlanOptions};
use uuid::Uuid;

fn it_enabled() -> bool {
    env::var("COMM_IT_ENABLE").ok().as_deref() == Some("1")
}

fn parse_env<T: std::str::FromStr>(name: &str) -> Option<T> {
    env::var(name).ok()?.parse().ok()
}

fn parse_parity(name: &str) -> Option<SerialParity> {
    match env::var(name).ok().as_deref() {
        Some("None") => Some(SerialParity::None),
        Some("Even") => Some(SerialParity::Even),
        Some("Odd") => Some(SerialParity::Odd),
        _ => None,
    }
}

#[tokio::test]
async fn tcp_quality_ok_for_two_points_when_enabled() {
    if !it_enabled() {
        println!("SKIP tcp_quality_ok_for_two_points_when_enabled: COMM_IT_ENABLE!=1");
        return;
    }

    let host = match env::var("COMM_IT_TCP_HOST") {
        Ok(v) => v,
        Err(_) => {
            println!("SKIP tcp_quality_ok_for_two_points_when_enabled: COMM_IT_TCP_HOST not set");
            return;
        }
    };
    let port: u16 = match parse_env("COMM_IT_TCP_PORT") {
        Some(v) => v,
        None => {
            println!(
                "SKIP tcp_quality_ok_for_two_points_when_enabled: COMM_IT_TCP_PORT not set/invalid"
            );
            return;
        }
    };
    let unit_id: u8 = match parse_env("COMM_IT_TCP_UNITID") {
        Some(v) => v,
        None => {
            println!(
                "SKIP tcp_quality_ok_for_two_points_when_enabled: COMM_IT_TCP_UNITID not set/invalid"
            );
            return;
        }
    };

    let profile = ConnectionProfile::Tcp {
        channel_name: "it-tcp".to_string(),
        device_id: unit_id,
        read_area: RegisterArea::Holding,
        start_address: 0,
        length: 10,
        ip: host,
        port,
        timeout_ms: 1000,
        retry_count: 0,
        poll_interval_ms: 500,
    };

    let points = vec![
        CommPoint {
            point_key: Uuid::from_u128(1),
            hmi_name: "IT_U16".to_string(),
            data_type: DataType::UInt16,
            byte_order: ByteOrder32::ABCD,
            channel_name: "it-tcp".to_string(),
            address_offset: None,
            scale: 1.0,
        },
        CommPoint {
            point_key: Uuid::from_u128(2),
            hmi_name: "IT_F32".to_string(),
            data_type: DataType::Float32,
            byte_order: ByteOrder32::ABCD,
            channel_name: "it-tcp".to_string(),
            address_offset: None,
            scale: 1.0,
        },
    ];

    let plan = build_read_plan(&[profile.clone()], &points, PlanOptions::default()).unwrap();
    let driver = ModbusTcpDriver::new();

    let mut client = driver.connect(&profile).await.unwrap();
    for (i, job) in plan.jobs.iter().enumerate() {
        println!(
            "tcp job[{i}] area={:?} startAddress={} length={}",
            job.read_area, job.start_address, job.length
        );
        let raw = driver.read_with_client(&mut client, job).await;
        println!("tcp raw job[{i}] = {raw:?}");
    }

    let (results, stats) = execute_plan_once(&driver, &[profile], &points, &plan).await;

    assert_eq!(stats.total, 2);
    assert_eq!(results.len(), 2);

    assert_eq!(results[0].point_key, points[0].point_key);
    assert_eq!(results[0].quality, Quality::Ok);
    assert!(results[0].error_message.is_empty());
    assert!(!results[0].value_display.is_empty());
    assert!(results[0].value_display.parse::<f64>().is_ok());

    assert_eq!(results[1].point_key, points[1].point_key);
    assert_eq!(results[1].quality, Quality::Ok);
    assert!(results[1].error_message.is_empty());
    assert!(!results[1].value_display.is_empty());
    assert!(results[1].value_display.parse::<f64>().is_ok());
}

#[tokio::test]
async fn rtu_quality_ok_for_one_point_when_enabled() {
    if !it_enabled() {
        println!("SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_ENABLE!=1");
        return;
    }

    let serial_port = match env::var("COMM_IT_RTU_PORT") {
        Ok(v) => v,
        Err(_) => {
            println!("SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_RTU_PORT not set");
            return;
        }
    };
    let baud_rate: u32 = match parse_env("COMM_IT_RTU_BAUD") {
        Some(v) => v,
        None => {
            println!(
                "SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_RTU_BAUD not set/invalid"
            );
            return;
        }
    };
    let parity = match parse_parity("COMM_IT_RTU_PARITY") {
        Some(v) => v,
        None => {
            println!(
                "SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_RTU_PARITY not set/invalid"
            );
            return;
        }
    };
    let data_bits: u8 = match parse_env("COMM_IT_RTU_DATABITS") {
        Some(v) => v,
        None => {
            println!(
                "SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_RTU_DATABITS not set/invalid"
            );
            return;
        }
    };
    let stop_bits: u8 = match parse_env("COMM_IT_RTU_STOPBITS") {
        Some(v) => v,
        None => {
            println!(
                "SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_RTU_STOPBITS not set/invalid"
            );
            return;
        }
    };
    let slave_id: u8 = match parse_env("COMM_IT_RTU_SLAVEID") {
        Some(v) => v,
        None => {
            println!(
                "SKIP rtu_quality_ok_for_one_point_when_enabled: COMM_IT_RTU_SLAVEID not set/invalid"
            );
            return;
        }
    };

    let profile = ConnectionProfile::Rtu485 {
        channel_name: "it-rtu".to_string(),
        device_id: slave_id,
        read_area: RegisterArea::Holding,
        start_address: 0,
        length: 10,
        serial_port,
        baud_rate,
        parity,
        data_bits,
        stop_bits,
        timeout_ms: 1000,
        retry_count: 0,
        poll_interval_ms: 500,
    };

    let points = vec![CommPoint {
        point_key: Uuid::from_u128(1),
        hmi_name: "IT_U16".to_string(),
        data_type: DataType::UInt16,
        byte_order: ByteOrder32::ABCD,
        channel_name: "it-rtu".to_string(),
        address_offset: None,
        scale: 1.0,
    }];

    let plan = build_read_plan(&[profile.clone()], &points, PlanOptions::default()).unwrap();
    let driver = ModbusRtuDriver::new();

    let mut client = driver.connect(&profile).await.unwrap();
    for (i, job) in plan.jobs.iter().enumerate() {
        println!(
            "rtu job[{i}] area={:?} startAddress={} length={}",
            job.read_area, job.start_address, job.length
        );
        let raw = driver.read_with_client(&mut client, job).await;
        println!("rtu raw job[{i}] = {raw:?}");
    }

    let (results, stats) = execute_plan_once(&driver, &[profile], &points, &plan).await;

    assert_eq!(stats.total, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].point_key, points[0].point_key);
    assert_eq!(results[0].quality, Quality::Ok);
    assert!(results[0].error_message.is_empty());
    assert!(!results[0].value_display.is_empty());
    assert!(results[0].value_display.parse::<f64>().is_ok());
}
