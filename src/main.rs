// External modules for API integration
mod geo_location; // Time API integration
mod weather; // Weather API integration
mod error; // Custom error types

use error::AppError;

use std::{env, io::Write};

use futures::stream::{self, StreamExt};
use genai::{
    Client,
    chat::{ChatMessage, ChatRequest, ChatResponse, MessageContent, Tool, ToolCall, ToolResponse},
};
use serde_json::json;
use tracing::{Instrument, debug, error, info, span};
use tracing_subscriber::EnvFilter;

// Gemini model version used for this application
const MODEL: &str = "gemini-2.0-flash";

/// Entry point for the Gemini-powered weather and time assistant.
///
/// This function:
/// 1. Sets up logging with tracing
/// 2. Configures the Gemini client
/// 3. Defines tools for weather and time queries
/// 4. Processes user input in a continuous loop until 'exit' is received
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize logging with environment-based filter configuration
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Initialize the Gemini API client
    let client = Client::default();

    // Define tool for weather information queries
    // This tool requires city, country, and temperature unit parameters
    let weather_tool = Tool::new("get_weather")
        .with_description("Get the current weather for a location")
        .with_schema(json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "City name in English, Latin script (e.g., \"Seattle\")."
                },
                "country": {
                    "type": "string",
                    "description": "ISO‑3166‑1 alpha‑2 country code, e.g., \"US\"."
                },
                "unit": {
                    "type": "string",
                    "enum": ["C", "F"],
                    "description": "Temperature unit (C for Celsius, F for Fahrenheit)"
                }
            },
            "required": ["city", "country", "unit"]
        }));

    // Define tool for time information queries
    // This tool requires city and country parameters
    let current_time_tool: Tool = Tool::new("get_current_time")
        .with_description("Get the current time for a location")
        .with_schema(json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "City name in English, Latin script (e.g., \"Seattle\")."
                },
                "country": {
                    "type": "string",
                    "description": "ISO‑3166‑1 alpha‑2 country code, e.g., \"US\"."
                }
            },
            "required": ["city", "country"]
        }));

    // Initialize chat request with system prompt and available tools
    let mut chat_req = ChatRequest::default()
        .with_system("Answer with one sentence or tool call. Send `exit` to stop.")
        .with_tools(vec![weather_tool, current_time_tool]);

    // Display welcome message to the user
    span!(tracing::Level::INFO, "chat", role = "assistant").in_scope(|| {
        info!("Hi, I'm a weather bot. I can help you with the weather forecast");
        info!("Send `exit` to stop");
    });

    // Main interaction loop - process user requests until 'exit' is received
    let mut buffer = String::new();
    print!("> ");
    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut buffer)?;

    while buffer.trim() != "exit" {
        let user_request = buffer.trim_start_matches('>').trim();

        // Skip empty requests
        if user_request.is_empty() {
            continue;
        }

        // Log user input with appropriate tracing span
        span!(tracing::Level::INFO, "chat", role = "user").in_scope(|| {
            info!(user_request);
        });

        // Add user message to the ongoing conversation
        let chat_message = ChatMessage::user(user_request.to_string());
        chat_req = chat_req.append_message(chat_message);

        // Process the request through the Gemini model
        // This may involve multiple calls if tool usage is required
        chat_req = call_loop(&client, chat_req)
            .instrument(span!(tracing::Level::INFO, "call_loop"))
            .await?;

        // Check if the assistant response is 'exit' to terminate the conversation
        if let Some(last_message) = chat_req.messages.last() {
            if let MessageContent::Text(text) = &last_message.content {
                span!(tracing::Level::INFO, "chat", role = "assistant")
                    .in_scope(|| info!("{}", text));
                if text.as_str() == "exit" {
                    break;
                }
            }
        }

        // Prepare for next user input
        buffer.clear();
        print!("> ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut buffer)?;
    }

    Ok(())
}

/// Continuously make calls to the model until no more tool responses are needed.
///
/// This function handles the complete conversation flow when tools are involved:
/// 1. Makes initial call to the model
/// 2. If the model requests tool calls, executes them
/// 3. Feeds tool responses back to the model
/// 4. Repeats until the model provides a final text response
///
/// This approach allows the model to use tools as needed to fulfill the user request
/// without requiring additional user input during the process.
async fn call_loop(client: &Client, chat_req: ChatRequest) -> Result<ChatRequest, AppError> {
    let mut req = chat_req;

    loop {
        // Make a call to the model and get updated request with response
        req = make_call(client, req).await?;

        // Break the loop if the last message is not a tool response
        // This indicates the model has completed its processing
        if let Some(last_message) = req.messages.last() {
            if !matches!(last_message.content, MessageContent::ToolResponses(_)) {
                break;
            }
        } else {
            // Also break if there are no messages (shouldn't normally happen)
            break;
        }
    }

    Ok(req)
}

/// Make a tool call to the model.
async fn make_tool_call(tool_call: ToolCall) -> ToolResponse {
    info!(
        "Tool call: \n\tFunction: {}\n\tArguments: {}",
        tool_call.fn_name, tool_call.fn_arguments
    );

    // Execute a tool call requested by the model and format the response.
    //
    // Handles two types of tools:
    // - get_weather: Fetches current weather conditions for a location
    // - get_current_time: Fetches current time for a location
    //
    // Returns a properly formatted ToolResponse that will be sent back to the model.
    let tool_response = async {
        // Extract arguments from the tool call
        let args = tool_call.fn_arguments.as_object()
            .ok_or_else(|| AppError::ResponseParseError("Invalid tool call arguments format".to_string()))?;

        match tool_call.fn_name.as_str() {
            // Weather information tool
            "get_weather" => {
                // Extract and validate required parameters
                let city = args
                    .get("city")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::MissingParameter("city".to_string()))?;

                let country = args
                    .get("country")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::MissingParameter("country".to_string()))?;

                let unit = args
                    .get("unit")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::MissingParameter("temperature unit".to_string()))?;

                let location = format!("{},{}", city, country);

                // Call the weather API to get current conditions
                let weather_api_key = env::var("WEATHER_API_KEY")
                    .map_err(|_| AppError::EnvVarNotSet("WEATHER_API_KEY".to_string()))?;
                let weather_response = weather::get_weather(&weather_api_key, &location).await?;

                // Convert temperature to requested unit
                let temperature: f64 = match unit {
                    "C" => weather_response.current.temp_c,
                    "F" => weather_response.current.temp_f,
                    _ => weather_response.current.temp_c,
                };

                // Format the response with relevant weather information
                Ok(ToolResponse::new(
                    tool_call.call_id.clone(),
                    json!({
                        "temperature": temperature,
                        "condition": weather_response.current.condition.text,
                        "humidity": weather_response.current.humidity,
                    })
                    .to_string(),
                ))
            }

            // Time information tool
            "get_current_time" => {
                // Extract and validate required parameters
                let city = args
                    .get("city")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::MissingParameter("city".to_string()))?;

                let country = args
                    .get("country")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::MissingParameter("country".to_string()))?;

                let location = format!("{},{}", city, country);

                // Call the geolocation API to get time information
                let geo_location_api_key = env::var("IP_GEOLOCATION_API_KEY")
                    .map_err(|_| AppError::EnvVarNotSet("IP_GEOLOCATION_API_KEY".to_string()))?;
                let time_response =
                    geo_location::get_time(&geo_location_api_key, &location).await?;

                // Format the response with date and time information
                Ok(ToolResponse::new(
                    tool_call.call_id.clone(),
                    json!({
                        "time": format!("{} {}", time_response.date, time_response.time_12),
                    })
                    .to_string(),
                ))
            }

            // Handle unsupported tool calls
            _ => Err(AppError::UnsupportedToolCall(tool_call.fn_name.clone())),
        }
    }
    .await;

    // Handle successful responses or errors
    match tool_response {
        Ok(tool_response) => tool_response,
        Err(e) => {
            error!("Failed to make tool call: {}", e);
            // Return error information in a format the model can understand
            ToolResponse::new(
                tool_call.call_id.clone(),
                json!({
                    "error": e.to_string(),
                })
                .to_string(),
            )
        }
    }
}

/// Make a call to the Gemini model and process the response.
///
/// This function:
/// 1. Sends the current conversation to the model
/// 2. Processes different types of responses (text or tool calls)
/// 3. For tool calls, executes them in parallel and adds results to conversation
/// 4. Returns the updated conversation context
async fn make_call(client: &Client, chat_req: ChatRequest) -> Result<ChatRequest, AppError> {
    // Send the request to the model and log for debugging
    debug!("Sending request to the model: {:?}", chat_req.messages);
    let response: ChatResponse = client.exec_chat(MODEL, chat_req.clone(), None).await
        .map_err(|e| AppError::ApiRequestFailed(format!("Failed to call Gemini API: {}", e)))?;

    // Process different types of model responses
    let req: ChatRequest = match response.content {
        // Handle simple text responses
        Some(MessageContent::Text(text)) => {
            chat_req.append_message(ChatMessage::assistant(text.trim()))
        }

        // Handle tool call requests from the model
        Some(MessageContent::ToolCalls(tool_calls)) => {
            // First add the model's tool call request to the conversation
            let chat_req = chat_req.append_message(ChatMessage::assistant(
                MessageContent::ToolCalls(tool_calls.clone()),
            ));

            // Execute tool calls in parallel (up to 3 concurrent calls)
            let tool_calls: Vec<ToolResponse> = stream::iter(tool_calls)
                .map(|tool_call| async move { make_tool_call(tool_call).await })
                .buffer_unordered(3)
                .collect::<Vec<ToolResponse>>()
                .await;

            // Log tool call results for debugging
            debug!("Tool calls: {:#?}", tool_calls);

            // Add all tool responses to the conversation
            tool_calls
                .into_iter()
                .fold(chat_req, |chat_req, next| chat_req.append_message(next))
        }

        // Handle unsupported response types
        Some(_) => {
            error!("> Bot: Unsupported response type");
            chat_req.append_message(ChatMessage::assistant("Unsupported response type"))
        }

        // Handle empty responses
        None => {
            error!("> Bot: No response");
            chat_req.append_message(ChatMessage::assistant("No response"))
        }
    };

    Ok(req)
}
