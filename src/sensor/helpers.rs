use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::sensor::motion::MotionSensor;
use std::sync::Arc;

pub async fn process_detections_data(
    sensor: Arc<Vec<Mutex<MotionSensor>>>,
    process: Arc<CancellationToken>,
) {
    let mut detection_data: Vec<(i16, Instant)> = vec![(0, Instant::now()); sensor.len()];
    loop {
        if process.is_cancelled() {
            break;
        }

        let s = sensor.clone();
        for (idx, r) in s.iter().enumerate() {
            let (last_trigger_count, last_check_time) = detection_data[idx];
            if let Ok(mut locked_resource) = r.try_lock() {
                let (tmp_trigger, tmp_time) = locked_resource
                    .process_detections(last_trigger_count, last_check_time)
                    .await;

                detection_data[idx] = (tmp_trigger, tmp_time);
            }
        }
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
}

pub async fn reading_data_from_sensors(
    sensors: Arc<Vec<Mutex<MotionSensor>>>,
    reading: Arc<CancellationToken>,
) {
    loop {
        if reading.is_cancelled() {
            break;
        }

        let s = sensors.clone();
        for r in s.iter() {
            if let Ok(mut data) = r.try_lock() {
                data.reading_from_sensor().await;
            }
        }
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
}

pub fn spawn_detection_threads(
    sensors: Arc<Vec<Mutex<MotionSensor>>>,
    stop_command: Arc<CancellationToken>,
) {
    let sensors_copy = sensors.clone();

    let stop_command_copy = stop_command.clone();
    tokio::spawn(async move { process_detections_data(sensors_copy, stop_command_copy).await });

    let stop_command_copy = stop_command.clone();
    tokio::spawn(async move {
        reading_data_from_sensors(sensors.clone(), stop_command_copy).await;
    });
}
