FROM debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251
ARG TARGETARCH
ARG SUPABASE_CLI_VERSION=2.75.0
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl util-linux \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /tmp/cli
RUN curl -fsSLO "https://github.com/supabase/cli/releases/download/v${SUPABASE_CLI_VERSION}/supabase_linux_${TARGETARCH}.tar.gz" \
    && curl -fsSLO "https://github.com/supabase/cli/releases/download/v${SUPABASE_CLI_VERSION}/supabase_${SUPABASE_CLI_VERSION}_checksums.txt" \
    && grep " supabase_linux_${TARGETARCH}.tar.gz$" "supabase_${SUPABASE_CLI_VERSION}_checksums.txt" | sha256sum -c - \
    && tar -xzf "supabase_linux_${TARGETARCH}.tar.gz" -C /usr/local/bin supabase \
    && rm -rf /tmp/cli
WORKDIR /work
COPY infra/migrator/run.sh /usr/local/bin/migrate
ENTRYPOINT ["/bin/sh", "/usr/local/bin/migrate"]
