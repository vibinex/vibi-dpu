#!/bin/bash
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
        image: asia-south1-docker.pkg.dev/$PROJECT_ID/dpu-test/dpu-test:$SHORT_SHA
        ports:
        - containerPort: 80
        env:
        - name: INSTALL_ID
          value: $_INSTALL_ID
EOF
