mod geo_location;
mod weather;

use std::{env, io::Write};

use futures::stream::{self, StreamExt};
use genai::{
    Client,
    chat::{ChatMessage, ChatRequest, ChatResponse, MessageContent, Tool, ToolCall, ToolResponse},
};
use serde_json::json;
use tracing::{Instrument, debug, error, info, span};
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gemini-2.0-flash";

/// The main function initializes the tracing subscriber, sets up chat tools for fetching weather
/// and time information, and enters a loop to process user requests. The bot can handle requests
/// for current weather and time, using predefined tools, and will continue until the user inputs "exit".
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let client = Client::default();

    // Define a tool for getting weather information
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

    let mut chat_req = ChatRequest::default()
        .with_system("Anwser with one sentense or tool call. Send `exit` to stop.")
        .with_tools(vec![weather_tool, current_time_tool]);

    span!(tracing::Level::INFO, "chat", role = "assistant").in_scope(|| {
        info!("Hi, I'm a weather bot. I can help you with the weather forecast");
        info!("Send `exit` to stop");
    });

    // read user requests until it sends `exit`
    let mut buffer = String::new();
    print!("> ");
    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut buffer)?;

    while buffer.trim() != "exit" {
        let user_request = buffer.trim_start_matches('>').trim();

        if user_request.is_empty() {
            continue;
        }

        // Create a chat message with the user's input
        span!(tracing::Level::INFO, "chat", role = "user").in_scope(|| {
            info!(user_request);
        });

        // add user message to the chat request
        let chat_message = ChatMessage::user(user_request.to_string());
        chat_req = chat_req.append_message(chat_message);

        // Send the request to the model
        chat_req = call_loop(&client, chat_req)
            .instrument(span!(tracing::Level::INFO, "call_loop"))
            .await?;

        // check if the last message is `exit` (that means the user wants to stop the chat)
        if let Some(last_message) = chat_req.messages.last() {
            if let MessageContent::Text(text) = &last_message.content {
                span!(tracing::Level::INFO, "chat", role = "assistant")
                    .in_scope(|| info!("{}", text));
                if text.as_str() == "exit" {
                    info!("User wants to exit");
                    break;
                }
            }
        }

        print!("> ");
        std::io::stdout().flush()?;

        buffer.clear();
        std::io::stdin().read_line(&mut buffer)?;
    }

    Ok(())
}

/// Continuously make calls to the model until no more tool responses are returned.
///
/// The function takes a client and a chat request as arguments and makes a call to the model
/// using the `make_call` function. If the last message in the response is a tool response,
/// the function makes another call to the model, otherwise it breaks the loop and returns
/// the chat request.
///
/// This function is used to continuously ask the model for more information until the user
/// does not need any more information.
async fn call_loop(client: &Client, chat_req: ChatRequest) -> anyhow::Result<ChatRequest> {
    let mut chat_req = make_call(client, chat_req).await?;
    while let Some(last_message) = chat_req.messages.last() {
        if let MessageContent::ToolResponses(_) = last_message.content {
            // make another call to the model
            debug!("Tool call response detected, making another call to the model");
            chat_req = make_call(client, chat_req).await?;
        } else {
            break;
        }
    }

    Ok(chat_req)
}

/// Make a tool call to the model.
async fn make_tool_call(tool_call: ToolCall) -> ToolResponse {
    info!(
        "Tool call: \n\tFunction: {}\n\tArguments: {}",
        tool_call.fn_name, tool_call.fn_arguments
    );

    // make the tool call
    let tool_response: anyhow::Result<ToolResponse> = async {
        if tool_call.fn_name == "get_weather" {
            let args = tool_call.fn_arguments.as_object().unwrap();

            // all parameters should be present
            let city = args
                .get("city")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("City is not presented"))?;

            let country = args
                .get("country")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Country is not presented"))?;

            let unit = args
                .get("unit")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Unit is not presented"))?;

            let location = format!("{},{}", city, country);

            // Call the weather API
            let weather_api_key =
                env::var("WEATHER_API_KEY").expect("WEATHER_API_KEY environment variable not set");
            let weather_response = weather::get_weather(&weather_api_key, &location).await?;

            let temperature: f64 = match unit {
                "C" => weather_response.current.temp_c,
                "F" => weather_response.current.temp_f,
                _ => weather_response.current.temp_c,
            };

            Ok(ToolResponse::new(
                tool_call.call_id.clone(),
                json!({
                    "temperature": temperature,
                    "condition": weather_response.current.condition.text,
                    "humidity": weather_response.current.humidity,
                })
                .to_string(),
            ))
        } else if tool_call.fn_name == "get_current_time" {
            let args = tool_call.fn_arguments.as_object().unwrap();
            let city = args
                .get("city")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("City is not presented"))?;

            let country = args
                .get("country")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Country is not presented"))?;

            let location = format!("{},{}", city, country);

            // Call the get location API
            let geo_location_api_key = env::var("IP_GEOLOCATION_API_KEY")
                .expect("IP_GEOLOCATION_API_KEY environment variable not set");
            let time_response = geo_location::get_time(&geo_location_api_key, &location).await?;

            Ok(ToolResponse::new(
                tool_call.call_id.clone(),
                json!({
                    "time": format!("{} {}", time_response.date, time_response.time_12),
                })
                .to_string(),
            ))
        } else {
            Err(anyhow::anyhow!("Tool call function not implemented"))
        }
    }
    .await;

    match tool_response {
        Ok(tool_response) => tool_response,
        Err(e) => {
            error!("Failed to make tool call: {}", e);
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

/// Make a call to the model and process the response.
async fn make_call(client: &Client, chat_req: ChatRequest) -> anyhow::Result<ChatRequest> {
    // Send the request to the model
    debug!("Sending request to the model: {:?}", chat_req.messages);
    let response: ChatResponse = client.exec_chat(MODEL, chat_req.clone(), None).await?;

    // Process the response
    let req: ChatRequest = match response.content {
        Some(MessageContent::Text(text)) => {
            chat_req.append_message(ChatMessage::assistant(text.trim()))
        }
        Some(MessageContent::ToolCalls(tool_calls)) => {
            // remember the tool calls to append them to the chat request
            let chat_req = chat_req.append_message(ChatMessage::assistant(
                MessageContent::ToolCalls(tool_calls.clone()),
            ));

            // make the tool calls
            let tool_calls: Vec<ToolResponse> = stream::iter(tool_calls)
                .map(|tool_call| async move { make_tool_call(tool_call).await })
                .buffer_unordered(3)
                .collect::<Vec<ToolResponse>>()
                .await;

            // log tool calls
            debug!("Tool calls: {:#?}", tool_calls);

            tool_calls
                .into_iter()
                .fold(chat_req, |chat_req, next| chat_req.append_message(next))
        }
        Some(_) => {
            error!("> Bot: Unsupported response type");
            chat_req.append_message(ChatMessage::assistant("Unsupported response type"))
        }
        None => {
            error!("> Bot: No response");
            chat_req.append_message(ChatMessage::assistant("No response"))
        }
    };

    Ok(req)
}
