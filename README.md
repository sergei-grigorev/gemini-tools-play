# Experiments and learning Gemini API and Tools

I'm learning LLM tools and store the results here. 

To run the project you need to register on some services and create a file named `.env` with their tokens:
- [ipgeolocation](https://ipgeolocation.io/ip-location-api.html)
- [google gemini](https://gemini.google.com/app)
- [weather](https://www.weatherapi.com)

All that services provide free token you can use to play the same I do.

For convinent work I use [just](https://just.systems) which is similar to Makefile. To run the project make sure you have `.env` file with that format

```text
RUST_LOG="info"
GEMINI_API_KEY="<your gemini token>"
WEATHER_API_KEY="<your weather api>"
IP_GEOLOCATION_API_KEY="<your ip>"
```

Then run `just run`
