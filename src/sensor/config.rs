use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SensorConfig {
    pub name: String,
    pub pin_number: u8,
    pub refresh_rate_milisecs: u64,       // miliseconds
    pub motion_time_period_milisecs: u64, // miliseconds
    pub minimal_triggering_number: i16,
}
