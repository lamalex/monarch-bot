# ------------------------------------------------------------------------------
# Cargo Build Stage
# ------------------------------------------------------------------------------
FROM rust:1.45.2 as cargo-build
RUN USER=root cargo new --bin monarchy_mcmonarch_bot
WORKDIR ./monarchy_mcmonarch_bot
RUN USER=root cargo new --bin mcmonarch
RUN USER=root cargo new --lib mcmonarch_web
RUN USER=root cargo new --lib mcmonarch_bot
COPY ./Cargo.toml ./Cargo.toml
COPY ./mcmonarch/Cargo.toml ./mcmonarch/Cargo.toml
COPY ./mcmonarch_web/Cargo.toml ./mcmonarch_web/Cargo.toml
COPY ./mcmonarch_bot/Cargo.toml ./mcmonarch_bot/Cargo.toml

RUN cargo build --release
RUN rm src/*.rs mcmonarch/src/*.rs mcmonarch_web/src/*.rs mcmonarch_bot/src/*.rs

ADD . ./
RUN rm ./target/release/deps/monarchy_mcmonarch_bot*

RUN cargo build --release

# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------
FROM ubuntu:20.04
ARG APP=/usr/src/monarchy_mcmonarch_bot

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 3030

ENV TZ=Etc/UTC \
    APP_USER=mmbf

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP} \
    && mkdir -p ${APP}/static

COPY --from=cargo-build /monarchy_mcmonarch_bot/target/release/monarchy-mcmonarch-bot ${APP}/monarchy-mcmonarch-bot
COPY --from=cargo-build /monarchy_mcmonarch_bot/mcmonarch_web/static/* ${APP}/static/
RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./monarchy-mcmonarch-bot"]
