# Use a lightweight Linux distribution as the base image
FROM ubuntu:latest

# # Install dependencies required by the application
RUN \
  apt-get update && \
  apt-get install -y ca-certificates git ripgrep libssl3 && \
  apt-get clean

# Run the DPU with a stable, unprivileged identity. The fixed IDs let operators
# grant write access to bind-mounted configuration without using root.
RUN groupadd --gid 10001 dpu && \
  useradd --uid 10001 --gid dpu --home-dir /app --no-create-home --shell /usr/sbin/nologin dpu

ARG GCP_CREDENTIALS
ARG TOPIC_NAME 
ARG SUBSCRIPTION_NAME
ARG DPU_QUEUE_TRANSPORT
ARG DPU_POLL_INTERVAL_MS
ARG DPU_JOB_LEASE_SECONDS
ARG BITBUCKET_CLIENT_ID
ARG BITBUCKET_CLIENT_SECRET
ARG BITBUCKET_BASE_URL
ARG INSTALL_ID
ARG SERVER_URL
ARG GITHUB_APP_ID
ARG GITHUB_APP_CLIENT_ID
ARG GITHUB_APP_CLIENT_SECRET
ARG GITHUB_BASE_URL
ARG GITHUB_PAT
ARG PROVIDER


ENV GCP_CREDENTIALS=$GCP_CREDENTIALS  
ENV TOPIC_NAME=$TOPIC_NAME
ENV SUBSCRIPTION_NAME=$SUBSCRIPTION_NAME
ENV DPU_QUEUE_TRANSPORT=$DPU_QUEUE_TRANSPORT
ENV DPU_POLL_INTERVAL_MS=$DPU_POLL_INTERVAL_MS
ENV DPU_JOB_LEASE_SECONDS=$DPU_JOB_LEASE_SECONDS
ENV BITBUCKET_CLIENT_ID=$BITBUCKET_CLIENT_ID
ENV BITBUCKET_CLIENT_SECRET=$BITBUCKET_CLIENT_SECRET
ENV BITBUCKET_BASE_URL=$BITBUCKET_BASE_URL
ENV INSTALL_ID=$INSTALL_ID
ENV SERVER_URL=$SERVER_URL
ENV GITHUB_APP_ID=$GITHUB_APP_ID
ENV GITHUB_APP_CLIENT_ID=$GITHUB_APP_CLIENT_ID
ENV GITHUB_APP_CLIENT_SECRET=$GITHUB_APP_CLIENT_SECRET
ENV GITHUB_BASE_URL=$GITHUB_BASE_URL
ENV GITHUB_PAT=$GITHUB_PAT
ENV PROVIDER=$PROVIDER

COPY --chown=dpu:dpu ./vibi-dpu/target/release/vibi-dpu /app/vibi-dpu
COPY --chown=dpu:dpu ./prompts /app/prompts

# The DPU persists credentials under /app/config and writes rotated logs under
# /var/log/dpu. Repository checkouts and the embedded database use /tmp.
RUN mkdir -p /app/config /var/log/dpu && \
  chown -R dpu:dpu /app /var/log/dpu

USER dpu:dpu

# Start the Rust application
CMD ["/app/vibi-dpu"]
