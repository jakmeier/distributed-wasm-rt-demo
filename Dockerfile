# Docker image for the signaling server.
#
# Deployed to https://hub.docker.com/r/jakmeier/demo/tags?page=1&name=ntmy
#
# Currently only buster is supported on the Jelastic platform:
# https://www.virtuozzo.com/application-platform-docs/container-image-requirements/
FROM rust:1.73-buster

WORKDIR /usr/src/ntmy-signaling
COPY ./ntmy ./ntmy
COPY ./webrtc-signaling-server ./webrtc-signaling-server

RUN cargo install --path ./webrtc-signaling-server

ENV RUST_LOG warn

CMD ["webrtc-signaling-server"]