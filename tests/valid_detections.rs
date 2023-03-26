use pir_motion_sensor::sensor::motion::MotionSensor;

struct TestCase {
    sensor: MotionSensor,
    expected_detections_count: u64,
    test_timeout_milisecs: u128,
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{Duration, Instant, SystemTime},
    };

    use pir_motion_sensor::sensor::helpers::{process_detections_data, reading_data_from_sensors};
    use tokio::sync::mpsc::{self, Receiver, Sender};
    use tokio::sync::Mutex;

    use super::*;

    #[tokio::test]
    async fn valid_detections() {
        #[allow(clippy::type_complexity)]
        let (detections_channel_in, mut detections_channel_out): (
            Sender<(String, SystemTime)>,
            Receiver<(String, SystemTime)>,
        ) = mpsc::channel(100);

        let test_cases_list: Vec<TestCase> = vec![
            TestCase {
                //
                // Test Case: we have two detection, one at 500 milisec, another at 1000 milisec.
                //            because sensor refresh rate is 500 milisec, one detection will be recognized
                //            in period 0-500 milisec, another in period 500-1000 milisec. Because there are
                //            two detections, sensor configuration in this test (required number of detection to classify)
                //            will classify them as 1 valid detection and this is our expected number as a value for
                //            expected detections count
                //
                sensor: MotionSensor::new(
                    String::from("SensorValidDetections1"), // name of the sensor
                    0,                                      // pin number - not relevant in tests
                    500,                                    // sensor refresh rate in miliseconds
                    1000,                                   // motion time period in miliseconds
                    2, // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![500, 1000]), // at which milisec detection happens, here at 500 ms and 1000 ms
                ),
                expected_detections_count: 1, // how many "valid" detections will be classified based on sensor and detection configurations
                test_timeout_milisecs: 1100, // timeout for the test (miliseconds) - after this moment we stop sensor thread, it's good
                                             // to set timeout about +100 miliseconds more than last test detection
            },
            TestCase {
                //
                // Test Case: here we have six testing detections at 100, 200, 300, 400, 500 and 501 milisecs
                //            sensor refresh rate is 100 milisec and motion period is checked for 500 msec and requires
                //            5 detections to classify them as 1 "valid". So in this test we expect 1 valid detection,
                //            as detection at 501 milisecs is after limit of motion time period.
                //
                //
                sensor: MotionSensor::new(
                    String::from("SensorValidDetections2"), // name of the sensor
                    0,                                      // pin number - not relevant in tests
                    100,                                    // sensor refresh rate in miliseconds
                    500,                                    // motion time period in miliseconds
                    5, // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![100, 200, 300, 400, 500, 501]), // at which milisec detection happens, here at 500 ms and 1000 ms
                ),
                expected_detections_count: 1, // how many "valid" detections will be classified based on sensor and detection configurations
                test_timeout_milisecs: 550, // timeout for the test (miliseconds) - after this moment we stop sensor thread, it's good
                                            // to set timeout about +50 miliseconds more than last test detection
            },
            TestCase {
                //
                // Test Case: similar case as previous one, but we have more detections but not enough to have
                //            additional "valid" detection - last detection at 1001 milisec happens too late for
                //            this motion time period so we end up with only 1 valid detection in this scenario
                //
                //
                sensor: MotionSensor::new(
                    String::from("SensorValidDetections3"), // name of the sensor
                    0,                                      // pin number - not relevant in tests
                    100,                                    // sensor refresh rate in miliseconds
                    500,                                    // motion time period in miliseconds
                    5, // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![100, 200, 300, 400, 500, 501, 520, 540, 560, 1001]), // at which milisec detection happens
                ),
                expected_detections_count: 1, // how many "valid" detections will be classified based on sensor and detection configurations
                test_timeout_milisecs: 1100, // timeout for the test (miliseconds) - after this moment we stop sensor thread,
            },
            TestCase {
                sensor: MotionSensor::new(
                    String::from("SensorValidDetections3_ManyDetections"),
                    0,
                    100,
                    200,
                    1,
                    detections_channel_in.clone(),
                    Some(vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]),
                ),
                expected_detections_count: 10, // 10 because we can count all of these detections
                test_timeout_milisecs: 1100,
            },
            TestCase {
                /*
                 */
                sensor: MotionSensor::new(
                    String::from("SensorValidDetections4_OneBigDetection"),
                    0,
                    100,
                    1000,
                    10,
                    detections_channel_in,
                    Some(vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]),
                ),
                expected_detections_count: 1, // only 1 because motion time period is 1000 milisecs
                test_timeout_milisecs: 1100,
            },
        ];

        // #[allow(clippy::type_complexity)]
        // let (detections_channel_sender, mut detections_channel_receiver): (
        //     Sender<(String, SystemTime)>,
        //     Receiver<(String, SystemTime)>,
        // ) = mpsc::channel(100);

        //
        //
        //
        for test_case in test_cases_list.into_iter() {
            //
            let mut sensors = Vec::new();
            sensors.push(Arc::new(Mutex::new(test_case.sensor)));

            // bulding list of sensors to use it later
            let sensors_list_copy = sensors.clone();

            tokio::spawn(async move { process_detections_data(sensors_list_copy).await });

            tokio::spawn(async move {
                reading_data_from_sensors(sensors).await;
            });

            let mut detections_count = 0;
            let test_time_start = Instant::now();

            loop {
                if let Ok(_detection_message) = detections_channel_out.try_recv() {
                    detections_count += 1;
                }

                if test_time_start.elapsed().as_millis() > test_case.test_timeout_milisecs {
                    break;
                }

                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            assert_eq!(detections_count, test_case.expected_detections_count);
        }
        println!("finished tests");
    }
}
