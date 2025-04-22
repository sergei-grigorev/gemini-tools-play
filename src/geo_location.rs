mod response;

use tracing::{debug, error, info};

const GEO_LOCATION_ENDPOINT: &str = "https://api.ipgeolocation.io/timezone";

pub async fn get_time(api_key: &str, location: &str) -> anyhow::Result<response::TimeResponse> {
    info!("Fetching time data for location: {}", location);
    let url = format!(
        "{}?apiKey={}&location={}",
        GEO_LOCATION_ENDPOINT, api_key, location
    );
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let weather_response: response::TimeResponse = response.json().await?;
        debug!("Time data fetched successfully: {:?}", weather_response);
        Ok(weather_response)
    } else {
        error!("Failed to fetch time data: {}", response.status());
        Err(anyhow::anyhow!("Failed to fetch time data"))
    }
}
