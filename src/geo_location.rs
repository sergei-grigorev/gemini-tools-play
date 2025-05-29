// Module containing response data structures for time information
mod response;

use crate::error::AppError;
use tracing::{debug, error, info};

// API endpoint for the IPGeolocation timezone service
const GEO_LOCATION_ENDPOINT: &str = "https://api.ipgeolocation.io/timezone";

/// Fetches current time information for a specific location using the IPGeolocation API.
///
/// # Arguments
/// * `api_key` - The API key for accessing the IPGeolocation service
/// * `location` - Location string in format "city,country" (e.g., "London,GB")
///
/// # Returns
/// * `TimeResponse` containing date and time information for the specified location
/// * Error if the API request fails or returns an unsuccessful status code
pub async fn get_time(api_key: &str, location: &str) -> Result<response::TimeResponse, AppError> {
    info!("Fetching time data for location: {}", location);

    // Construct the API URL with query parameters
    let url = format!(
        "{}?apiKey={}&location={}",
        GEO_LOCATION_ENDPOINT, api_key, location
    );

    // Create HTTP client and send the request
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        // Parse successful response into TimeResponse struct
        let time_response: response::TimeResponse = response.json().await?;
        debug!("Time data fetched successfully: {:?}", time_response);
        Ok(time_response)
    } else {
        // Log and return error for unsuccessful responses
        error!("Failed to fetch time data: {}", response.status());
        Err(AppError::ApiRequestFailed(format!("Failed to fetch time data: {}", response.status())))
    }
}
