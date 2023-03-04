use pir_motion_sensor::sensor::motion::MotionSensor;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

struct TestCase {
    sensor: MotionSensor,
    expected_detections_count: u64,
    test_timeout_milisecs: u128,
}

#[cfg(test)]
mod tests {
    use std::{
        sync::mpsc,
        time::{Instant, SystemTime},
    };

    use super::*;

    #[tokio::test]
    async fn invalid_detections() {
        #[allow(clippy::type_complexity)]
        let (detections_channel_in, detections_channel_out): (
            SyncSender<(String, SystemTime)>,
            Receiver<(String, SystemTime)>,
        ) = sync_channel(0);

        let test_cases_list: Vec<TestCase> = vec![TestCase {
            //
            // Test Case: check valid_detections.rs for explanation
            //
            sensor: MotionSensor::new(
                String::from("SensorInValidDetections1"), // name of the sensor
                0,                                        // pin number - not relevant in tests
                500,                                      // sensor refresh rate in miliseconds
                1000,                                     // motion time period in miliseconds
                3, // required number of detection to classify
                detections_channel_in.clone(),
                Some(vec![500, 1000]), // detections time (milisecs)
            ),
            expected_detections_count: 0,
            test_timeout_milisecs: 1100,
        }];

        //
        //
        //
        for test_case in test_cases_list.into_iter() {
            let mut s = test_case.sensor;

            println!(
                "testing {}, timeout (milisecs): {}",
                s.config.name, test_case.test_timeout_milisecs
            );
            let (stop_detection_cmd, receiver) = mpsc::channel();

            // starting detector in the background
            tokio::task::spawn_blocking(move || s.start_detector(receiver));

            let mut time_start = None;

            let mut detections_count = 0;
            loop {
                // timer start for the first time
                if time_start.is_none() {
                    time_start = Some(Instant::now());
                }

                // test timeout
                if time_start.unwrap().elapsed().as_millis() >= test_case.test_timeout_milisecs {
                    println!("test timeout.");
                    break;
                }

                // receiving (or not) testing detections
                if detections_channel_out.try_recv().is_ok() {
                    detections_count += 1;
                }
            }

            // stopping this sensor
            stop_detection_cmd.send(true).unwrap();

            assert_eq!(detections_count, test_case.expected_detections_count);
        }
        println!("finished tests?");
    }
}
