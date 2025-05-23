set dotenv-load

default:
    just --list

# Clean the artifacts
clean:
    cargo clean

# Format the code
fmt:
    cargo fmt

# Test the code
test:
    cargo test

# Run linter
lint:
    cargo clippy

# run the code
run:
    cargo run
