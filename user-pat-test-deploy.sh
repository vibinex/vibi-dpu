#!/bin/bash
set -e

# Ensure variables are set
if [ -z "$PROJECT_ID" ] || [ -z "$SHORT_SHA" ] || [ -z "$_USER_ID" ] || [ -z "$_INSTALL_ID" ] || [ -z "$_GITHUB_PAT" ] || [ -z "$_PROVIDER" ]; then
	echo "Error: Environment variables not set."
	exit 1
fi

# Concatenate variables to create deployment and container names
DEPLOYMENT_NAME="dpu-deployment-${SHORT_SHA}-${_USER_ID}"
CONTAINER_NAME="dpu-container-${SHORT_SHA}-${_USER_ID}"

# Generate the deployment configuration file
cat <<EOF > generated-dpu-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: $DEPLOYMENT_NAME
spec:
  replicas: 1
  selector:
    matchLabels:
      app: dpu
  template:
    metadata:
      labels:
      app: dpu
    spec:
      containers:
      - name: $CONTAINER_NAME
      image: asia-south1-b-docker.pkg.dev/$PROJECT_ID/dpu-test/dpu-test:$SHORT_SHA
      ports:
      - containerPort: 80
      env:
      - name: INSTALL_ID
        value: $_INSTALL_ID
      - name: GITHUB_PAT
        value: $_GITHUB_PAT
      - name: PROVIDER
        value: $_PROVIDER	
EOF

# Display the generated YAML for debugging
echo "Generated YAML:"
cat generated-dpu-deployment.yaml