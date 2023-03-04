## pir-motion-sensor

Rust library to interact mainly with PIR motion sensors. This lib was tested on HC-SR501 on Raspberry Pi 400 and Raspberry Pi 4B and it's widely used at my appartment and at my familys house - for smart alarm purposes (code will be published) and some in-house activies like turning on/off various devices based on motion detection.

&nbsp;

## Tested devices

Raspberry Pis:
- Raspberry Pi 4B 4 and 8 GB RAM - Raspbian GNU/Linux 11 (bullseye)
- Raspbbery Pi 400 4 GB RAM - Raspbian GNU/Linux 11 (bullseye)

Sensors:

- HC-SR501 PIR

&nbsp;

## Why this library?

HC-SR501 PIR is very cheap and widely available an infrared motion sensor and it's very capable to various project based on arduino/raspberry/stm32, etc. 

However, because it's very cheap, it can detects "noise" from time to time, which means detection happens when there are no real motion within it's detection range - this is a false positive detection.

To eliminate this problem I created this simple library which allows to initialize this sensor with various parameters that they changes it's detection characteristic. The library, based on sensor configuration, can "ignore" these false detections and help make this sensors very reliable.

HC-SR501 PIR is not probably the only one infrared sensor which can be supported by the library - if you tested with another motion sensor (or even with microwave) please let me know.

&nbsp;

## Prerequsities

Use this reference manual for HC-SR501 PIR: https://lastminuteengineers.com/pir-sensor-arduino-tutorial/

Based on this manual, you should:

- set Sensitivity Adjustment yellow screw to longest range possible
- set Time-Delay Adjustment yellow screw to shortest time possible
- set Trigger Selection Jumper to Multiple Trigger Mode

Allow of above settings can be programmatically changed by this library, so you won't need to touch jumper and screws again anymore after this operation.

If you are not sure if you did it correctly - attach your sensor to VCC (5V), GND of your raspberry, and simple LED diode with **resistor** (betwen 1-10 kΩ) to OUT signal from the sensor and based on LED check how this sensor work after these adjustments. Be aware, that sensors sets ~5V on it's OUT PIN, when it detects motion, which is at high state.

&nbsp;

## How it works

In this instruction, there is a term `valid detection` - this is a detection which is classified as *valid by this library - which may be not the same as OUT pin state of the sensor* - everything depends on your configuration.

In other words: depends on sensor configuration, there can be many detections made by sensor (here defined as setting it's  OUT pin at high state), but it does not mean, there will be any `valid detection` classified.

&nbsp;

## Configuration

To init your sensor, consider the following parameters:

- GPIO PIN number (obvious staff), not required for tests, see `tests/valid_detections.rs`

- **sensor refresh rate**
  This isn't refresh rate for a sensor itself, but refresh rate for loop reading sensor PIN state. Shorter time allows
  to read data from sensor more often, thus it leads to better "refresh rate" of sensors itself, but may impose sligtly higher system load (it may matters when you have many sensors and RPi 2/3 or Zero).

  Longer time may lead to "miss" some fast detections.

- **motion time period**
  It's the time limit which app classifies valid detection. If this period is shorter, it means it will be "harder" to detect motion
  within time range which means

- **minimal triggering number**
  When sensors detects motion (or something similar) it sets it's OUT pin to high state. If you make initial adjustment correctly, this can happen couple times per second and that's fine. This setting is a number of such high state sets which is required within **motion time period** to classify valid detection. This option is especially useful for excluding "noise" detections if >1

To short sum up these parameters: based on `sensor refresh rate` time it periodically reads state of sensor OUT pin. If there is a detection (or high state), the library will try to count up these states to `minimal triggering number` within `motion time period`. If `minimal triggering number` within `motion time period` is reached, then it means valid detection (from library standpoint) just happened.

Setting these parameters allow you to decide how sensitive and accurate is your sensor. 

&nbsp;
## Using in your project

&nbsp;
## Contributions

Contributions highly welcomed.

&nbsp;
## TODO
