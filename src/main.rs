mod geo_location;
mod weather;

use std::{env, io::Write};

use futures::stream::{self, StreamExt, TryStreamExt};
use genai::{
    Client,
    chat::{ChatMessage, ChatRequest, ChatResponse, MessageContent, Tool, ToolCall, ToolResponse},
};
use serde_json::json;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt::format};

const MODEL: &str = "gemini-2.0-flash";

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
        .with_system("Anwser with one sentense or tool call")
        .with_tools(vec![weather_tool, current_time_tool]);

    println!(
        "> Bot: Hi, I'm a weather bot. I can help you with the weather forecast.\n> Bot: Send exit to stop"
    );

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
        debug!("User: {}", user_request);

        // add user message to the chat request
        let chat_message = ChatMessage::user(user_request.to_string());
        chat_req = chat_req.append_message(chat_message);

        // Send the request to the model
        chat_req = call_loop(&client, chat_req).await?;

        print!("> ");
        std::io::stdout().flush()?;

        buffer.clear();
        std::io::stdin().read_line(&mut buffer)?;
    }

    Ok(())
}

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
async fn make_tool_call(tool_call: ToolCall) -> anyhow::Result<ToolResponse> {
    info!(
        "Tool call: \n\tFunction: {}\n\tArguments: {}",
        tool_call.fn_name, tool_call.fn_arguments
    );

    let tool_response: ToolResponse = if tool_call.fn_name == "get_weather" {
        // todo: check if the arguments are valid
        let args = tool_call.fn_arguments.as_object().unwrap();
        let city = args["city"].as_str().unwrap_or("Unknown");
        let country = args["country"].as_str().unwrap_or("Unknown");
        let unit = args["unit"].as_str().unwrap_or("C");

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

        ToolResponse::new(
            tool_call.call_id.clone(),
            json!({
                "temperature": temperature,
                "condition": weather_response.current.condition.text,
                "humidity": weather_response.current.humidity,
            })
            .to_string(),
        )
    } else if tool_call.fn_name == "get_current_time" {
        // todo: check if the arguments are valid
        let args = tool_call.fn_arguments.as_object().unwrap();
        let city = args["city"].as_str().unwrap_or("Unknown");
        let country = args["country"].as_str().unwrap_or("Unknown");

        let location = format!("{},{}", city, country);

        // Call the get location API
        let geo_location_api_key = env::var("IP_GEOLOCATION_API_KEY")
            .expect("IP_GEOLOCATION_API_KEY environment variable not set");
        let time_response = geo_location::get_time(&geo_location_api_key, &location).await?;

        ToolResponse::new(
            tool_call.call_id.clone(),
            json!({
                "time": format!("{} {}", time_response.date, time_response.time_12),
            })
            .to_string(),
        )
    } else {
        todo!()
    };

    Ok(tool_response)
}

/// Make a call to the model and process the response.
async fn make_call(client: &Client, chat_req: ChatRequest) -> anyhow::Result<ChatRequest> {
    // Send the request to the model
    debug!("Sending request to the model: {:?}", chat_req.messages);
    let response: ChatResponse = client.exec_chat(MODEL, chat_req.clone(), None).await?;

    // Process the response
    let req: ChatRequest = match response.content {
        Some(MessageContent::Text(text)) => {
            println!("> Bot: {}", text);
            chat_req.append_message(ChatMessage::assistant(text))
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
                .try_collect::<Vec<ToolResponse>>()
                .await?;

            tool_calls
                .into_iter()
                .fold(chat_req, |chat_req, next| chat_req.append_message(next))
        }
        Some(_) => {
            println!("> Bot: Unsupported response type");
            chat_req.append_message(ChatMessage::assistant("Unsupported response type"))
        }
        None => {
            println!("> Bot: No response");
            chat_req.append_message(ChatMessage::assistant("No response"))
        }
    };

    Ok(req)
}
