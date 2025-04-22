// serialized format of api.weatherapi.com/v1/current.json
#[derive(serde::Deserialize, Debug)]
pub struct WeatherResponse {
    pub current: CurrentWeather,
}

#[derive(serde::Deserialize, Debug)]
pub struct CurrentWeather {
    pub temp_c: f64,
    pub temp_f: f64,
    pub condition: WeatherCondition,
    pub humidity: i32,
}

#[derive(serde::Deserialize, Debug)]
pub struct WeatherCondition {
    pub text: String,
}
