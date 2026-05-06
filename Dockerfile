ARG RUST_VERSION=1.91.1

FROM rust:${RUST_VERSION}-alpine AS build
WORKDIR /app

RUN apk add --no-cache clang lld musl-dev git

COPY src src
COPY Cargo.toml ./
COPY Cargo.lock ./
RUN cargo build --locked --release

FROM alpine:3.23.4 AS final

# Copy the executable from the "build" stage.
COPY --from=build /app/target/release/ctg /bin/

# What the container should run when it is started.
ENTRYPOINT /bin/ctg
