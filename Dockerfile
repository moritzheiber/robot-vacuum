# syntax=docker/dockerfile
FROM rust:1.65.0-alpine3.16 as builder

WORKDIR /build
ADD . .

RUN --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=private,target=/build/target \
    apk --no-cache add build-base \
    openssl-dev \
    sqlite-dev && \
    cargo build --release && \
    cp target/release/robot-vacuum /robot-vacuum

FROM alpine:3.16

ARG TZ="Europe/Berlin"
ENV TZ="${TZ}"

COPY --from=builder /robot-vacuum /robot-vacuum

RUN apk --no-cache add alpine-conf && \
   setup-timezone -z "${TZ}" && \
   apk del alpine-conf

VOLUME /db

ENV DATABASE_URL="sqlite:///db/sqlite.db?mode=rwc"

EXPOSE 5000/tcp

CMD ["/robot-vacuum"]
