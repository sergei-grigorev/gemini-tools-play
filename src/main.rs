use std::io::Write;

use genai::{
    Client,
    chat::{ChatMessage, ChatRequest, ChatResponse, MessageContent, Tool, ToolResponse},
};
use serde_json::json;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gemini-2.0-flash";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut chat_req = ChatRequest::default()
        .with_system("Anwser with one sentense or tool call")
        .with_tools(vec![weather_tool]);

    println!(
        "> Bot: Hi, I'm a weather bot. I can help you with the weather forecast.\n> Bot: Send exit to stop"
    );

    // read user requests until it sends `exit`
    let mut user_request = String::new();
    while user_request.trim() != "exit" {
        user_request.clear();

        print!("> ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut user_request)?;
        let user_request = user_request.trim_start_matches('>').trim();

        if user_request.is_empty() {
            continue;
        }

        // Create a chat message with the user's input
        debug!("User: {}", user_request);

        // add user message to the chat request
        let chat_message = ChatMessage::user(user_request.to_string());
        chat_req = chat_req.append_message(chat_message);

        // Send the request to the model
        chat_req = make_call(&client, chat_req).await?;

        // check if the last message is a tool call
        if let Some(last_message) = chat_req.messages.last() {
            if let MessageContent::ToolResponses(_) = last_message.content {
                // make another call to the model
                debug!("Tool call response detected, making another call to the model");
                chat_req = make_call(&client, chat_req).await?;
            }
        }
    }

    Ok(())
}

async fn make_call(
    client: &Client,
    chat_req: ChatRequest,
) -> Result<ChatRequest, Box<dyn std::error::Error>> {
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
            // debugging output
            for tool_call in &tool_calls {
                info!(
                    "Tool call: \n\tFunction: {}\n\tArguments: {}",
                    tool_call.fn_name, tool_call.fn_arguments
                );
            }

            let first_tool_call = &tool_calls[0];
            let tool_response = ToolResponse::new(
                first_tool_call.call_id.clone(),
                json!({
                    "temperature": 22.5,
                    "condition": "Sunny",
                    "humidity": 65
                })
                .to_string(),
            );

            chat_req
                .append_message(ChatMessage::assistant(MessageContent::ToolCalls(
                    tool_calls,
                )))
                .append_message(tool_response)
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
