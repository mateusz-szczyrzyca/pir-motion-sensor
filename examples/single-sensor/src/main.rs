use pir_motion_sensor::sensor::motion::MotionSensor;
use std::{
    sync::mpsc::{self, sync_channel, Receiver, SyncSender},
    time::SystemTime,
};

#[tokio::main]
async fn main() {
    // channel for sensor data
    #[allow(clippy::type_complexity)]
    let (detections_channel_sender, detections_channel_receiver): (
        SyncSender<(String, SystemTime)>,
        Receiver<(String, SystemTime)>,
    ) = sync_channel(0);

    // sensor initialization - check README for more details about sensor parameters
    let mut sensor_bedroom = MotionSensor::new(
        String::from("SensorBedroom"), // name
        6,                             // gpio PIN number
        100,                           // sensor refresh rate in miliseconds
        300,                           // sensor motion time period in miliseconds
        2,                             // sensor minimal triggering number
        detections_channel_sender,     // channel where sensor thread will be sending detections
        None,                          // None for real GPIO usage, Some(Vec<u128>) for unit tests
    );

    // this is for sending stop requests for motion sensor thread
    let (_stop_command, receiver) = mpsc::channel();

    // starting detector in the background
    tokio::task::spawn_blocking(move || sensor_bedroom.start_detector(receiver));

    loop {
        if let Ok(detection_msg) = detections_channel_receiver.try_recv() {
            // detection received
            // each "valid" detection constains sensor name and time of detection as SystemTime()
            let (detection_name, detection_time) = detection_msg;

            println!("detection happened, sensor: {detection_name}, time: {detection_time:?} ");
            //
            // put your action here like alarm or turn on/off light
            //
        }
    }
}
