// serialized format of api.weatherapi.com/v1/current.json
#[derive(serde::Deserialize, Debug)]
pub struct WeatherResponse {
    pub location: Location,
    pub current: CurrentWeather,
}

#[derive(serde::Deserialize, Debug)]
pub struct Location {
    pub name: String,
    pub region: String,
    pub country: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct CurrentWeather {
    pub last_updated_epoch: i64,
    pub temp_c: f64,
    pub temp_f: f64,
    pub condition: WeatherCondition,
    pub humidity: i32,
}

#[derive(serde::Deserialize, Debug)]
pub struct WeatherCondition {
    pub text: String,
}
