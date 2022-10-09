FROM rust as build
RUN rustc --version
COPY . .
RUN cd backend-e2e && cargo t --no-run
RUN find ./target/debug/deps -perm -111 -type f | grep backend_tests | xargs -I {} mv {} ./test

FROM debian
RUN apt-get update \
    && apt-get install -y openssl ca-certificates curl wget

# Dockerize is needed to sync containers startup
ENV DOCKERIZE_VERSION v0.6.0
RUN wget https://github.com/jwilder/dockerize/releases/download/$DOCKERIZE_VERSION/dockerize-alpine-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && tar -C /usr/local/bin -xzvf dockerize-alpine-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && rm dockerize-alpine-linux-amd64-$DOCKERIZE_VERSION.tar.gz

COPY --from=build ./test ./

CMD ["./test"]
