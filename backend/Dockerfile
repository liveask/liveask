FROM debian:bookworm-slim
LABEL maintainer="extrawurst"
RUN apt-get update \
    && apt-get install -y pkg-config libssl-dev ca-certificates curl libfontconfig

ADD linux_bin/server /app/server

ENV APP_USER=app-user
RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && chown -R $APP_USER:$APP_USER /app

WORKDIR /app

CMD ["./server"]
