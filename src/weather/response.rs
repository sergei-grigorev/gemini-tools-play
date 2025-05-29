/// Response structure for the WeatherAPI current weather endpoint
/// Represents the JSON structure returned by api.weatherapi.com/v1/current.json
#[derive(serde::Deserialize, Debug)]
pub struct WeatherResponse {
    /// Current weather conditions
    pub current: CurrentWeather,
}

/// Contains the current weather data including temperature and conditions
#[derive(serde::Deserialize, Debug)]
pub struct CurrentWeather {
    /// Temperature in Celsius
    pub temp_c: f64,
    /// Temperature in Fahrenheit
    pub temp_f: f64,
    /// Text description of the current weather condition
    pub condition: WeatherCondition,
    /// Humidity percentage (0-100)
    pub humidity: i32,
}

/// Weather condition description
#[derive(serde::Deserialize, Debug)]
pub struct WeatherCondition {
    /// Human-readable description of the weather condition (e.g., "Partly cloudy")
    pub text: String,
}
