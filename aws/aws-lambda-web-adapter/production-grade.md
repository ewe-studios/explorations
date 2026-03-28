---
title: "Production-Grade Lambda Web Adapter"
subtitle: "Deployment, monitoring, and optimization for production workloads"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-web-adapter/production-grade.md
related: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-web-adapter/exploration.md
---

# Production-Grade Lambda Web Adapter

## Introduction

This document covers production deployment considerations for Lambda Web Adapter including performance tuning, monitoring, security, and cost optimization.

---

## Part 1: Deployment Strategies

### Docker Image Deployment (Recommended)

```dockerfile
FROM python:3.11-slim

WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application
COPY . .

# Add Lambda Web Adapter
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:1.0.0-rc1 \
  /lambda-adapter /opt/extensions/lambda-adapter

# Configure for Lambda
ENV AWS_LWA_PORT=8080
ENV AWS_LWA_READINESS_CHECK_PATH=/health
ENV AWS_LWA_INVOKE_MODE=buffered

EXPOSE 8080
CMD ["python", "app.py"]
```

### Terraform Deployment

```hcl
resource "aws_lambda_function" "web_app" {
  filename      = "lambda.zip"
  function_name = "my-web-app"
  role          = aws_iam_role.lambda_role.arn
  handler       = "run.sh"
  runtime       = "provided.al2023"
  timeout       = 30
  memory_size   = 512

  environment {
    variables = {
      AWS_LWA_PORT                  = "8080"
      AWS_LWA_READINESS_CHECK_PATH  = "/health"
      AWS_LWA_INVOKE_MODE           = "buffered"
      AWS_LAMBDA_EXEC_WRAPPER       = "/opt/bootstrap"
    }
  }

  layers = [
    "arn:aws:lambda:${var.region}:753240598075:layer:LambdaAdapterLayerX86:26"
  ]
}

resource "aws_lambda_function_url" "web_app_url" {
  function_name      = aws_lambda_function.web_app.function_name
  authorization_type = "AWS_IAM"

  cors {
    allow_origins     = ["*"]
    allow_methods     = ["*"]
    allow_headers     = ["*"]
    max_age           = 86400
  }
}
```

### SAM Template

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  WebAppFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: .
      Handler: run.sh
      Runtime: provided.al2023
      Timeout: 30
      MemorySize: 512
      Environment:
        Variables:
          AWS_LWA_PORT: "8080"
          AWS_LWA_READINESS_CHECK_PATH: "/health"
      Layers:
        - !Sub arn:aws:lambda:${AWS::Region}:753240598075:layer:LambdaAdapterLayerX86:26
      Events:
        ApiEvent:
          Type: HttpApi
          Properties:
            Path: /{proxy+}
            Method: ANY
```

---

## Part 2: Performance Optimization

### Cold Start Optimization

| Strategy | Impact | Implementation |
|----------|--------|----------------|
| **Provisioned Concurrency** | Eliminates cold starts | Configure in Lambda console |
| **SnapStart** (Java) | Up to 90% reduction | Enable for Java runtimes |
| **Smaller packages** | Faster download | Remove unused dependencies |
| **Arm64 architecture** | Better price/performance | Use `--arm64` in build |
| **Optimized readiness** | Faster init | Use TCP health checks |

### Memory Tuning

```
Memory vs Performance:

128 MB  - Minimum, slow CPU
256 MB  - Basic apps
512 MB  - Recommended starting point
1024 MB - High throughput
2048+ MB - CPU-intensive workloads

Rule: Memory allocation affects CPU proportionally
```

### Connection Pooling

```python
# For Python web apps
from requests.adapters import HTTPAdapter
from requests import Session

session = Session()
adapter = HTTPAdapter(pool_connections=10, pool_maxsize=20)
session.mount('http://', adapter)
```

### Response Compression

```bash
# Enable gzip compression
export AWS_LWA_ENABLE_COMPRESSION=true
```

---

## Part 3: Monitoring and Observability

### CloudWatch Metrics

```python
import boto3
cloudwatch = boto3.client('cloudwatch')

# Custom metrics
cloudwatch.put_metric_data(
    Namespace='LambdaWebAdapter',
    MetricData=[
        {
            'MetricName': 'RequestLatency',
            'Value': latency_ms,
            'Unit': 'Milliseconds'
        },
        {
            'MetricName': 'WebAppHealth',
            'Value': 1 if healthy else 0,
            'Unit': 'Count'
        }
    ]
)
```

### Distributed Tracing

```python
from opentelemetry import trace
from opentelemetry.exporter.otlp.proto.grpc.exporter import OTLPSpanExporter

# Initialize tracing
trace.set_tracer_provider(
    TracerProvider(
        span_processors=[
            BatchSpanProcessor(OTLPSpanExporter())
        ]
    )
)

tracer = trace.get_tracer(__name__)

@tracer.start_as_current_span("handle_request")
def handler(event, context):
    # Your handler logic
    pass
```

### Structured Logging

```python
import json
import logging

class LambdaFormatter(logging.Formatter):
    def format(self, record):
        return json.dumps({
            'level': record.levelname,
            'message': record.getMessage(),
            'timestamp': self.formatTime(record),
            'request_id': getattr(record, 'request_id', None),
        })

handler = logging.StreamHandler()
handler.setFormatter(LambdaFormatter())
logging.getLogger().addHandler(handler)
```

---

## Part 4: Security Considerations

### IAM Permissions

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
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "xray:PutTraceSegments",
        "xray:PutTelemetryRecords"
      ],
      "Resource": "*"
    }
  ]
}
```

### VPC Configuration

```hcl
resource "aws_lambda_function" "web_app" {
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

  egress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
```

### Environment Variable Encryption

```hcl
resource "aws_lambda_function" "web_app" {
  # ...

  environment {
    variables = {
      DATABASE_URL = aws_kms_encrypted_parameter.db_url.value
    }
  }

  kms_key_arn = aws_kms_key.lambda.arn
}
```

---

## Part 5: Cost Optimization

### Pricing Calculator

```
Lambda Pricing (us-east-1):
- Requests: $0.20 per 1M requests
- Duration: $0.0000166667 per GB-second

Example calculation:
- 1M requests/month
- 512 MB memory
- 500ms average duration

Cost = (1M * $0.20/1M) + (1M * 0.5GB * 0.5s * $0.0000166667)
     = $0.20 + $4.17
     = $4.37/month
```

### Cost Optimization Strategies

| Strategy | Savings | Trade-off |
|----------|---------|-----------|
| Right-size memory | 20-50% | May increase duration |
| Provisioned concurrency | Predictable costs | Higher base cost |
| ARM64 architecture | 20% | Compatibility testing |
| Request batching | Significant | Increased latency |

---

## Part 6: Error Handling and Retry

### Error Response Mapping

```python
def lambda_handler(event, context):
    try:
        response = web_app.handle(event)
        return {
            'statusCode': response.status_code,
            'body': response.body,
            'headers': response.headers
        }
    except TimeoutError:
        return {
            'statusCode': 504,
            'body': 'Gateway Timeout'
        }
    except ConnectionError:
        return {
            'statusCode': 502,
            'body': 'Bad Gateway'
        }
    except Exception as e:
        return {
            'statusCode': 500,
            'body': f'Internal Server Error: {str(e)}'
        }
```

### Retry Configuration

```hcl
resource "aws_lambda_function" "web_app" {
  # ...

  destinations {
    on_failure {
      destination = aws_sqs_queue.dlq.arn
    }
  }
}

resource "aws_lambda_function_event_invoke_config" "web_app" {
  function_name = aws_lambda_function.web_app.function_name

  maximum_retry_attempts = 2
  maximum_event_age_in_seconds = 3600
}
```

---

## Part 7: Testing Strategies

### Local Testing with RIE

```bash
# Build image
docker build -t my-web-app .

# Run with Runtime Interface Emulator
docker run -d -p 9000:8080 \
  -e AWS_LAMBDA_RUNTIME_API=0.0.0.0:9001 \
  --entrypoint /lambda-adapter \
  my-web-app

# Test invocation
curl -X POST "http://localhost:9000/2015-03-31/functions/function/invocations" \
  -d '{"key": "value"}'
```

### Integration Tests

```python
import pytest
import requests

@pytest.fixture
def lambda_url():
    return os.environ['LAMBDA_FUNCTION_URL']

def test_api_gateway_event(lambda_url):
    response = requests.post(
        lambda_url,
        json={
            'version': '2.0',
            'rawPath': '/api/users',
            'requestContext': {
                'http': {'method': 'GET'}
            }
        }
    )
    assert response.status_code == 200
```

---

## Part 8: Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Timeout on init | Slow readiness check | Use TCP check, reduce timeout |
| 502 Bad Gateway | Web app crashed | Check app logs, increase memory |
| High latency | Cold starts | Enable provisioned concurrency |
| Memory exceeded | App memory leak | Profile app, increase memory |

### Debug Mode

```bash
# Enable debug logging
export AWS_LWA_LOG_LEVEL=debug
export RUST_LOG=debug

# Check adapter logs
aws logs tail /aws/lambda/my-function --follow
```

---

## Summary

| Area | Key Recommendations |
|------|---------------------|
| **Deployment** | Docker images with adapter layer |
| **Performance** | Provisioned concurrency, ARM64 |
| **Monitoring** | CloudWatch + distributed tracing |
| **Security** | Least privilege IAM, VPC isolation |
| **Cost** | Right-size memory, batch requests |
| **Testing** | RIE for local, integration tests |

---

*See [exploration.md](exploration.md) for the complete project overview.*
