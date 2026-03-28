---
title: "Event Sources Deep Dive"
subtitle: "Handling API Gateway, SQS, SNS, S3, and other Lambda event sources"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-rust-runtime/02-event-sources-deep-dive.md
related: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-rust-runtime/exploration.md
---

# Event Sources Deep Dive

## Introduction

This document provides comprehensive coverage of how the Lambda Rust Runtime handles various AWS event sources. The `lambda-events` crate provides strongly-typed event structs for 60+ AWS services.

### Event Crate Structure

```
lambda-events/
├── src/
│   ├── event/
│   │   ├── apigw/           # API Gateway (REST + HTTP)
│   │   ├── alb/             # Application Load Balancer
│   │   ├── sqs/             # SQS messages
│   │   ├── sns/             # SNS notifications
│   │   ├── s3/              # S3 object events
│   │   ├── dynamodb/        # DynamoDB streams
│   │   ├── kinesis/         # Kinesis streams
│   │   ├── cognito/         # Cognito triggers
│   │   ├── cloudwatch/      # CloudWatch events
│   │   └── ...              # 60+ event types
│   ├── encodings/           # Custom serializers
│   └── custom_serde/        # Serde helpers
```

---

## Part 1: API Gateway Events

### API Gateway HTTP API (v2.0)

```rust
use lambda_http::{LambdaEvent, Request, Response, Error};
use lambda_http::request::Payload;

async fn handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let (event, context) = event.into_parts();

    // Access request details
    match event.payload() {
        Payload::ApiGatewayV2(req) => {
            let method = &req.context.http.method;      // "GET", "POST", etc.
            let path = &req.raw_path;                    // "/api/users"
            let query = &req.raw_query_string;           // "page=1&limit=10"
            let headers = &req.headers;                  // HashMap<HeaderName, String>
            let body = &req.body;                        // Option<String>
        }
        _ => {}
    }

    // Build response
    Ok(http::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&response)?)
        .map_err(|e| e.into())?)
}
```

### API Gateway REST API (v1.0)

```rust
use lambda_http::{LambdaEvent, Request, Error};

async fn rest_handler(event: LambdaEvent<Request>) -> Result<Request, Error> {
    let (event, _context) = event.into_parts();

    match event.payload() {
        Payload::ApiGatewayProxy(req) => {
            let resource = &req.resource;           // "/users/{id}"
            let path = &req.path;                   // "/users/123"
            let http_method = &req.http_method;     // "GET"
            let path_params = &req.path_parameters; // {"id": "123"}
            let query_params = &req.query_string_parameters;
        }
        _ => {}
    }

    Ok(event)
}
```

### Path Parameter Extraction

```rust
use lambda_http::{Request, RequestExt, Error};

async fn get_user(event: LambdaEvent<Request>) -> Result<Value, Error> {
    let (event, _) = event.into_parts();

    // Extract path parameters
    if let Some(id) = event.path_parameters().get("id") {
        let user = fetch_user(id).await?;
        return Ok(json!({ "user": user }));
    }

    Ok(json!({ "error": "User ID not found" }))
}
```

### Query String Handling

```rust
use lambda_http::{Request, RequestExt, Error};
use serde::Deserialize;

#[derive(Deserialize)]
struct Pagination {
    page: Option<i32>,
    limit: Option<i32>,
}

async fn list_users(event: LambdaEvent<Request>) -> Result<Value, Error> {
    let (event, _) = event.into_parts();

    let pagination = event.query_string_parameters::<Pagination>()?;
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(10);

    let users = fetch_users(page, limit).await?;
    Ok(json!({ "users": users, "page": page, "limit": limit }))
}
```

---

## Part 2: SQS Events

### Basic SQS Handler

```rust
use aws_lambda_events::sqs::SqsEvent;
use lambda_runtime::{LambdaEvent, Error, service_fn};

async fn handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let message_id = record.message_id;
        let body: Value = serde_json::from_str(&record.body)?;

        tracing::info!("Processing message: {}", message_id);

        // Process message
        process_message(body).await?;
    }

    Ok(())
}
```

### SQS Batch Processing

```rust
use aws_lambda_events::sqs::{SqsEvent, SqsBatchResponse, SqsBatchItemFailure};

async fn sqs_batch_handler(event: LambdaEvent<SqsEvent>) -> Result<SqsBatchResponse, Error> {
    let mut batch_item_failures = Vec::new();

    for record in event.payload.records {
        match process_message(&record.body).await {
            Ok(()) => {
                tracing::info!("Processed message: {}", record.message_id);
            }
            Err(e) => {
                tracing::error!("Failed to process {}: {}", record.message_id, e);
                batch_item_failures.push(SqsBatchItemFailure {
                    item_identifier: record.message_id,
                });
            }
        }
    }

    Ok(SqsBatchResponse { batch_item_failures })
}
```

### Typed SQS Messages

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct OrderMessage {
    order_id: String,
    customer_id: String,
    items: Vec<OrderItem>,
    total: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct OrderItem {
    product_id: String,
    quantity: i32,
    price: f64,
}

async fn order_handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let order: OrderMessage = serde_json::from_str(&record.body)?;
        process_order(order).await?;
    }
    Ok(())
}
```

---

## Part 3: SNS Events

### Basic SNS Handler

```rust
use aws_lambda_events::sns::SnsEvent;
use lambda_runtime::{LambdaEvent, Error};

async fn sns_handler(event: LambdaEvent<SnsEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let message = &record.sns.message;
        let topic_arn = &record.sns.topic_arn;
        let message_id = &record.sns.message_id;

        tracing::info!("SNS message from {}: {}", topic_arn, message_id);

        // Parse message based on topic
        match topic_arn.contains("orders") {
            true => process_order_notification(message).await?,
            false => process_generic_notification(message).await?,
        }
    }

    Ok(())
}
```

### Typed SNS Messages

```rust
#[derive(Debug, Deserialize)]
struct UserEvent {
    event_type: String,
    user_id: String,
    timestamp: String,
    data: UserData,
}

#[derive(Debug, Deserialize)]
struct UserData {
    email: String,
    name: Option<String>,
}

async fn typed_sns_handler(event: LambdaEvent<SnsEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let user_event: UserEvent = serde_json::from_str(&record.sns.message)?;

        match user_event.event_type.as_str() {
            "user.created" => handle_user_created(&user_event).await?,
            "user.updated" => handle_user_updated(&user_event).await?,
            "user.deleted" => handle_user_deleted(&user_event).await?,
            _ => tracing::warn!("Unknown event type: {}", user_event.event_type),
        }
    }

    Ok(())
}
```

---

## Part 4: S3 Events

### S3 Object Event Handler

```rust
use aws_lambda_events::s3::S3Event;
use lambda_runtime::{LambdaEvent, Error};

async fn s3_handler(event: LambdaEvent<S3Event>) -> Result<(), Error> {
    for record in event.payload.records {
        let bucket = record.s3.bucket.name;
        let key = record.s3.object.key;
        let event_type = record.event_type;

        tracing::info!("S3 {} on {}/{}", event_type, bucket, key);

        match event_type.as_str() {
            "ObjectCreated:Put" => handle_object_created(&bucket, &key).await?,
            "ObjectRemoved:Delete" => handle_object_deleted(&bucket, &key).await?,
            _ => {}
        }
    }

    Ok(())
}
```

### S3 Object Processing

```rust
use aws_sdk_s3::Client as S3Client;

async fn handle_object_created(bucket: &str, key: &str) -> Result<(), Error> {
    // Download object
    let s3_client = S3Client::from_conf(sdk_config).await;

    let obj = s3_client.get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let body = obj.body.collect().await?.into_bytes();

    // Process object (e.g., resize image, parse CSV)
    process_object(&body).await?;

    Ok(())
}
```

---

## Part 5: DynamoDB Stream Events

### DynamoDB Stream Handler

```rust
use aws_lambda_events::dynamodb::DynamoDbStreamEvent;
use lambda_runtime::{LambdaEvent, Error};

async fn dynamodb_handler(event: LambdaEvent<DynamoDbStreamEvent>) -> Result<(), Error> {
    for record in event.payload.records {
        let event_name = record.event_name;
        let table_name = record.dynamodb.table_name;

        match event_name.as_str() {
            "INSERT" => {
                let new_image = record.dynamodb.new_image.unwrap();
                handle_insert(&table_name, new_image).await?;
            }
            "MODIFY" => {
                let old_image = record.dynamodb.old_image;
                let new_image = record.dynamodb.new_image.unwrap();
                handle_modify(&table_name, old_image, new_image).await?;
            }
            "REMOVE" => {
                let old_image = record.dynamodb.old_image.unwrap();
                handle_remove(&table_name, old_image).await?;
            }
            _ => {}
        }
    }

    Ok(())
}
```

---

## Part 6: Event Builder Patterns

### Lambda Events Comprehensive Builders

```rust
use aws_lambda_events::sqs::{
    SqsMessage, SqsEvent, SqsAttributes, SqsMessageAttribute,
};
use chrono::{DateTime, Utc};

// Build SQS event for testing
fn build_test_sqs_event() -> SqsEvent {
    SqsEvent {
        records: vec![SqsMessage {
            message_id: Some("test-123".to_string()),
            receipt_handle: Some("receipt-456".to_string()),
            body: r#"{"order_id": "ORD-001"}"#.to_string(),
            attributes: SqsAttributes {
                approximate_receive_count: "1".to_string(),
                sent_timestamp: DateTime::<Utc>::from_utc(
                    chrono::NaiveDateTime::from_timestamp_opt(1234567890, 0).unwrap(),
                    Utc,
                ),
                sender_id: "sender".to_string(),
                approximate_first_receive_timestamp: DateTime::<Utc>::from_utc(
                    chrono::NaiveDateTime::from_timestamp_opt(1234567890, 0).unwrap(),
                    Utc,
                ),
                sequence_number: None,
                message_group_id: None,
                message_deduplication_id: None,
                aws_trace_header: None,
            },
            message_attributes: std::collections::HashMap::new(),
            md5_of_body: Some("md5-hash".to_string()),
            md5_of_message_attributes: None,
            event_source_arn: Some("arn:aws:sqs:us-east-1:123456789012:test-queue".to_string()),
            event_source: Some("aws:sqs".to_string()),
            aws_region: Some("us-east-1".to_string()),
        }],
    }
}
```

---

## Part 7: Event Source Mapping

### Complete Event Handler Router

```rust
use lambda_runtime::{LambdaEvent, Error};
use serde_json::Value;

enum EventSource {
    ApiGateway,
    Sqs,
    Sns,
    S3,
    Unknown,
}

fn detect_event_source(event: &Value) -> EventSource {
    if event.get("requestContext").is_some()
        && event.get("version").and_then(|v| v.as_str()) == Some("2.0")
    {
        EventSource::ApiGateway
    } else if event.get("Records").and_then(|r| r.as_array()).map_or(false, |r| {
        r.first().and_then(|f| f.get("eventSource")).and_then(|s| s.as_str()) == Some("aws:sqs")
    }) {
        EventSource::Sqs
    } else if event.get("Records").and_then(|r| r.as_array()).map_or(false, |r| {
        r.first().and_then(|f| f.get("EventSource")).and_then(|s| s.as_str()) == Some("aws:sns")
    }) {
        EventSource::Sns
    } else if event.get("Records").and_then(|r| r.as_array()).map_or(false, |r| {
        r.first().and_then(|f| f.get("eventSource")).and_then(|s| s.as_str()) == Some("aws:s3")
    }) {
        EventSource::S3
    } else {
        EventSource::Unknown
    }
}

async fn router_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    match detect_event_source(&event.payload) {
        EventSource::ApiGateway => handle_api_gateway(event).await,
        EventSource::Sqs => handle_sqs(event).await,
        EventSource::Sns => handle_sns(event).await,
        EventSource::S3 => handle_s3(event).await,
        EventSource::Unknown => Ok(json!({"error": "Unknown event source"})),
    }
}
```

---

## Summary

| Event Source | Event Type | Key Fields |
|--------------|-----------|------------|
| **API Gateway HTTP** | `Request` (Payload::ApiGatewayV2) | `raw_path`, `headers`, `body` |
| **API Gateway REST** | `Request` (Payload::ApiGatewayProxy) | `resource`, `pathParameters`, `httpMethod` |
| **SQS** | `SqsEvent` | `records[].body`, `messageId` |
| **SNS** | `SnsEvent` | `records[].sns.message`, `topicArn` |
| **S3** | `S3Event` | `records[].s3.bucket.name`, `key` |
| **DynamoDB** | `DynamoDbStreamEvent` | `records[].dynamodb.newImage` |

---

*Continue to [03-cold-start-optimization-deep-dive.md](03-cold-start-optimization-deep-dive.md) for performance optimization strategies.*
