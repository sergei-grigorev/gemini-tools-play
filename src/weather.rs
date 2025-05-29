use crate::error::AppError;
use tracing::{debug, error, info};

// Module containing response data structures for weather information
mod response;

// API endpoint for the WeatherAPI current weather data
const WEATHER_ENDPOINT: &str = "https://api.weatherapi.com/v1/current.json";

/// Fetches current weather information for a specific location using the WeatherAPI.
///
/// # Arguments
/// * `api_key` - The API key for accessing the WeatherAPI service
/// * `location` - Location string in format "city,country" (e.g., "London,GB")
///
/// # Returns
/// * `WeatherResponse` containing temperature, condition, and humidity information
/// * Error if the API request fails or returns an unsuccessful status code
pub async fn get_weather(
    api_key: &str,
    location: &str,
) -> Result<response::WeatherResponse, AppError> {
    info!("Fetching weather data for location: {}", location);

    // Construct the API URL with query parameters
    let url = format!("{}?key={}&q={}", WEATHER_ENDPOINT, api_key, location);

    // Create HTTP client and send the request
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        // Parse successful response into WeatherResponse struct
        let weather_response: response::WeatherResponse = response.json().await?;
        debug!("Weather data fetched successfully: {:?}", weather_response);
        Ok(weather_response)
    } else {
        // Log and return error for unsuccessful responses
        error!("Failed to fetch weather data: {}", response.status());
        Err(AppError::ApiRequestFailed(format!("Failed to fetch weather data: {}", response.status())))
    }
}
