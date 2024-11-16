use pir_motion_sensor::sensor::motion::MotionSensor;
use tokio_util::sync::CancellationToken;

///////////////////////////////////////////////////////////////////////////////
// For more information check valid_detections.rs test file
///////////////////////////////////////////////////////////////////////////////
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
    async fn bugfix_test_detections() {
        #[allow(clippy::type_complexity)]
        let (detections_channel_in, mut detections_channel_out): (
            Sender<(String, SystemTime)>,
            Receiver<(String, SystemTime)>,
        ) = mpsc::channel(10);

        let test_cases_list: Vec<TestCase> = vec![
            TestCase {
                //
                // Issue: https://github.com/mateusz-szczyrzyca/pir-motion-sensor/issues/8
                // std::time::Instant was incorrectly used to measure motion time period - it was
                // reset each time after a valid detection
                //
                // expected_detections_count=0 in this scenario as within specified motion time period
                // no minimal triggering number is reached
                //
                sensor: MotionSensor::new(
                    String::from(
                        "Test for invalid motion time period processing (many detections)",
                    ), // name of sensor
                    0,    // pin number - not relevant in tests
                    100,  // sensor refresh rate in miliseconds
                    1000, // motion time period in miliseconds
                    11,   // required number of detection to classify
                    detections_channel_in.clone(),
                    Some(vec![90, 190, 290, 390, 490, 590, 690, 790, 890, 990]), // at which milisec test detections happen
                ),
                expected_detections_count: 0, //
                test_timeout_milisecs: 1500, // timeout for the test (miliseconds) - after this moment we stop sensor thread
            },
            TestCase {
                //
                // Issue: https://github.com/mateusz-szczyrzyca/pir-motion-sensor/issues/8
                // Same as above but only one detection
                //
                sensor: MotionSensor::new(
                    String::from(
                        "Test for invalid motion time period processing (single detection)",
                    ), // name of sensor
                    0,   // pin number - not relevant in tests
                    100, // sensor refresh rate in miliseconds
                    100, // motion time period in miliseconds
                    1,   // required number of detection to classify
                    detections_channel_in,
                    Some(vec![110]), // at which milisec test detections happen
                ),
                expected_detections_count: 0, //
                test_timeout_milisecs: 150, // timeout for the test (miliseconds) - after this moment we stop sensor thread
            },
        ];

        //
        //
        //
        for test_case in test_cases_list.into_iter() {
            //
            let mut sensors = Vec::new();
            let name = test_case.sensor.config.name.clone();
            sensors.push(Mutex::new(test_case.sensor));

            let sensors = Arc::new(sensors);

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
