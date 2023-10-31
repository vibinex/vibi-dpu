# Use a lightweight Linux distribution as the base image
FROM ubuntu:latest

# # Install dependencies required by the application
RUN \
  apt-get update && \
  apt-get install ca-certificates git -y && \
  apt-get clean
ADD http://archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2_amd64.deb /tmp
RUN chmod a+x /tmp/libssl1.1_1.1.1f-1ubuntu2_amd64.deb && \
    apt-get install /tmp/libssl1.1_1.1.1f-1ubuntu2_amd64.deb -y && \
    rm -rf /tmp/*.deb

ARG GCP_CREDENTIALS
ARG TOPIC_NAME 
ARG SUBSCRIPTION_NAME
ARG BITBUCKET_CLIENT_ID
ARG BITBUCKET_CLIENT_SECRET
ARG BITBUCKET_BASE_URL
ARG INSTALL_ID
ARG SERVER_URL


ENV GCP_CREDENTIALS=$GCP_CREDENTIALS  
ENV TOPIC_NAME=$TOPIC_NAME
ENV SUBSCRIPTION_NAME=$SUBSCRIPTION_NAME
ENV BITBUCKET_CLIENT_ID=$BITBUCKET_CLIENT_ID
ENV BITBUCKET_CLIENT_SECRET=$BITBUCKET_CLIENT_SECRET
ENV BITBUCKET_BASE_URL=$BITBUCKET_BASE_URL
ENV INSTALL_ID=$INSTALL_ID
ENV SERVER_URL=$SERVER_URL

COPY ./vibi-dpu/target/debug/vibi-dpu /app/vibi-dpu
COPY ./pubsub-sa.json /app/pubsub-sa.json
COPY ./repo-profiler.pem /app/repo-profiler.pem

# Start the Rust application
CMD ["/app/vibi-dpu"]
