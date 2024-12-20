use log::info;
use rppal::gpio::Mode::Input;
use rppal::gpio::{Gpio, IoPin};
use std::time::{Instant, SystemTime};
use tokio::sync::mpsc::{self, Receiver, Sender};

const DETECTION_MARGIN_MILISECS: u64 = 10;

use super::config::SensorConfig;

#[derive(Debug)]
pub struct MotionSensor {
    // config
    pub config: SensorConfig,
    pub detection_channel: Sender<(String, SystemTime)>,
    // last "valid" detection time
    pub last_detection_time: Option<SystemTime>,
    // last moment when gpio PIN was set to High (it may not mean "valid" detection - depends on configuration)
    pub last_any_detection_time: Option<Instant>,
    // additional settings for the future
    pub additional_settings: SensorAdditionalSettings,
}

#[derive(Debug)]
pub struct SensorAdditionalSettings {
    pub stop: bool,
    pub sensor_test_data: Option<Vec<u64>>,
    pub sensor_test_time: Option<Instant>,
    pub sensor_test_index: usize,
    pub pin: Option<IoPin>,
    pub detection_stream_channel: Option<Sender<bool>>,
    pub detections_receiver: Receiver<bool>,
}

impl MotionSensor {
    pub fn new(
        sensor_name: String,
        sensor_pin_number: u8,
        sensor_refresh_rate_milisecs: u64,
        sensor_motion_time_period_milisecs: u64,
        sensor_minimal_triggering_number: i16,
        sensor_transmission_channel: Sender<(String, SystemTime)>,
        sensor_test_data: Option<Vec<u64>>,
    ) -> Self {
        let config = SensorConfig {
            name: sensor_name,
            pin_number: sensor_pin_number,
            refresh_rate_milisecs: sensor_refresh_rate_milisecs,
            motion_time_period_milisecs: sensor_motion_time_period_milisecs,
            minimal_triggering_number: sensor_minimal_triggering_number,
        };

        // default values
        let mut pin_init = None;
        let detection_stream_channel_init;

        if sensor_test_data.is_none() {
            let gpio = Gpio::new().unwrap();
            let pin: IoPin;
            loop {
                pin = match gpio.get(sensor_pin_number) {
                    Ok(p) => p.into_io(Input),
                    Err(_) => {
                        continue;
                    }
                };
                break;
            }

            pin_init = Some(pin);
        }

        let (detections_stream, detections_receiver) = mpsc::channel(10);

        detection_stream_channel_init = Some(detections_stream);

        //
        // initialization
        //
        info!("Starting sensor: {:#?}", config.name);

        // by default it's None when sensor is initialized
        // for testing it will be initialized with current time
        let sensor_test_time = None;
        let sensor_test_index = 0;

        let additional_settings = SensorAdditionalSettings {
            stop: false,
            sensor_test_data,
            sensor_test_time,
            sensor_test_index,
            pin: pin_init,
            detection_stream_channel: detection_stream_channel_init,
            detections_receiver,
        };

        Self {
            config,
            detection_channel: sensor_transmission_channel,
            last_detection_time: None,
            last_any_detection_time: None,
            additional_settings,
        }
    }

    pub async fn reading_from_sensor(&mut self) {
        //
        let detection_stream_channel = self.additional_settings.detection_stream_channel.clone();

        //
        // BEGIN: real detections from GPIO
        //
        if self.additional_settings.sensor_test_data.is_none() {
            let pin = self.additional_settings.pin.as_mut().expect(
                "sensor not initialized - this method should be called AFTER start_detector()",
            );
            //
            if pin.is_high() {
                // try to send as many as possible but if the channel is full, we just ignore it
                // that's why try_send() is used here
                // unwrap_or_default() - because we don't care if each single detection is successfully
                //                       sent
                if let Some(detection_stream_channel) = detection_stream_channel.as_ref() {
                    detection_stream_channel
                        .try_send(true)
                        .unwrap_or_default()
                }
            }
        }
        //
        // END: real detections from GPIO
        //

        //
        // BEGIN: testing detections logic: we take detections from Vec<u64> - each such detection
        //        invokes same actions as normal GPIO pin
        //
        if self.additional_settings.sensor_test_data.is_some() {
            if self.additional_settings.sensor_test_time.is_none() {
                // starting internal timer which will be used as a reference for testing
                self.additional_settings.sensor_test_time = Some(Instant::now());
            }

            let detections_time_list = self.additional_settings.sensor_test_data.clone().unwrap();
            let detections_time_list_length = detections_time_list.len();

            let current_index = self.additional_settings.sensor_test_index;

            if current_index < detections_time_list_length {
                let milisecs_now = self
                    .additional_settings
                    .sensor_test_time
                    .unwrap()
                    .elapsed()
                    .as_millis() as u64;

                // taking detection time from the list
                let testing_detection_time_milisecs = detections_time_list[current_index];

                // this +10 milisec is added as a precaution - sometimes we want to have detection at 500 milisec,
                // but it happens at 501 or 502 milisecond which is "too late". Normal condition here (==) would fail so
                // we check broader condition ">=" but using only this this would be not sufficient here (each subsequent detection
                // will happen in >= time - obvious). So additionaly we check a small margin of +10 additional miliseconds which
                // is here as DETECTION_MARGIN_MILISECS constant
                if milisecs_now >= testing_detection_time_milisecs
                    && milisecs_now + DETECTION_MARGIN_MILISECS > testing_detection_time_milisecs
                {
                    // updating index - next time we will take next detection from the list
                    self.additional_settings.sensor_test_index += 1;

                    // sending testing detection to the channel which looks like "real"
                    if let Some(detection_channel) = detection_stream_channel {
                        detection_channel
                            .try_send(true)
                            .expect("cannot use channel for detection stream");
                    }
                }
            }
        }
        //
        // END: testing detections logic: we take detections from Vec<u64> - each such detection
        //      invokes same actions as normal GPIO pin
        //
    }

    //
    // processing detections, they may be real from GPIO or from testing code.
    //
    pub async fn process_detections(
        &mut self,
        last_sensor_trigger_count: i16,
        last_check_time: Instant,
    ) -> (i16, Instant) {
        let mut sensor_trigger_count = last_sensor_trigger_count;

        if last_check_time.elapsed().as_millis() as u64 <= self.config.refresh_rate_milisecs {
            // "sensor refresh rate" - if it's too early to check, then we return instantly
            // but we don't modify "last_check_time" - in another function await this time still
            // will be used to determine if it's time to check internal channel for detections
            return (last_sensor_trigger_count, last_check_time);
        }

        // sensor refresh rate is larger or equal the actual timer - now we can read state of the
        // channel and proceed with the logic. In that sense, short "refresh_rate_milisec" values
        // just means we read detection channel more often and we can count more detections.

        //
        // reading detections from channel - these detections may come from real gpio
        // pin or from tests without gpio involved
        //
        // try_recv() because this is an async func - we don't care if there is no detection data
        // in the channel and moving forward asap. If there are detections in the channel then we
        // will proceed them normally, but the highest priority of this function is to don't block it.
        //
        if self
            .additional_settings
            .detections_receiver
            .try_recv()
            .is_ok()
        {
            // this func is async so we increment counter (or not)
            sensor_trigger_count += 1;

            // because we use Instant::now, the real time difference needs to be multiply by counts to
            // reflect real motion time period time
            let time_difference = self.config.refresh_rate_milisecs * sensor_trigger_count as u64;

            if time_difference > self.config.motion_time_period_milisecs {
                // this is a new detection - reset counter
                sensor_trigger_count = 1;
            }

            if sensor_trigger_count >= self.config.minimal_triggering_number {
                //
                // minimal_triggering_number is reached - this is valid detection so send it to the main channel
                //
                let t = SystemTime::now();
                self.last_detection_time = Some(t);

                // sending real (VALID) detection to the main channel as we reached suitable "minimal_triggering_number"
                self.detection_channel
                    .try_send((self.config.name.clone(), t))
                    .unwrap_or_default();

                // reset counter - next detection will be counted as different one from zero again
                sensor_trigger_count = 0;
            }
        }

        // return current counter and time which later will be used to determine another detections (valid or pre-detections)
        (sensor_trigger_count, Instant::now())
    }
}
