FROM rust:latest

WORKDIR /app

# Copy files
COPY ./http ./http
COPY ./src ./src
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build -r

EXPOSE 8080

CMD ["cargo" , "run", "-r"]
