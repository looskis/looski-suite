FROM debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251 AS download
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /tmp
RUN case "$TARGETARCH" in \
      arm64) checksum=6b8dc4333c53a5a57c9e5834e3a48f92605d7154014cd07269ff3327db5d37f4 ;; \
      amd64) checksum=cbe24006683f8eb669266162894b9a522a1af52f2665fbc63a4bb032ed26ac10 ;; \
      *) exit 1 ;; \
    esac \
    && curl -fsSL "https://github.com/FiloSottile/age/releases/download/v1.3.2/age-v1.3.2-linux-${TARGETARCH}.tar.gz" -o age.tar.gz \
    && echo "$checksum  age.tar.gz" | sha256sum -c - \
    && tar -xzf age.tar.gz
FROM debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251
COPY --from=download /tmp/age/age /usr/local/bin/age
COPY --from=download /tmp/age/age-keygen /usr/local/bin/age-keygen
ENTRYPOINT ["age"]
