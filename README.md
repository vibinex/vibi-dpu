# Vibi-DPU

Vibi-DPU is an application written in Rust and packaged as a Docker image. It runs on the users' infrastructure to analyze private Intellectual Property data, empowering users through analysis without sacrificing privacy.

The application communicates with our Next.js server, hosted on our infrastructure, which is also open source. You can find it [here](https://github.com/Alokit-Innovations/team-monitor-website/).

Currently, we analyze code in Git repositories, and are soon planning to add APM data and business events. The insights we get from these analyses are communicated through comments/actions on pull requests and through our [open-source Chrome Extension](https://chrome.google.com/webstore/detail/vibinex-code-review/jafgelpkkkopeaefadkdjcmnicgpcncc). 

For more information, visit our website at https://vibinex.com/.

## Setup Instructions

To run Vibi-DPU locally:

1. Generate public url using ngrok - `ngrok http 3000`. We will run our next server locally on port 3000 in later steps.
2. Paste this in OAuth consumers in callback_url field.
3. Clone [team-monitor-webiste](https://github.com/Alokit-Innovations/team-monitor-website/) locally.
4. Paste the client id and secret in team-monitor-wesite in .env.local in root directory. Also use them in the docker command below.
5. Fire up cloud sql proxy - `./cloud-sql-proxy --port 5432 vibi-test-394606:asia-south1:test-db`
6. Change url in team-monitor-website in .env.local - `NEXTAUTH_URL=https://example.ngrok-free.app`
7. Start team-monitor-website - `npm run dev`
8. Build vibi-dpu, go to vibi-dpu/vibi-dpu and run - `cargo build`
9. Go up to the root directory of vibi-dpu - `cd ../`
10. **Build the Docker image**: In the root directory of the project, run the following command to build a Docker image with the name "dpu".

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
11. **Run the Docker container**: After building the image, you can run it using the following command.

    ```bash
    docker run dpu
    ```
12. For bitbucket, replace your url in this url and paste it on your browser and visit it. If you are using ngrok, you might get a "visit site" ngrok welcome page. Click and visit site. Grant any permissions asked from your user to bitbucket. Example URL - `https://bitbucket.org/site/oauth2/authorize?response_type=code&client_id=raFykYJRvEBHPttQAm&redirect_uri=https%3A%2F%2F5bef-171-76-86-89.ngrok-free.app%2Fapi%2Fbitbucket%2Fcallbacks%2Finstall&scope=repository%20pullrequest%20pullrequest:write%20webhook%20account%20repository:write`. You only need to replace the `5bef-171-76-86-89.ngrok-free.app` part with your own ngrok url instead of generating a new formatted url.
13. This would start the "setting up" part of dpu, where it calls bitbucket apis and collects repo info, user info, workspace info and pr info.
14. Next begin your testing. For instance, if you push to a PR, you should be able to see logs in next server, in dpu and see the required actions being performed on the PR.

## Contributing

We welcome contributions from the community! Please read our contributing guidelines before submitting a pull request.

## License

This project is licensed under the terms of the GNU-GPLv3.