FROM rust:1.62.1 as builder

WORKDIR /mstress
RUN cargo init

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

# build for release
RUN rm ./target/release/deps/mstress*
RUN cargo build --release

FROM rust:1.62.1
COPY --from=builder /mstress/target/release/mstress .

EXPOSE 8080
CMD ["./mstress"]
