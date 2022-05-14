FROM rust:1.60 AS build
COPY . /SBot
WORKDIR /SBot
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11
COPY --from=build /SBot/target/release/sbot .
CMD ["/sbot"]
