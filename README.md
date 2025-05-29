# Cloud Gemini - Learning Project

This project demonstrates the integration of Google's Gemini API with external services for weather and geolocation data. It's designed as a learning tool for understanding LLM tools and their practical applications.

**Note: This project is for learning purposes only and not intended for production use.**

## Project Architecture

The project is built in Rust and follows a modular architecture:

### Core Components

1. **Main Application (`main.rs`)**
   - Initializes the application and sets up the chat loop
   - Defines and registers tools for the Gemini model
   - Handles user input and model responses
   - Manages the chat flow and tool calls

2. **Weather Module (`weather.rs`)**
   - Provides functionality to fetch current weather data
   - Communicates with the WeatherAPI service
   - Returns temperature, condition, and humidity information

3. **Geolocation Module (`geo_location.rs`)**
   - Retrieves current time information for a specified location
   - Communicates with the IPGeolocation API
   - Returns date and time data

### Key Features

- Interactive chat interface with Gemini 2.0 Flash model
- Tool-based architecture for extending model capabilities
- Asynchronous API calls using Tokio and Reqwest
- Structured logging with tracing

## Prerequisites

- Rust toolchain (2024 edition)
- [just](https://just.systems) command runner (optional but recommended)
- API keys for the following services:
  - [Google Gemini](https://gemini.google.com/app)
  - [WeatherAPI](https://www.weatherapi.com)
  - [IPGeolocation](https://ipgeolocation.io/ip-location-api.html)

All these services offer free tiers that are sufficient for experimenting with this project.

## Setup and Configuration

1. Clone the repository
2. Create a `.env` file in the project root with the following content:

```text
RUST_LOG="info"
GEMINI_API_KEY="<your gemini token>"
WEATHER_API_KEY="<your weather api>"
IP_GEOLOCATION_API_KEY="<your ip>"
```

## Building and Running

### Using just

The project includes a `justfile` for common tasks:

```bash
# Run the application
just run

# Format the code
just fmt

# Run tests
just test

# Run linter
just clippy

# Clean build artifacts
just clean
```

### Using Cargo directly

```bash
# Run the application
cargo run

# Build in release mode
cargo build --release
```

## Usage

Once running, the application provides a simple chat interface. You can:

1. Ask about the weather in a specific location
2. Request the current time for a location
3. Type `exit` to quit the application

The Gemini model will automatically determine when to use the appropriate tools based on your queries.
