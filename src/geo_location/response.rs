/// Response structure for the IPGeolocation timezone API
/// Contains date and time information for a specific location
#[derive(serde::Deserialize, Debug)]
pub struct TimeResponse {
    /// Current date in format "YYYY-MM-DD"
    pub date: String,
    /// Current time in 12-hour format (e.g., "08:30 PM")
    pub time_12: String,
}
