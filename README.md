# Vibi-DPU

Vibi-DPU is an application written in Rust and packaged as a Docker image. It runs on the users' infrastructure to analyze private Intellectual Property data, empowering users through analysis without sacrificing privacy.

The application communicates with our Next.js server, hosted on our infrastructure, which is also open source. You can find it [here](https://github.com/Alokit-Innovations/team-monitor-website/).

Currently, we analyze code in Git repositories, and are soon planning to add APM data and business events. The insights we get from these analyses are communicated through comments/actions on pull requests and through our [open-source Chrome Extension](https://chrome.google.com/webstore/detail/vibinex-code-review/jafgelpkkkopeaefadkdjcmnicgpcncc). 

For more information, visit our website at https://vibinex.com/.

## Setup Instructions

To run Vibi-DPU locally:

1. Generate public url using ngrok - `ngrok http 3000`
2. Fire up cloud sql proxy - `./cloud-sql-proxy --port 5432 vibi-test-394606:asia-south1:test-db`
3. Change url in team-monitor-website in .env.local - `NEXTAUTH_URL=https://example.ngrok-free.app`
4. Start team-monitor-website - `npm run dev`
5. Build vibi-dpu, go to vibi-dpu/vibi-dpu and run - `cargo build`
6. Go up to the root directory of vibi-dpu - `cd ../`
7. **Build the Docker image**: In the root directory of the project, run the following command to build a Docker image with the name "dpu".

    ```bash
    docker build \
      --build-arg GCP_CREDENTIALS=/path/to/your/keyfile.json \
      --build-arg TOPIC_NAME=my-topic-name \
      --build-arg SUBSCRIPTION_NAME=my-subscription-name \
      --build-arg BITBUCKET_CLIENT_ID=your-bitbucket-client-id \
      --build-arg BITBUCKET_CLIENT_SECRET=your-bitbucket-client-secret \
      --build-arg BITBUCKET_BASE_URL=your-bitbucket-base-url \
      --build-arg INSTALL_ID=your-install-id \
      --build-arg SERVER_URL=your-server-url \
      -t dpu .
    ```
8. **Run the Docker container**: After building the image, you can run it using the following command.

    ```bash
    docker run dpu
    ```

## Contributing

We welcome contributions from the community! Please read our contributing guidelines before submitting a pull request.

## License

This project is licensed under the terms of the GNU-GPLv3.