use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::sensor::motion::MotionSensor;
use std::sync::Arc;

pub async fn process_detections_data(
    sensor: Vec<Arc<Mutex<MotionSensor>>>,
    process: CancellationToken,
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
    sensors: Vec<Arc<Mutex<MotionSensor>>>,
    reading: CancellationToken,
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
    sensors: Vec<Arc<Mutex<MotionSensor>>>,
    stop_command: CancellationToken,
) {
    let sensors_copy = sensors.clone();
    let token_copy = stop_command.clone();
    tokio::spawn(async move { process_detections_data(sensors, stop_command.clone()).await });

    tokio::spawn(async move {
        reading_data_from_sensors(sensors_copy, token_copy).await;
    });
}
