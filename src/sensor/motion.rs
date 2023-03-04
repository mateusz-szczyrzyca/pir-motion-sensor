use log::{info, warn};
use rppal::gpio::Mode::Input;
use rppal::gpio::{Gpio, IoPin};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::config::SensorConfig;

#[derive(Debug, Clone)]
pub struct MotionSensor {
    pub config: SensorConfig,
    pub detection_channel: SyncSender<(String, SystemTime)>,
    // last "valid" detection time
    pub last_detection_time: Option<SystemTime>,
    // last moment when gpio PIN was set to High (it may not mean "valid" detection - depends on configuration)
    pub last_any_detection_time: Option<Instant>,
    pub stop: bool,
    pub sensor_test_data: Option<Vec<u128>>, // vector of miliseconds detections starting from 0
}

impl MotionSensor {
    pub fn new(
        sensor_name: String,
        sensor_pin_number: u8,
        sensor_refresh_rate_milisecs: u64,
        sensor_motion_time_period_milisecs: u64,
        sensor_minimal_triggering_number: i16,
        sensor_transmission_channel: SyncSender<(String, SystemTime)>,
        sensor_test_data: Option<Vec<u128>>,
    ) -> Self {
        let config = SensorConfig {
            name: sensor_name,
            pin_number: sensor_pin_number,
            refresh_rate_milisecs: sensor_refresh_rate_milisecs,
            motion_time_period_milisecs: sensor_motion_time_period_milisecs,
            minimal_triggering_number: sensor_minimal_triggering_number,
        };

        let detection_channel = sensor_transmission_channel;

        Self {
            config,
            detection_channel,
            last_detection_time: None,
            last_any_detection_time: None,
            stop: false,
            sensor_test_data,
        }
    }

    pub fn start_detector(&mut self, stop_channel: Receiver<bool>) {
        info!("Starting sensor: {:#?}", self.config);

        let (detections_stream, detections_receiver) = mpsc::sync_channel(1);

        //
        // BEGIN: real detections from GPIO
        //
        if self.sensor_test_data.is_none() {
            let gpio = Gpio::new().unwrap();
            let mut pin: IoPin;
            loop {
                pin = match gpio.get(self.config.pin_number) {
                    Ok(p) => p.into_io(Input),
                    Err(_) => {
                        continue;
                    }
                };
                break;
            }

            // thread reading GPIO status
            let detection_stream_thread = detections_stream.clone();
            tokio::task::spawn_blocking(move || {
                loop {
                    if pin.is_high() {
                        pin.toggle();

                        // try to send as many as possible but if the channel is full, we just ignore it
                        // that's why try_send() is used here
                        let _ = detection_stream_thread.send(true);
                    }
                }
            });
        }

        //
        // BEGIN: testing detections
        //
        if self.sensor_test_data.is_some() {
            //
            // BEGIN: test logic
            //
            let detections_time_list = self.sensor_test_data.clone().unwrap();

            // thread sending detections at specific time
            tokio::task::spawn_blocking(move || {
                let detections_time_list_length = detections_time_list.len();
                let time_start = Instant::now();
                let mut index = 0;
                let mut current_detection_time;

                loop {
                    if index >= detections_time_list_length {
                        break;
                    }

                    current_detection_time = detections_time_list[index];

                    //
                    if time_start.elapsed().as_millis() == current_detection_time {
                        let _ = detections_stream.send(true);
                        index += 1;
                    }
                }
            });
            //
            // END: test logic
            //
        }

        let detection_moment: Option<Instant> = None;
        let mut sensor_trigger_count: i16 = 0;

        // let mut time_temp = Instant::now();

        let mut detection_moment = None;
        loop {
            if let Ok(stop_command) = stop_channel.try_recv() {
                if stop_command {
                    warn!("sensor stopping request came");
                    break;
                }
            }

            // reading detections from channel - these detections may come from real gpio
            // pin or from tests without gpio involved
            if detections_receiver.try_recv().is_ok() {
                if detection_moment.is_none() {
                    // first init of this variable hee
                    detection_moment = Some(Instant::now());
                }

                let time_difference = detection_moment.unwrap().elapsed().as_millis();

                if time_difference > self.config.motion_time_period_milisecs.into() {
                    // this is a new detection - reset time and counter
                    sensor_trigger_count = 0;
                    detection_moment = Some(Instant::now());
                }

                sensor_trigger_count += 1;

                // println!(
                //     "-> received detection: {} milisec, count: {}, minimal_triggering_num: {}",
                //     time_temp.elapsed().as_millis(),
                //     sensor_trigger_count,
                //     self.config.minimal_triggering_number
                // );

                if sensor_trigger_count >= self.config.minimal_triggering_number {
                    // minimal_triggering_number is reached - this is valid detection
                    let t = SystemTime::now();
                    self.last_detection_time = Some(t);

                    self.detection_channel
                        .send((self.config.name.clone(), t))
                        .unwrap();

                    // reset counter - next detection will be counted as different one
                    sensor_trigger_count = 0;
                }
                thread::sleep(Duration::from_millis(self.config.refresh_rate_milisecs));
            }
        }
    }
}
