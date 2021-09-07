# Build Image
FROM rust:1.54-alpine AS builder

RUN apk add musl-dev

WORKDIR /usr/src/howareyou
COPY . .
# Download the package separate to increase build time when rebuild
RUN cargo fetch
# Build the binary
RUN cargo install --path .

# Runtime Image
FROM alpine:3.14
COPY --from=builder /usr/local/cargo/bin/howareyou /usr/local/bin/howareyou
CMD ["howareyou"]