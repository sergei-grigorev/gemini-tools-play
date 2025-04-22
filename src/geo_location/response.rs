#[derive(serde::Deserialize, Debug)]
pub struct TimeResponse {
    pub date: String,
    pub time_12: String,
}
