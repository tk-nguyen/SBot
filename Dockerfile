FROM --platform=$BUILDPLATFORM rust:1.61 AS build
ARG TARGETPLATFORM
ARG BUILDPLATFORM
COPY . /SBot
WORKDIR /SBot
RUN case $TARGETPLATFORM in \
    linux/amd64) \
    rustup target add x86_64-unknown-linux-gnu && \
    cargo build --target=x86_64-unknown-linux-gnu --release && \
    cp target/x86_64-unknown-linux-gnu/release/sbot . \
    ;; \
    linux/arm64) \
    apt update && apt install -y crossbuild-essential-arm64 && \
    rustup target add aarch64-unknown-linux-gnu && \
    cargo build --target=aarch64-unknown-linux-gnu --release && \
    cp target/aarch64-unknown-linux-gnu/release/sbot . \
    ;; \
    esac

FROM --platform=$TARGETPLATFORM gcr.io/distroless/cc-debian11
LABEL org.opencontainers.image.authors="Nguyen Thai <shiroemon279@gmail.com>"
COPY --from=build /SBot/sbot .
CMD ["/sbot"]
