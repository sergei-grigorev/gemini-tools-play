use tracing::{debug, error, info};

mod response;

const WEATHER_ENDPOINT: &str = "https://api.weatherapi.com/v1/current.json";

pub async fn get_weather(
    api_key: &str,
    location: &str,
) -> anyhow::Result<response::WeatherResponse> {
    info!("Fetching weather data for location: {}", location);
    let url = format!("{}?key={}&q={}", WEATHER_ENDPOINT, api_key, location);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let weather_response: response::WeatherResponse = response.json().await?;
        debug!("Weather data fetched successfully: {:?}", weather_response);
        Ok(weather_response)
    } else {
        error!("Failed to fetch weather data: {}", response.status());
        Err(anyhow::anyhow!("Failed to fetch weather data"))
    }
}
