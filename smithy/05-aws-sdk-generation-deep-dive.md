---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy
repository: github.com:smithy-lang/smithy
explored_at: 2026-04-04
focus: AWS SDK generation, smithy-rs architecture, client/runtime patterns
---

# Deep Dive: AWS SDK Generation and smithy-rs

## Overview

This deep dive examines smithy-rs, the Rust SDK generator that powers the AWS SDK for Rust. We explore the client architecture, runtime components, serialization/deserialization, error handling, and credential providers.

## Architecture

```mermaid
flowchart TB
    subgraph Smithy Model
        AWS[AWS Service Model] --> Traits[AWS Traits]
        Traits --> Endpoint[Endpoint Rules]
        Traits --> Auth[Auth Traits]
        Traits --> Paging[Pagination Traits]
    end
    
    subgraph Code Generation
        AWS --> RustGen[smithy-rs Generator]
        RustGen --> Client[Client Code]
        RustGen --> Types[Type Definitions]
        RustGen --> Protocol[Protocol Code]
    end
    
    subgraph Runtime
        Client --> Runtime[aws-smithy-runtime]
        Runtime --> HttpClient[HTTP Client]
        Runtime --> Middleware[Middleware Stack]
        Runtime --> Retry[Retry Logic]
    end
    
    subgraph Components
        Types --> Config[Config Types]
        Types --> Input[Input Types]
        Types --> Output[Output Types]
        Types --> Error[Error Types]
    end
    
    subgraph AWS Integration
        Endpoint --> Resolver[Endpoint Resolver]
        Auth --> SigV4[SigV4 Signer]
        Paging --> Paginator[Paginator]
    end
```

## smithy-rs Structure

```
smithy-rs/
├── codegen-core/           # Shared code generation logic
│   └── src/main/kotlin/software/amazon/smithy/rust/codegen/core/
│       ├── core/           # Core generation utilities
│       ├── smithy/         # Smithy model handling
│       └── writer/         # Rust code writers
│
├── codegen-client/         # Client-side code generation
│   └── src/main/kotlin/software/amazon/smithy/rust/codegen/client/
│       ├── client/         # Client generator
│       ├── config/         # Config generator
│       ├── operation/      # Operation generator
│       └── protocol/       # Protocol generators
│
├── codegen-server/         # Server-side code generation
│   └── src/main/kotlin/software/amazon/smithy/rust/codegen/server/
│       ├── server/         # Server generator
│       ├── protocol/       # Protocol handlers
│       └── config/         # Server config
│
├── rust-runtime/           # Rust runtime crates
│   ├── aws-runtime/        # AWS-specific runtime
│   ├── aws-smithy-async/   # Async utilities
│   ├── aws-smithy-http/    # HTTP utilities
│   ├── aws-smithy-runtime/ # Runtime client
│   ├── aws-smithy-types/   # Type definitions
│   └── aws-smithy-json/    # JSON utilities
│
└── aws/                    # AWS SDK generation
    ├── sdk/                # Full SDK generator
    └── sdk-codegen/        # AWS-specific codegen
```

## Client Generator

```kotlin
// codegen-client/src/main/kotlin/software/amazon/smithy/rust/codegen/client/ClientGenerator.kt

class ClientGenerator(
    private val model: Model,
    private val settings: RustSettings,
    private val symbolProvider: RustSymbolProvider,
    private val protocolGenerator: ProtocolGenerator
) {
    fun generate(writer: RustWriter) {
        // Generate client struct
        generateClientStruct(writer)
        
        // Generate client builder
        generateClientBuilder(writer)
        
        // Generate operations
        generateOperations(writer)
        
        // Generate config
        generateConfig(writer)
        
        // Generate error types
        generateErrors(writer)
    }
    
    private fun generateClientStruct(writer: RustWriter) {
        writer.rustBlock("pub struct Client") {
            write("handle: Handle,")
        }
        writer.rustBlock("struct Handle") {
            write("conf: crate::Config,")
            write("runtime_components: RuntimeComponents,")
            write("endpoint_resolver: EndpointResolver,")
            write("http_client: Arc<dyn HttpClient>,")
            write("retry_config: Option<RetryConfig>,")
        }
    }
    
    private fun generateClientBuilder(writer: RustWriter) {
        writer.rustBlock("pub struct Builder") {
            write("conf: crate::Config,")
            write("http_client: Option<Arc<dyn HttpClient>>,")
            write("retry_config: Option<RetryConfig>,")
        }
        
        writer.rustBlock("impl Builder") {
            // Config method
            rustBlock("pub fn config(mut self, conf: crate::Config) -> Self") {
                write("self.conf = conf;")
                write("self")
            }
            
            // HTTP client method
            rustBlock("pub fn http_client(mut self, client: impl HttpClient + 'static) -> Self") {
                write("self.http_client = Some(Arc::new(client));")
                write("self")
            }
            
            // Retry config method
            rustBlock("pub fn retry_config(mut self, config: RetryConfig) -> Self") {
                write("self.retry_config = Some(config);")
                write("self")
            }
            
            // Build method
            rustBlock("pub fn build(self) -> Result<Client, Error>") {
                write("let handle = Handle {")
                indent()
                write("conf: self.conf,")
                write("runtime_components: RuntimeComponents::new(&self.conf),")
                write("endpoint_resolver: self.conf.endpoint_resolver()?,")
                write("http_client: self.http_client.ok_or(Error::MissingHttpClient)?,")
                write("retry_config: self.retry_config,")
                dedent()
                write("};")
                write("Ok(Client { handle: Arc::new(handle) })")
            }
        }
    }
    
    private fun generateOperations(writer: RustWriter) {
        for (operation in model.operationShapes) {
            generateOperationMethod(writer, operation)
        }
    }
    
    private fun generateOperationMethod(
        writer: RustWriter,
        operation: OperationShape
    ) {
        val inputName = operation.inputShape.name
        val outputName = operation.outputShape.name
        val errorName = operation.errorShape.name
        
        writer.rustBlock(
            "pub async fun ${operation.name}(&self, input: $inputName) -> Result<$outputName, $errorName>"
        ) {
            write("let request = self.build_request(&input)?;")
            write("let response = self.send_with_retry(request).await?;")
            write("parse_response(response)")
        }
    }
}
```

## Operation Generator

```kotlin
// codegen-client/src/main/kotlin/software/amazon/smithy/rust/codegen/client/OperationGenerator.kt

class OperationGenerator(
    private val model: Model,
    private val symbolProvider: RustSymbolProvider,
    private val protocolGenerator: ProtocolGenerator
) {
    fun generateOperationFiles(operation: OperationShape, context: GenerationContext) {
        // Generate input structure
        generateInput(context, operation)
        
        // Generate output structure
        generateOutput(context, operation)
        
        // Generate error types
        generateErrors(context, operation)
        
        // Generate operation builder
        generateOperationBuilder(context, operation)
        
        // Generate serialization
        protocolGenerator.generateRequestSerializer(context, operation)
        
        // Generate deserialization
        protocolGenerator.generateResponseDeserializer(context, operation)
    }
    
    private fun generateInput(context: GenerationContext, operation: OperationShape) {
        val inputShape = operation.inputShape
        val writer = context.writerFor("input.rs")
        
        writer.rustBlock("pub struct ${inputShape.name}") {
            for (member in inputShape.members) {
                val type = symbolProvider.toRustType(member.target)
                val name = symbolProvider.toMemberName(member)
                write("$name: $type,")
            }
        }
        
        // Generate builder
        generateBuilderPattern(writer, inputShape)
    }
    
    private fun generateBuilderPattern(
        writer: RustWriter,
        shape: StructureShape
    ) {
        val builderName = "${shape.name}Builder"
        
        writer.rustBlock("pub struct $builderName") {
            for (member in shape.members) {
                val name = symbolProvider.toMemberName(member)
                write("$name: std::option::Option<${symbolProvider.toRustType(member.target)}>,")
            }
        }
        
        writer.rustBlock("impl $builderName") {
            for (member in shape.members) {
                val name = symbolProvider.toMemberName(member)
                val type = symbolProvider.toRustType(member.target)
                
                rustBlock("pub fn $name(mut self, input: impl Into<$type>) -> Self") {
                    write("self.$name = Some(input.into());")
                    write("self")
                }
            }
            
            rustBlock("pub fn build(self) -> Result<${shape.name}, Error>") {
                write("Ok(${shape.name} {")
                indent()
                for (member in shape.members) {
                    val name = symbolProvider.toMemberName(member)
                    if (member.isRequired) {
                        write("$name: self.$name.ok_or(Error::MissingField($S))?,", name)
                    } else {
                        write("$name: self.$name,")
                    }
                }
                dedent()
                write("})")
            }
        }
    }
}
```

## HTTP Client Runtime

```rust
// aws-smithy-runtime/src/client/http.rs

/// HTTP client trait
pub trait HttpClient: Send + Sync + std::fmt::Debug {
    /// Send an HTTP request
    fn http_request(
        &self,
        request: http::Request<SdkBody>,
    ) -> Result<http::Response<SdkBody>, BoxError>;
    
    /// Get connector info
    fn connector_info(&self) -> &ConnectorInfo;
}

/// Default HTTP client (hyper)
pub struct HyperClient {
    inner: hyper::Client<HttpsConnector<HttpConnector>, SdkBody>,
    connector_info: ConnectorInfo,
}

impl HttpClient for HyperClient {
    fn http_request(
        &self,
        request: http::Request<SdkBody>,
    ) -> Result<http::Response<SdkBody>, BoxError> {
        // Execute request
        let response = self.inner.request(request)?;
        
        // Wait for response
        let response = tokio::runtime::Handle::current()
            .block_on(response)?;
        
        Ok(response)
    }
    
    fn connector_info(&self) -> &ConnectorInfo {
        &self.connector_info
    }
}

/// Connector information
pub struct ConnectorInfo {
    pub name: &'static str,
    pub version: &'static str,
}

impl Default for ConnectorInfo {
    fn default() -> Self {
        Self {
            name: "hyper",
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}
```

## Retry Logic

```rust
// aws-smithy-runtime/src/client/retry.rs

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts (including initial attempt)
    pub max_attempts: u32,
    
    /// Initial retry delay
    pub initial_backoff: Duration,
    
    /// Maximum retry delay
    pub max_backoff: Duration,
    
    /// Backoff multiplier
    pub base: u32,
    
    /// Retry strategy
    pub strategy: RetryStrategy,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(20),
            base: 2,
            strategy: RetryStrategy::Standard,
        }
    }
}

/// Retry strategy
#[derive(Debug, Clone, Copy)]
pub enum RetryStrategy {
    /// Standard exponential backoff with jitter
    Standard,
    
    /// Adaptive rate limiting
    Adaptive,
}

/// Retry classifier for errors
pub trait RetryClassifier: Send + Sync {
    /// Classify an error
    fn classify(&self, error: &Error) -> RetryKind;
}

/// Retry decision
#[derive(Debug)]
pub enum RetryKind {
    /// Do not retry
    NotRetryable,
    
    /// Retryable with explicit delay
    Retryable { delay: Duration },
    
    /// Retryable with computed delay
    ComputeDelay,
}

/// Standard AWS retry classifier
pub struct AwsRetryClassifier;

impl RetryClassifier for AwsRetryClassifier {
    fn classify(&self, error: &Error) -> RetryKind {
        match error {
            // 5xx errors are retryable
            Error::HttpResponse { status, .. } if status.is_server_error() => {
                RetryKind::ComputeDelay
            }
            
            // 429 Too Many Requests is retryable
            Error::HttpResponse { status, .. } if status.as_u16() == 429 => {
                RetryKind::ComputeDelay
            }
            
            // Connection errors are retryable
            Error::Connection(_) => RetryKind::ComputeDelay,
            
            // Timeout is retryable
            Error::Timeout(_) => RetryKind::ComputeDelay,
            
            _ => RetryKind::NotRetryable,
        }
    }
}

/// Retry executor
pub struct RetryExecutor {
    config: RetryConfig,
    classifier: Box<dyn RetryClassifier>,
}

impl RetryExecutor {
    pub fn new(config: RetryConfig, classifier: Box<dyn RetryClassifier>) -> Self {
        Self { config, classifier }
    }
    
    pub async fn run_with_retry<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, E>
    where
        F: Fn() -> BoxFuture<'static, Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0;
        let mut current_delay = self.config.initial_backoff;
        
        loop {
            attempt += 1;
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    // Classify error
                    let retry_kind = self.classifier.classify(&error);
                    
                    match retry_kind {
                        RetryKind::NotRetryable => return Err(error),
                        
                        RetryKind::Retryable { delay } => {
                            if attempt >= self.config.max_attempts {
                                return Err(error);
                            }
                            tokio::time::sleep(delay).await;
                        }
                        
                        RetryKind::ComputeDelay => {
                            if attempt >= self.config.max_attempts {
                                return Err(error);
                            }
                            
                            // Exponential backoff with jitter
                            let jitter = rand::random::<f32>();
                            let delay = current_delay.mul_f32(1.0 + jitter);
                            
                            tokio::time::sleep(delay).await;
                            
                            current_delay = std::cmp::min(
                                current_delay * self.config.base,
                                self.config.max_backoff,
                            );
                        }
                    }
                }
            }
        }
    }
}
```

## Credential Providers

```rust
// aws-runtime/src/auth/credentials.rs

/// AWS credentials
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    /// Access key ID
    pub access_key_id: String,
    
    /// Secret access key
    pub secret_access_key: String,
    
    /// Session token (for temporary credentials)
    pub session_token: Option<String>,
    
    /// Expiration time
    pub expiration: Option<DateTime>,
}

/// Credential provider trait
pub trait ProvideCredentials: Send + Sync {
    /// Provide credentials
    fn provide_credentials<'a>(
        &'a self,
    ) -> ProvideCredentialsFuture<'a>;
}

/// Credential provider chain
pub struct CredentialChain {
    providers: Vec<Box<dyn ProvideCredentials>>,
}

impl CredentialChain {
    pub fn builder() -> CredentialChainBuilder {
        CredentialChainBuilder::default()
    }
}

impl ProvideCredentials for CredentialChain {
    fn provide_credentials<'a>(
        &'a self,
    ) -> ProvideCredentialsFuture<'a> {
        ProvideCredentialsFuture::new(async move {
            // Try each provider in order
            for provider in &self.providers {
                match provider.provide_credentials().await {
                    Ok(creds) => return Ok(creds),
                    Err(_) => continue,  // Try next provider
                }
            }
            Err(CredentialsError::no_providers())
        })
    }
}

/// Environment variable credential provider
pub struct EnvironmentProvider;

impl ProvideCredentials for EnvironmentProvider {
    fn provide_credentials<'a>(
        &'a self,
    ) -> ProvideCredentialsFuture<'a> {
        ProvideCredentialsFuture::new(async move {
            let access_key = std::env::var("AWS_ACCESS_KEY_ID")
                .map_err(|_| CredentialsError::missing_access_key())?;
            
            let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
                .map_err(|_| CredentialsError::missing_secret_key())?;
            
            let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
            
            Ok(AwsCredentials {
                access_key_id: access_key,
                secret_access_key: secret_key,
                session_token,
                expiration: None,
            })
        })
    }
}

/// EC2 IMDS credential provider
pub struct ImdsProvider {
    client: ImdsClient,
}

impl ProvideCredentials for ImdsProvider {
    fn provide_credentials<'a>(
        &'a self,
    ) -> ProvideCredentialsFuture<'a> {
        ProvideCredentialsFuture::new(async move {
            // Get IAM role name
            let role_name = self.client
                .get_metadata("/latest/meta-data/iam/security-credentials/")
                .await?;
            
            // Get credentials for role
            let creds_response: ImdsCredentialsResponse = self.client
                .get_json(&format!(
                    "/latest/meta-data/iam/security-credentials/{}",
                    role_name
                ))
                .await?;
            
            Ok(AwsCredentials {
                access_key_id: creds_response.access_key_id,
                secret_access_key: creds_response.secret_access_key,
                session_token: Some(creds_response.token),
                expiration: Some(creds_response.expiration),
            })
        })
    }
}

/// SigV4 request signer
pub struct SigV4Signer;

impl SigV4Signer {
    pub fn sign(
        &self,
        request: &mut http::Request<SdkBody>,
        credentials: &AwsCredentials,
        region: &str,
        service: &str,
    ) -> Result<(), SignError> {
        let signing_time = SystemTime::now();
        
        // Create canonical request
        let canonical_request = CanonicalRequest::from_request(request)?;
        
        // Create string to sign
        let string_to_sign = StringToSign::new(
            signing_time,
            &canonical_request,
            region,
            service,
        );
        
        // Compute signature
        let signature = compute_signature(
            &credentials.secret_access_key,
            region,
            service,
            signing_time,
            &string_to_sign,
        )?;
        
        // Add authorization header
        let authorization = AuthorizationHeader::new(
            credentials.access_key_id.clone(),
            region,
            service,
            signing_time,
            signature,
        );
        
        request.headers_mut().insert(
            http::header::AUTHORIZATION,
            authorization.to_value().parse().unwrap(),
        );
        
        Ok(())
    }
}
```

## Conclusion

smithy-rs provides:

1. **Type-Safe Clients**: Generated Rust code from Smithy models
2. **Runtime Components**: HTTP client, retry, credentials, signing
3. **AWS Integration**: SigV4, IMDS, endpoint resolution
4. **Builder Pattern**: Ergonomic API construction
5. **Error Handling**: Typed errors with retry classification
