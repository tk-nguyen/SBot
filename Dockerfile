FROM rust:1.60 AS build
COPY . /SBot
WORKDIR /SBot
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11
# We use s6 init as pid 1
ARG S6_OVERLAY_VERSION=3.1.0.1

COPY --from=build /SBot/target/release/sbot .
CMD ["/sbot"]

ADD https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz /tmp
RUN tar -C / -Jxpf /tmp/s6-overlay-noarch.tar.xz
ADD https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-x86_64.tar.xz /tmp
RUN tar -C / -Jxpf /tmp/s6-overlay-x86_64.tar.xz
ENTRYPOINT ["/init"]
