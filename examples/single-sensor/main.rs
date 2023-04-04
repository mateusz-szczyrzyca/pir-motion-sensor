mod sensor;
use pir_motion_sensor::sensor::helpers::{process_detections_data, reading_data_from_sensors};
use pir_motion_sensor::sensor::motion::MotionSensor;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};

use std::{sync::Arc, time::SystemTime};

#[tokio::main]
async fn main() {
    // channel for sensor data
    #[allow(clippy::type_complexity)]
    let (detections_channel_sender, mut detections_channel_receiver): (
        Sender<(String, SystemTime)>,
        Receiver<(String, SystemTime)>,
    ) = mpsc::channel(100);

    //
    // sensors initialization - check README for more details about sensor parameters
    // in this example we initialize 4 motion sensors
    //
    let sensors_list = vec![
        // Bedroom
        MotionSensor::new(
            String::from("SensorBedroom5na5"), // name
            6,                                 // gpio PIN number
            100,                               // sensor refresh rate in miliseconds
            500,                               // sensor motion time period in miliseconds
            5,                                 // sensor minimal triggering number
            detections_channel_sender.clone(), // channel where sensor thread will be sending detections
            None, // None for real GPIO usage, Some(Vec<u128>) for unit tests, please check tests/* */
        ),
        // Main door
        MotionSensor::new(
            String::from("MainDoorSlow"),
            25,
            100,
            1000,
            4,
            detections_channel_sender.clone(),
            None,
        ),
        // Kitchen
        MotionSensor::new(
            String::from("KitchenFast"),
            20,
            20,
            1000,
            4,
            detections_channel_sender.clone(),
            None,
        ),
        // Garage
        MotionSensor::new(
            String::from("Garage"),
            16,
            100,
            500,
            5,
            detections_channel_sender,
            None,
        ),
    ];

    // starting detector in the background
    let mut sensors = Vec::new();

    // bulding list of sensors to use it later
    sensors_list.into_iter().for_each(|sensor| {
        let s = Arc::new(Mutex::new(sensor));
        sensors.push(s);
    });
    let sensors_list_copy = sensors.clone();
    //
    // task: processing detections data in async function
    //       You don't have to bother this if you don't want - just leave it as it is
    //

    tokio::spawn(async move { process_detections_data(sensors_list_copy).await });

    //
    // task: reading data from sensor using async function reading_from_sensor()
    //       You don't have to bother this if you don't want - just leave it as it is
    //
    tokio::spawn(async move {
        reading_data_from_sensors(sensors).await;
    });

    //
    // main loop: here we put logic to handle valid detections, place your code here
    //
    loop {
        if let Ok(detection_message) = detections_channel_receiver.try_recv() {
            // valid detection received
            // each "valid" detection contains the sensor name and time of detection as SystemTime
            let (detection_name, detection_time) = detection_message;

            println!("detection happened, sensor: {detection_name}, time: {detection_time:?} ");
            //
            // put your action here like alarm or turn on/off light
            // to interact with rest GPIOs you can check rppal lib examples here: https://github.com/golemparts/rppal/tree/master/examples
            //
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
