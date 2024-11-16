use pir_motion_sensor::sensor::motion::MotionSensor;
use tokio_util::sync::CancellationToken;

struct TestCase {
    sensor: MotionSensor,
    expected_detections_count: u64,
    test_timeout_milisecs: u64,
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{Duration, Instant, SystemTime},
    };

    use pir_motion_sensor::sensor::helpers::spawn_detection_threads;
    use tokio::sync::mpsc::{self, Receiver, Sender};
    use tokio::sync::Mutex;

    use super::*;

    #[tokio::test]
    async fn valid_detections() {
        #[allow(clippy::type_complexity)]
        let (detections_channel_in, mut detections_channel_out): (
            Sender<(String, SystemTime)>,
            Receiver<(String, SystemTime)>,
        ) = mpsc::channel(10);

        let test_cases_list: Vec<TestCase> = vec![
            TestCase {
                //
                // Test Case: we have two detection, one at 490 milisec, another at 990 milisec.
                //            because sensor refresh rate is 500 milisec, one detection will be recognized
                //            in period 0-500 milisec, another in period 500-1000 milisec. Because there are
                //            two detections, sensor configuration in this test (required number of detection to classify)
                //            will classify them as 1 valid detection and this is our expected number as a value for
                //            expected detections count.
                //
                sensor: MotionSensor::new(
                    String::from("Simple detection 1"), // name of the sensor
                    0,                                  // pin number - not relevant in tests
                    500,                                // sensor refresh rate in miliseconds
                    1000,                               // motion time period in miliseconds
                    2,                                  // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![490, 990]), // at which milisec test detection happen, here at 490 ms and 500 ms
                                          //
                                          // WARNING: to have deterministic results it's good to keeping these detections moments
                                          //          around 10 milisec "away" from sensor refresh rate. If you set values on the edge of
                                          //          sensor refresh rate then test result may not be deterministic (sometimes test will fail)
                ),
                expected_detections_count: 1, // how many "valid" detections will be classified based on sensor and detection configurations
                test_timeout_milisecs: 1050, // timeout for the test (miliseconds) - after this moment we stop sensor thread, it's good
                                             // to set timeout about +50 miliseconds more than last test detection
            },
            TestCase {
                //
                // Test Case: here we have six testing detections at 90, 190, 290, 390, 490 and 501 milisecs
                //            sensor refresh rate is 100 milisec and motion period is checked for 500 msec and requires
                //            5 detections to classify them as 1 "valid". So in this test we expect 1 valid detection,
                //            as detection at 501 milisecs is after limit of motion time period.
                //
                //
                sensor: MotionSensor::new(
                    String::from("Simple detection 2"), // name of the sensor
                    0,                                  // pin number - not relevant in tests
                    100,                                // sensor refresh rate in miliseconds
                    500,                                // motion time period in miliseconds
                    5,                                  // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![90, 190, 290, 390, 490, 501]), // at which milisec test detection happen
                ),
                expected_detections_count: 1, // how many "valid" detections will be classified based on sensor and detection configurations
                test_timeout_milisecs: 550, // timeout for the test (miliseconds) - after this moment we stop sensor thread,
            },
            TestCase {
                //
                // Test Case: similar case as previous one, but we have more detections but not enough to have
                //            additional "valid" detection - last detection at 1001 milisec happens too late for
                //            this motion time period so we end up with only 1 valid detection in this scenario
                //
                //
                sensor: MotionSensor::new(
                    String::from("Two valid detections from 10"), // name of the sensor
                    0,   // pin number - not relevant in tests
                    100, // sensor refresh rate in miliseconds
                    500, // motion time period in miliseconds
                    5,   // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![90, 190, 290, 390, 490, 550, 650, 750, 950, 1010]), // at which milisec test detection happen
                ),
                expected_detections_count: 1, // how many "valid" detections will be classified based on sensor and detection configurations
                test_timeout_milisecs: 1020, // timeout for the test (miliseconds) - after this moment we stop sensor thread,
            },
            TestCase {
                //
                // Test Case: there is a quick refresh rate (100 milisec) and only 1 detection
                //            is required to classify a valid detection. This test makes 10 detections, each in another
                //            iteration of sensor refresh rate so we try to achieve 10 valid detections here
                //
                //
                sensor: MotionSensor::new(
                    String::from("10/10 fast detections"), // name of the sensor
                    0,                                     // pin number - not relevant in tests
                    100,                                   // sensor refresh rate in miliseconds
                    200,                                   // motion time period in miliseconds
                    1, // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![90, 190, 290, 390, 490, 590, 690, 790, 890, 990]), // at which milisec test detections happen
                ),
                expected_detections_count: 10, // 10 because we can count all of these detections
                test_timeout_milisecs: 1050, // timeout for the test (miliseconds) - after this moment we stop sensor thread,
            },
            TestCase {
                //
                // Test Case: similar as previous but this time motion_time_period=1000 ms - it means we should count only
                //            one VALID detection instead of 10 as previous.
                //
                sensor: MotionSensor::new(
                    String::from("Single detection from 10"), // name of sensor
                    0,                                        // pin number - not relevant in tests
                    100,                                      // sensor refresh rate in miliseconds
                    1000,                                     // motion time period in miliseconds
                    10, // required number of detection to classify
                    detections_channel_in,
                    Some(vec![90, 190, 290, 390, 490, 590, 690, 790, 890, 990]), // at which milisec test detections happen
                ),
                expected_detections_count: 1, // only 1 because motion time period is 1000 milisecs
                test_timeout_milisecs: 1050, // timeout for the test (miliseconds) - after this moment we stop sensor thread
            },
        ];

        //
        //
        //
        for test_case in test_cases_list.into_iter() {
            //
            let mut sensors_vec = Vec::new();
            let name = test_case.sensor.config.name.clone();
            sensors_vec.push(Mutex::new(test_case.sensor));
            let sensors = Arc::new(sensors_vec);

            let token = Arc::new(CancellationToken::new());

            spawn_detection_threads(sensors, token.clone());

            let mut detections_count = 0;
            let test_time_start = Instant::now();

            println!("current test case: {}", name);

            loop {
                if let Ok(_detection_message) = detections_channel_out.try_recv() {
                    detections_count += 1;
                }

                if test_time_start.elapsed().as_millis() as u64 > test_case.test_timeout_milisecs {
                    println!("test timeout.");
                    break;
                }

                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            // finishing test
            token.cancel();
            assert_eq!(detections_count, test_case.expected_detections_count);
        }
        println!("finished tests");
    }
}
