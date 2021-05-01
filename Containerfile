FROM rust:1.51 as builder

COPY Makefile .
RUN OUT_DIR=/build make valor
COPY . .
RUN OUT_DIR=/build make plugins

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/valor /usr/local/bin/
COPY --from=builder /build/plugins/* /usr/local/lib/
RUN ldconfig

ENTRYPOINT ["valor"]
CMD ["-p", "/plugins.json"]
