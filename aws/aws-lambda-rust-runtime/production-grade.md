---
title: "Production-Grade Lambda Rust Runtime"
subtitle: "Deployment, monitoring, and optimization for production Rust Lambda functions"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-rust-runtime/production-grade.md
related: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-rust-runtime/exploration.md
---

# Production-Grade Lambda Rust Runtime

## Introduction

This document covers production deployment strategies for Rust Lambda functions including Cargo Lambda, monitoring, CI/CD, and cost optimization.

---

## Part 1: Cargo Lambda Deployment

### Installation

```bash
# Using pip
pip install cargo-lambda

# Using Homebrew (macOS)
brew install cargo-lambda

# Using uv
uv tool install cargo-lambda

# From source
cargo install cargo-lambda
```

### Creating a New Function

```bash
# Create new function from template
cargo lambda new my-function

# Create with specific template
cargo lambda new my-function --template http
cargo lambda new my-function --template sqs
cargo lambda new my-function --template scheduler
```

### Building for Deployment

```bash
# Build for current architecture
cargo lambda build --release

# Build for ARM64 (Graviton2)
cargo lambda build --release --arm64

# Build for x86_64
cargo lambda build --release --x86-64

# Build specific binary
cargo lambda build --release --bin my-function
```

### Deploying

```bash
# Deploy function
cargo lambda deploy

# Deploy with options
cargo lambda deploy \
  --function-name my-function \
  --memory-size 512 \
  --timeout 30 \
  --env-var LOG_LEVEL=debug

# Deploy with layers
cargo lambda deploy --layer arn:aws:lambda:us-east-1:123456789012:layer:my-layer:1
```

### Local Testing

```bash
# Start local server
cargo lambda watch

# Invoke function locally
curl -X POST http://localhost:9000/invoke \
  -H "Content-Type: application/json" \
  -d '{"firstName": "World"}'

# Invoke with event file
cargo lambda invoke --data-file events/api-gateway.json
```

---

## Part 2: Infrastructure as Code

### Terraform

```hcl
provider "aws" {
  region = "us-east-1"
}

resource "aws_lambda_function" "rust_function" {
  filename      = "bootstrap.zip"
  function_name = "my-rust-function"
  role          = aws_iam_role.lambda_role.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  timeout       = 30
  memory_size   = 512

  environment {
    variables = {
      RUST_LOG = "info"
      DATABASE_URL = var.database_url
    }
  }

  tracing_config {
    mode = "Active"
  }
}

resource "aws_iam_role" "lambda_role" {
  name = "lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
    }]
  })
}

resource "aws_lambda_function_url" "function_url" {
  function_name      = aws_lambda_function.rust_function.function_name
  authorization_type = "AWS_IAM"

  cors {
    allow_origins = ["*"]
    allow_methods = ["*"]
    allow_headers = ["*"]
  }
}
```

### AWS SAM

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Globals:
  Function:
    Timeout: 30
    MemorySize: 512
    Runtime: provided.al2023

Resources:
  RustFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: .
      Handler: bootstrap
      Architecture: arm64
      Environment:
        Variables:
          RUST_LOG: info
      Events:
        ApiEvent:
          Type: HttpApi
          Properties:
            Path: /{proxy+}
            Method: ANY

Outputs:
  ApiUrl:
    Description: API Gateway URL
    Value: !Sub 'https://${ServerlessHttpApi}.execute-api.${AWS::Region}.amazonaws.com'
```

### Pulumi (TypeScript)

```typescript
import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";

const role = new aws.iam.Role("lambda-role", {
  assumeRolePolicy: JSON.stringify({
    Version: "2012-10-17",
    Statement: [{
      Action: "sts:AssumeRole",
      Principal: { Service: "lambda.amazonaws.com" },
      Effect: "Allow",
    }],
  }),
});

const function_ = new aws.lambda.Function("rust-function", {
  code: new pulumi.asset.FileArchive("./bootstrap.zip"),
  role: role.arn,
  handler: "bootstrap",
  runtime: "provided.al2023",
  architecture: "arm64",
  timeout: 30,
  memorySize: 512,
  environment: {
    variables: {
      RUST_LOG: "info",
    },
  },
});

const url = new aws.lambda.FunctionUrl("function-url", {
  functionName: function_.name,
  authorizationType: "AWS_IAM",
});
```

---

## Part 3: CI/CD Pipeline

### GitHub Actions

```yaml
name: Deploy Lambda

on:
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-action@stable
      with:
        targets: aarch64-unknown-linux-gnu

    - name: Install cargo-lambda
      run: pip install cargo-lambda

    - name: Build
      run: cargo lambda build --release --arm64

    - name: Configure AWS Credentials
      uses: aws-actions/configure-aws-credentials@v4
      with:
        aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
        aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        aws-region: us-east-1

    - name: Deploy
      run: cargo lambda deploy
```

### GitLab CI

```yaml
stages:
  - test
  - build
  - deploy

variables:
  CARGO_HOME: "${CI_PROJECT_DIR}/.cargo"

test:
  stage: test
  image: rust:latest
  script:
    - cargo test

build:
  stage: build
  image: rust:latest
  script:
    - rustup target add aarch64-unknown-linux-gnu
    - pip install cargo-lambda
    - cargo lambda build --release --arm64
  artifacts:
    paths:
      - target/lambda/*/bootstrap

deploy:
  stage: deploy
  image: amazon/aws-cli
  script:
    - pip install cargo-lambda
    - cargo lambda deploy
  only:
    - main
```

---

## Part 4: Monitoring and Observability

### Structured Logging

```rust
use lambda_runtime::{tracing, Error, LambdaEvent};
use serde_json::Value;
use tracing_subscriber::fmt::format::JsonFields;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize JSON logging for CloudWatch Insights
    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .with_thread_ids(false)
        .init();

    let func = service_fn(handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

#[tracing::instrument(skip(event), fields(request_id = %context.aws_request_id))]
async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    tracing::info!("Processing request");

    let result = process(event.payload).await?;

    tracing::info!(
        target: "metrics",
        status = "success",
        duration_ms = 100,
        "Request completed"
    );

    Ok(result)
}
```

### Custom CloudWatch Metrics

```rust
use aws_sdk_cloudwatch::Client as CloudWatchClient;

async fn put_metric(
    client: &CloudWatchClient,
    name: &str,
    value: f64,
    unit: &str,
) -> Result<(), Error> {
    client.put_metric_data()
        .namespace("MyApp")
        .metric_data(
            aws_sdk_cloudwatch::types::MetricDatum::builder()
                .metric_name(name)
                .value(value)
                .unit(unit)
                .build()
        )
        .send()
        .await?;
    Ok(())
}

// Usage in handler
put_metric(&cw_client, "RequestLatency", latency_ms, "Milliseconds").await?;
```

### X-Ray Tracing

```rust
use lambda_runtime::{tracing, LambdaEvent, Error};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry::global;

#[tracing::instrument(name = "handler", skip(event))]
async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Add custom annotations
    tracing::Span::current().record("event_type", "api_gateway");

    // Create child spans
    let fetch_span = tracing::info_span!("fetch_data");
    let _fetch_guard = fetch_span.enter();
    let data = fetch_data().await?;

    // Process
    let result = process_data(data).await?;

    Ok(result)
}
```

---

## Part 5: Security Best Practices

### IAM Least Privilege

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": "arn:aws:logs:*:*:*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "xray:PutTraceSegments",
        "xray:PutTelemetryRecords"
      ],
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject"
      ],
      "Resource": "arn:aws:s3:::my-bucket/*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "ssm:GetParameter"
      ],
      "Resource": "arn:aws:ssm:*:*:parameter/my-app/*"
    }
  ]
}
```

### Secrets Management

```rust
use aws_sdk_secretsmanager::Client as SecretsManagerClient;

async fn get_secret(client: &SecretsManagerClient, name: &str) -> Result<String, Error> {
    let response = client.get_secret_value()
        .secret_id(name)
        .send()
        .await?;

    Ok(response.secret_string().unwrap().to_string())
}

// Cache secret in static
static DB_PASSWORD: OnceCell<String> = OnceCell::new();

fn get_db_password() -> &'static str {
    DB_PASSWORD.get().map(|s| s.as_str()).unwrap()
}
```

### VPC Configuration

```hcl
resource "aws_lambda_function" "rust_function" {
  # ... other config

  vpc_config {
    subnet_ids         = aws_subnet.private[*].id
    security_group_ids = [aws_security_group.lambda.id]
  }
}

resource "aws_security_group" "lambda" {
  egress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
```

---

## Part 6: Cost Optimization

### Right-Sizing Memory

```
Memory vs Cost (us-east-1):

128 MB  - $0.0000021 per GB-second
256 MB  - Minimum for most Rust apps
512 MB  - Recommended starting point
1024 MB - For CPU-intensive workloads

Rule: More memory = faster CPU = potentially lower cost
```

### ARM64 Benefits

```bash
# Build for ARM64
cargo lambda build --release --arm64

# ARM64 is 20% cheaper than x86_64
# Also typically 10-20% faster for Rust
```

### Request Batching

```rust
// Instead of:
//   1000 invocations @ 100ms each = 100 seconds

// Batch process:
//   100 invocations @ 500ms each = 50 seconds

async fn batch_handler(event: LambdaEvent<BatchEvent>) -> Result<Value, Error> {
    let items = event.payload.items;
    let results = futures::future::join_all(
        items.into_iter().map(process_item)
    ).await;
    Ok(json!({ "processed": results.len() }))
}
```

---

## Part 7: Testing Strategies

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_handler_success() {
        let event = json!({ "firstName": "Test" });
        let result = handler_sync(event).unwrap();
        assert_eq!(result["message"], "Hello, Test!");
    }

    #[test]
    fn test_handler_default_name() {
        let event = json!({});
        let result = handler_sync(event).unwrap();
        assert_eq!(result["message"], "Hello, world!");
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration {
    use aws_lambda_events::sqs::SqsEvent;

    #[tokio::test]
    async fn test_sqs_processing() {
        let event = build_test_sqs_event();
        let result = sqs_handler(event).await.unwrap();
        assert!(result.batch_item_failures.is_empty());
    }

    fn build_test_sqs_event() -> LambdaEvent<SqsEvent> {
        // Build test event
    }
}
```

---

## Summary

| Area | Recommendation |
|------|----------------|
| **Deployment** | Use Cargo Lambda for simplicity |
| **IaC** | Terraform for complex, SAM for simple |
| **CI/CD** | GitHub Actions with cargo-lambda |
| **Logging** | JSON format for CloudWatch Insights |
| **Tracing** | X-Ray with OpenTelemetry |
| **Security** | Least privilege IAM, Secrets Manager |
| **Cost** | ARM64, right-size memory, batch requests |

---

*See [04-valtron-integration.md](04-valtron-integration.md) for the Valtron alternative deployment guide.*
