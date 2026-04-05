---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy
repository: github.com:smithy-lang/smithy
explored_at: 2026-04-04
focus: Protocol generation (REST, JSON, RPC, GraphQL), serialization/deserialization, HTTP bindings
---

# Deep Dive: Protocol Generation and Serialization

## Overview

This deep dive examines how Smithy generates protocol-specific code for REST, JSON, RPC, and GraphQL protocols. We explore HTTP bindings, serialization/deserialization logic, error handling, and the protocol test framework.

## Architecture

```mermaid
flowchart TB
    subgraph Smithy Model
        Operation[Operation Shape] --> HttpTrait[@http trait]
        Operation --> Input[Input Structure]
        Operation --> Output[Output Structure]
    end
    
    subgraph Protocol Layer
        HttpTrait --> Protocol[Protocol Generator]
        Protocol --> REST[REST/JSON]
        Protocol --> RPC[RPC/gRPC]
        Protocol --> GraphQL[GraphQL]
    end
    
    subgraph Serialization
        Input --> HttpBinding[HTTP Binding Generator]
        HttpBinding --> Header[Header Serialization]
        HttpBinding --> Query[Query Parameter]
        HttpBinding --> Path[Path Parameter]
        HttpBinding --> Body[Body Serialization]
    end
    
    subgraph Deserialization
        Output --> ResponseParser[Response Parser]
        ResponseParser --> StatusParser[Status Code Parser]
        ResponseParser --> HeaderParser[Header Parser]
        ResponseParser --> BodyParser[Body Parser]
    end
    
    subgraph Error Handling
        Operation --> Errors[Error Shapes]
        Errors --> ErrorMapper[Error Mapper]
        ErrorMapper --> ErrorCode[Error Code]
        ErrorMapper --> ErrorType[Error Type]
    end
```

## HTTP Trait

```java
// software/amazon/smithy/model/traits/HttpTrait.java

@TraitDefinition("http")
public final class HttpTrait extends Trait implements HttpBindingTrait {
    private final String method;
    private final String uri;
    private final int code;
    private final Set<Integer> additionalSuccessCodes;
    private final List<HttpBinding> requestBindings;
    private final List<HttpBinding> responseBindings;
    
    public HttpTrait(Node node) {
        super(ShapeId.fromBuiltIn("http"), node);
        ObjectNode obj = node.expectObjectNode();
        
        this.method = obj.expectStringMember("method").getValue().toUpperCase();
        this.uri = obj.expectStringMember("uri").getValue();
        this.code = obj.expectNumberMember("code").getValue().intValue();
        
        this.additionalSuccessCodes = obj.getArrayMember("additionalSuccessCodes")
            .map(array -> array.getElementsAs(StringNode.class, 
                s -> Integer.parseInt(s.getValue())))
            .orElse(Collections.emptySet());
        
        this.requestBindings = parseBindings(obj.getObjectMember("request"));
        this.responseBindings = parseBindings(obj.getObjectMember("response"));
    }
    
    /// HTTP method
    public String getMethod() {
        return method;
    }
    
    /// URI pattern with labels
    public String getUri() {
        return uri;
    }
    
    /// Success status code
    public int getCode() {
        return code;
    }
    
    /// Check if method is read-only
    public boolean isReadOperation() {
        return "GET".equalsIgnoreCase(method) || 
               "HEAD".equalsIgnoreCase(method);
    }
    
    /// Parse URI labels
    public Set<String> getUriLabels() {
        Set<String> labels = new HashSet<>();
        Pattern pattern = Pattern.compile("\\{([^}]+)\\}");
        Matcher matcher = pattern.matcher(uri);
        
        while (matcher.find()) {
            labels.add(matcher.group(1));
        }
        
        return labels;
    }
    
    /// Get request binding for member
    public Optional<HttpBinding> getRequestBinding(String memberName) {
        return requestBindings.stream()
            .filter(b -> b.getMemberName().equals(memberName))
            .findFirst();
    }
    
    /// Get response binding for member
    public Optional<HttpBinding> getResponseBinding(String memberName) {
        return responseBindings.stream()
            .filter(b -> b.getMemberName().equals(memberName))
            .findFirst();
    }
}

/// HTTP binding location
public enum HttpBindingLocation {
    LABEL,      // URI path parameter
    QUERY,      // Query string parameter
    HEADER,     // HTTP header
    PREFIX,     // Header prefix
    PAYLOAD     // Body payload
}

/// HTTP binding definition
public class HttpBinding {
    private final String memberName;
    private final HttpBindingLocation location;
    private final String locationName;  // Custom name in HTTP
    private final boolean isGreedyLabel;
    
    public HttpBinding(
        String memberName,
        HttpBindingLocation location,
        String locationName,
        boolean isGreedyLabel
    ) {
        this.memberName = memberName;
        this.location = location;
        this.locationName = locationName;
        this.isGreedyLabel = isGreedyLabel;
    }
    
    /// Get HTTP name (or member name if not specified)
    public String getHttpName() {
        return locationName != null ? locationName : memberName;
    }
}
```

## REST/JSON Protocol Generator

```java
// software/amazon/smithy/aws-protocol-tests/src/main/java/RestJsonProtocolGenerator.java

public class RestJsonProtocolGenerator implements ProtocolGenerator {
    @Override
    public String getProtocolName() {
        return "restJson1";
    }
    
    @Override
    public void generateService(ServiceGeneratorContext context) {
        TypeScriptWriter writer = context.getWriter();
        ServiceShape service = context.getService();
        SymbolProvider symbolProvider = context.getSymbolProvider();
        
        // Write imports
        writer.addImport("HttpClient", "@aws-sdk/types");
        writer.addImport("RequestHandler", "@aws-sdk/types");
        
        // Generate client class
        writer.openBlock("export class $LClient {", "}",
            service.getId().getName(), () -> {
            
            // Configuration
            writer.write("private readonly config: $LClientConfig;", service.getId().getName());
            writer.write("private readonly client: RequestHandler<any, any>;");
            writer.write("");
            
            // Constructor
            writer.openBlock("constructor(config: $LClientConfig) {", "}",
                service.getId().getName(), () -> {
                writer.write("this.config = config;");
                writer.write("this.client = new HttpClient({ baseUrl: config.endpoint });");
            });
            writer.write("");
            
            // Generate operation methods
            for (ShapeId opId : service.getAllOperations()) {
                context.getModel().getShape(opId, OperationShape.class)
                    .ifPresent(op -> generateOperationMethod(context, op));
            }
        });
    }
    
    private void generateOperationMethod(
        ServiceGeneratorContext context,
        OperationShape operation
    ) {
        TypeScriptWriter writer = context.getWriter();
        SymbolProvider symbolProvider = context.getSymbolProvider();
        HttpTrait httpTrait = operation.getTrait(HttpTrait.class)
            .orElseThrow(() -> new CodegenException("Missing @http trait"));
        
        // Get input/output shapes
        StructureShape inputShape = operation.getInput()
            .flatMap(id -> context.getModel().getShape(id, StructureShape.class))
            .orElse(null);
        
        StructureShape outputShape = operation.getOutput()
            .flatMap(id -> context.getModel().getShape(id, StructureShape.class))
            .orElse(null);
        
        Symbol inputSymbol = inputShape != null ? symbolProvider.toSymbol(inputShape) : null;
        Symbol outputSymbol = outputShape != null ? symbolProvider.toSymbol(outputShape) : null;
        
        // Write documentation
        if (operation.getDocumentation().isPresent()) {
            writer.writeDocs(operation.getDocumentation().get());
        }
        
        // Generate method
        String inputParam = inputSymbol != null ? "input: " + inputSymbol.getName() : "";
        writer.openBlock("async $L($L): Promise<$T> {", "}",
            operation.getId().getName(), inputParam, outputSymbol, () -> {
            
            // Build request
            writer.write("const request = this.build$LRequest(input);", 
                operation.getId().getName());
            
            // Send request
            writer.write("const response = await this.client.send(request);");
            
            // Handle errors
            writer.write("if (!response.ok) {");
            writer.indent();
            writer.write("throw this.parseErrorResponse(response);");
            writer.dedent();
            writer.write("}");
            
            // Parse response
            writer.write("return this.parse$LResponse(response);", 
                operation.getId().getName());
        });
        writer.write("");
        
        // Generate request builder
        generateRequestBuilder(context, operation, httpTrait, inputShape);
        
        // Generate response parser
        generateResponseParser(context, operation, httpTrait, outputShape);
    }
    
    private void generateRequestBuilder(
        ServiceGeneratorContext context,
        OperationShape operation,
        HttpTrait httpTrait,
        StructureShape inputShape
    ) {
        TypeScriptWriter writer = context.getWriter();
        
        writer.openBlock("private build$LRequest(input: any): Request {", "}",
            operation.getId().getName(), () -> {
            
            // Build URL with path parameters
            Set<String> uriLabels = httpTrait.getUriLabels();
            String uriTemplate = httpTrait.getUri();
            
            if (!uriLabels.isEmpty()) {
                writer.write("let path = $S;", uriTemplate);
                for (String label : uriLabels) {
                    // Find member that maps to this label
                    Optional<HttpBinding> binding = httpTrait.getRequestBinding(label);
                    String memberName = binding.map(HttpBinding::getMemberName).orElse(label);
                    
                    writer.write("path = path.replace('{$L}', String(input.$L));",
                        label, memberName);
                }
            } else {
                writer.write("const path = $S;", httpTrait.getUri());
            }
            
            // Build query parameters
            writer.write("const queryParams: Record<string, string> = {};");
            
            // Add query bindings
            for (MemberShape member : inputShape.getAllMembers().values()) {
                httpTrait.getRequestBinding(member.getMemberName())
                    .filter(b -> b.getLocation() == HttpBindingLocation.QUERY)
                    .ifPresent(binding -> {
                        writer.write("if (input.$L !== undefined) {", member.getMemberName());
                        writer.indent();
                        writer.write("queryParams[$S] = String(input.$L);",
                            binding.getHttpName(), member.getMemberName());
                        writer.dedent();
                        writer.write("}");
                    });
            }
            
            // Build query string
            writer.write("const queryString = Object.entries(queryParams)");
            writer.write("  .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)");
            writer.write("  .join('&');");
            writer.write("if (queryString) path += '?' + queryString;");
            
            // Build headers
            writer.write("const headers: Record<string, string> = {");
            writer.indent();
            writer.write("'Content-Type': 'application/json',");
            
            // Add header bindings
            for (MemberShape member : inputShape.getAllMembers().values()) {
                httpTrait.getRequestBinding(member.getMemberName())
                    .filter(b -> b.getLocation() == HttpBindingLocation.HEADER)
                    .ifPresent(binding -> {
                        writer.write("'$L': input.$L,", 
                            binding.getHttpName(), member.getMemberName());
                    });
            }
            
            writer.dedent();
            writer.write("};");
            
            // Build body
            writer.write("let body: any;");
            writer.write("if ('POST' === '$L' || 'PUT' === '$L') {", 
                httpTrait.getMethod(), httpTrait.getMethod());
            writer.indent();
            writer.write("body = JSON.stringify(input);");
            writer.dedent();
            writer.write("}");
            
            // Construct request
            writer.write("return new Request(this.config.endpoint + path, {");
            writer.indent();
            writer.write("method: '$L',", httpTrait.getMethod());
            writer.write("headers,");
            writer.write("body");
            writer.dedent();
            writer.write("});");
        });
    }
    
    private void generateResponseParser(
        ServiceGeneratorContext context,
        OperationShape operation,
        HttpTrait httpTrait,
        StructureShape outputShape
    ) {
        TypeScriptWriter writer = context.getWriter();
        
        writer.openBlock("private parse$LResponse(response: Response): $T {", "}",
            operation.getId().getName(), symbolProvider.toSymbol(outputShape), () -> {
            
            writer.write("return response.json();");
        });
    }
}
```

## Query Parameter Serialization

```java
// software/amazon/smithy/protocol-traits/src/main/java/QueryParamsSerializer.java

public class QueryParamsSerializer {
    
    /// Serialize member to query parameter
    public static void serializeMember(
        TypeScriptWriter writer,
        MemberShape member,
        Shape targetShape,
        String paramName
    ) {
        switch (targetShape.getType()) {
            case STRING:
            case INTEGER:
            case LONG:
            case BOOLEAN:
            case DOUBLE:
            case FLOAT:
                // Simple scalar
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("queryParams[$S] = String(input.$L);",
                    paramName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case LIST:
                // Serialize as repeated params or comma-separated
                ListShape list = (ListShape) targetShape;
                writer.write("if (input.$L !== undefined && input.$L.length > 0) {",
                    member.getMemberName(), member.getMemberName());
                writer.indent();
                writer.write("queryParams[$S] = input.$L.join(',');",
                    paramName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case TIMESTAMP:
                // Serialize as ISO 8601
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("queryParams[$S] = input.$L.toISOString();",
                    paramName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            default:
                throw new CodegenException("Unsupported query param type: " + targetShape.getType());
        }
    }
}
```

## Header Serialization

```java
// software/amazon/smithy/protocol-traits/src/main/java/HeaderSerializer.java

public class HeaderSerializer {
    
    /// Serialize member to HTTP header
    public static void serializeMember(
        TypeScriptWriter writer,
        MemberShape member,
        Shape targetShape,
        String headerName
    ) {
        switch (targetShape.getType()) {
            case STRING:
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("headers[$S] = input.$L;", headerName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case INTEGER:
            case LONG:
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("headers[$S] = String(input.$L);", headerName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case BOOLEAN:
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("headers[$S] = input.$L ? 'true' : 'false';", 
                    headerName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case TIMESTAMP:
                // RFC 7231 date format
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("headers[$S] = input.$L.toUTCString();", 
                    headerName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case LIST:
                // Comma-separated values
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("headers[$S] = input.$L.join(',');", 
                    headerName, member.getMemberName());
                writer.dedent();
                writer.write("}");
                break;
                
            case MAP:
                // Header prefix pattern
                MapShape map = (MapShape) targetShape;
                writer.write("if (input.$L !== undefined) {", member.getMemberName());
                writer.indent();
                writer.write("for (const [key, value] of Object.entries(input.$L)) {",
                    member.getMemberName());
                writer.indent();
                writer.write("headers[$S + key] = String(value);", headerName);
                writer.dedent();
                writer.write("}");
                writer.dedent();
                writer.write("}");
                break;
                
            default:
                throw new CodegenException("Unsupported header type: " + targetShape.getType());
        }
    }
}
```

## JSON Body Serialization

```java
// software/amazon/smithy/json-codegen/src/main/java/JsonSerializer.java

public class JsonSerializer {
    
    /// Generate JSON serializer for structure
    public static void generateSerializer(
        TypeScriptWriter writer,
        StructureShape shape,
        SymbolProvider symbolProvider,
        Model model
    ) {
        String functionName = "serialize" + shape.getId().getName();
        
        writer.openBlock("export function $L(obj: $T): any {", "}",
            functionName, symbolProvider.toSymbol(shape), () -> {
            
            writer.write("const result: any = {};");
            
            for (MemberShape member : shape.getAllMembers().values()) {
                Shape target = model.getShape(member.getTarget()).orElseThrow();
                String memberName = symbolProvider.getMemberName(member);
                String jsonName = member.getMemberName(); // Could be customized
                
                writer.write("if (obj.$L !== undefined) {", memberName);
                writer.indent();
                
                switch (target.getType()) {
                    case STRING:
                    case INTEGER:
                    case LONG:
                    case BOOLEAN:
                    case DOUBLE:
                    case FLOAT:
                        writer.write("result.$S = obj.$L;", jsonName, memberName);
                        break;
                        
                    case TIMESTAMP:
                        writer.write("result.$S = obj.$L.toISOString();", jsonName, memberName);
                        break;
                        
                    case LIST:
                        writer.write("result.$S = obj.$L.map(item => {", jsonName, memberName);
                        writer.indent();
                        generateListElementSerializer(writer, (ListShape) target, symbolProvider, model);
                        writer.dedent();
                        writer.write("});");
                        break;
                        
                    case MAP:
                        writer.write("result.$S = Object.fromEntries(", jsonName);
                        writer.indent();
                        writer.write("Object.entries(obj.$L).map(([k, v]) => {", memberName);
                        writer.indent();
                        generateMapValueSerializer(writer, (MapShape) target, symbolProvider, model);
                        writer.dedent();
                        writer.write("})");
                        writer.dedent();
                        writer.write(");");
                        break;
                        
                    case STRUCTURE:
                        writer.write("result.$S = serialize$L(obj.$L);",
                            jsonName, target.getId().getName(), memberName);
                        break;
                        
                    default:
                        writer.write("result.$S = obj.$L;", jsonName, memberName);
                }
                
                writer.dedent();
                writer.write("}");
            }
            
            writer.write("return result;");
        });
    }
    
    private static void generateListElementSerializer(
        TypeScriptWriter writer,
        ListShape list,
        SymbolProvider symbolProvider,
        Model model
    ) {
        Shape element = model.getShape(list.getMember().getTarget()).orElseThrow();
        
        if (element.getType() == ShapeType.STRUCTURE) {
            writer.write("return serialize$L(item);", element.getId().getName());
        } else if (element.getType() == ShapeType.TIMESTAMP) {
            writer.write("return item.toISOString();");
        } else {
            writer.write("return item;");
        }
    }
    
    private static void generateMapValueSerializer(
        TypeScriptWriter writer,
        MapShape map,
        SymbolProvider symbolProvider,
        Model model
    ) {
        Shape value = model.getShape(map.getValue().getTarget()).orElseThrow();
        
        if (value.getType() == ShapeType.STRUCTURE) {
            writer.write("return [k, serialize$L(v);", value.getId().getName());
        } else if (value.getType() == ShapeType.TIMESTAMP) {
            writer.write("return [k, v.toISOString()];");
        } else {
            writer.write("return [k, v];");
        }
    }
}
```

## JSON Body Deserialization

```java
// software/amazon/smithy/json-codegen/src/main/java/JsonDeserializer.java

public class JsonDeserializer {
    
    /// Generate JSON deserializer for structure
    public static void generateDeserializer(
        TypeScriptWriter writer,
        StructureShape shape,
        SymbolProvider symbolProvider,
        Model model
    ) {
        String functionName = "deserialize" + shape.getId().getName();
        Symbol outputSymbol = symbolProvider.toSymbol(shape);
        
        writer.openBlock("export function $L(data: any): $T {", "}",
            functionName, outputSymbol, () -> {
            
            writer.write("const result: any = {};");
            
            for (MemberShape member : shape.getAllMembers().values()) {
                Shape target = model.getShape(member.getTarget()).orElseThrow();
                String memberName = symbolProvider.getMemberName(member);
                String jsonName = member.getMemberName();
                
                writer.write("if (data.$L !== undefined && data.$L !== null) {", 
                    jsonName, jsonName);
                writer.indent();
                
                switch (target.getType()) {
                    case STRING:
                        writer.write("result.$L = String(data.$L);", memberName, jsonName);
                        break;
                        
                    case INTEGER:
                        writer.write("result.$L = Number(data.$L);", memberName, jsonName);
                        break;
                        
                    case LONG:
                        writer.write("result.$L = BigInt(data.$L);", memberName, jsonName);
                        break;
                        
                    case BOOLEAN:
                        writer.write("result.$L = Boolean(data.$L);", memberName, jsonName);
                        break;
                        
                    case DOUBLE:
                    case FLOAT:
                        writer.write("result.$L = Number(data.$L);", memberName, jsonName);
                        break;
                        
                    case TIMESTAMP:
                        writer.write("result.$L = new Date(data.$L);", memberName, jsonName);
                        break;
                        
                    case LIST:
                        writer.write("result.$L = (data.$L as Array<any>).map(item => {",
                            memberName, jsonName);
                        writer.indent();
                        generateListElementDeserializer(writer, (ListShape) target, symbolProvider, model);
                        writer.dedent();
                        writer.write("});");
                        break;
                        
                    case MAP:
                        writer.write("result.$L = Object.fromEntries(", memberName);
                        writer.indent();
                        writer.write("Object.entries(data.$L).map(([k, v]) => {", jsonName);
                        writer.indent();
                        generateMapValueDeserializer(writer, (MapShape) target, symbolProvider, model);
                        writer.dedent();
                        writer.write("})");
                        writer.dedent();
                        writer.write(");");
                        break;
                        
                    case STRUCTURE:
                        writer.write("result.$L = deserialize$L(data.$L);",
                            memberName, target.getId().getName(), jsonName);
                        break;
                        
                    default:
                        writer.write("result.$L = data.$L;", memberName, jsonName);
                }
                
                writer.dedent();
                writer.write("}");
            }
            
            writer.write("return result as $T;", outputSymbol);
        });
    }
}
```

## Error Handling

```java
// software/amazon/smithy/codegen/core/ErrorGenerator.java

public class ErrorGenerator {
    
    /// Generate error class for operation
    public static void generateErrorClass(
        TypeScriptWriter writer,
        StructureShape errorShape,
        SymbolProvider symbolProvider,
        Model model
    ) {
        String errorName = errorShape.getId().getName();
        HttpErrorTrait httpError = errorShape.getTrait(HttpErrorTrait.class).orElse(null);
        int statusCode = httpError != null ? httpError.getCode() : 500;
        
        // Generate error class
        writer.openBlock("export class $L extends Error {", "}", errorName, () -> {
            
            // Properties
            writer.write("readonly code = $S;", errorName);
            writer.write("readonly statusCode = $L;", statusCode);
            
            for (MemberShape member : errorShape.getAllMembers().values()) {
                Symbol memberSymbol = symbolProvider.toSymbol(member);
                writer.write("$L: $T;", member.getMemberName(), memberSymbol);
            }
            
            writer.write("");
            
            // Constructor
            writer.openBlock("constructor(message: string, opts: any = {}) {", "}", () -> {
                writer.write("super(message);");
                writer.write("this.name = $S;", errorName);
                
                for (MemberShape member : errorShape.getAllMembers().values()) {
                    String memberName = member.getMemberName();
                    writer.write("this.$L = opts.$L;", memberName, memberName);
                }
            });
        });
        writer.write("");
        
        // Generate error parser
        generateErrorParser(writer, errorShape, symbolProvider, model);
    }
    
    private static void generateErrorParser(
        TypeScriptWriter writer,
        StructureShape errorShape,
        SymbolProvider symbolProvider,
        Model model
    ) {
        String errorName = errorShape.getId().getName();
        
        writer.openBlock("export function parse$L(data: any): $L {", "}",
            errorName, errorName, () -> {
            
            writer.write("return new $L(data.message ?? 'Error', {", errorName);
            writer.indent();
            
            for (MemberShape member : errorShape.getAllMembers().values()) {
                String memberName = member.getMemberName();
                Shape target = model.getShape(member.getTarget()).orElseThrow();
                
                if (target.getType() == ShapeType.STRING) {
                    writer.write("$L: data.$L,", memberName, memberName);
                } else if (target.getType() == ShapeType.TIMESTAMP) {
                    writer.write("$L: data.$L ? new Date(data.$L) : undefined,", 
                        memberName, memberName, memberName);
                } else {
                    writer.write("$L: data.$L,", memberName, memberName);
                }
            }
            
            writer.dedent();
            writer.write("});");
        });
    }
    
    /// Generate error dispatcher
    public static void generateErrorDispatcher(
        TypeScriptWriter writer,
        OperationShape operation,
        SymbolProvider symbolProvider,
        Model model
    ) {
        writer.openBlock("function parseErrorResponse(response: Response): Error {", "}", () -> {
            
            writer.write("const statusCode = response.status;");
            writer.write("const contentType = response.headers.get('content-type') || '';");
            writer.write("");
            
            // Parse body
            writer.write("let data: any;");
            writer.write("if (contentType.includes('application/json')) {");
            writer.indent();
            writer.write("data = response.json();");
            writer.dedent();
            writer.write("} else {");
            writer.indent();
            writer.write("data = { message: response.statusText };");
            writer.dedent();
            writer.write("}");
            writer.write("");
            
            // Dispatch based on status code
            writer.write("switch (statusCode) {");
            
            for (ShapeId errorId : operation.getErrors()) {
                StructureShape error = model.getShape(errorId, StructureShape.class).orElseThrow();
                HttpErrorTrait httpError = error.getTrait(HttpErrorTrait.class).orElseThrow();
                
                writer.write("case $L:", httpError.getCode());
                writer.indent();
                writer.write("return parse$L(data);", error.getId().getName());
                writer.dedent();
            }
            
            writer.write("default:");
            writer.indent();
            writer.write("return new Error(data.message);");
            writer.dedent();
            
            writer.write("}");
        });
    }
}
```

## Protocol Test Framework

```java
// software/amazon/smithy/protocol-tests/src/main/java/ProtocolTestGenerator.java

public class ProtocolTestGenerator {
    
    /// Generate protocol test cases
    public static void generateTests(
        TypeScriptWriter writer,
        Model model,
        List<ProtocolTestCase> testCases
    ) {
        writer.writeLine("import { describe, it, expect } from 'vitest';");
        writer.writeLine("import { serializeRequest, parseResponse } from './serialization';");
        writer.writeLine("");
        
        for (ProtocolTestCase testCase : testCases) {
            writer.openBlock("describe('$L', () => {", "}", testCase.getName(), () -> {
                
                // Request test
                if (testCase.getRequestTest().isPresent()) {
                    RequestTest reqTest = testCase.getRequestTest().get();
                    
                    writer.openBlock("it('serializes request correctly', () => {", "}", () -> {
                        
                        writer.write("const input = $L;", reqTest.getInput());
                        writer.write("const request = serializeRequest(input);");
                        writer.write("");
                        writer.write("expect(request.method).toBe($S);", reqTest.getMethod());
                        writer.write("expect(request.path).toBe($S);", reqTest.getPath());
                        writer.write("expect(request.headers).toEqual($L);", reqTest.getExpectedHeaders());
                        writer.write("expect(request.body).toEqual($L);", reqTest.getExpectedBody());
                    });
                }
                
                // Response test
                if (testCase.getResponseTest().isPresent()) {
                    ResponseTest respTest = testCase.getResponseTest().get();
                    
                    writer.openBlock("it('parses response correctly', () => {", "}", () -> {
                        
                        writer.write("const response = {");
                        writer.indent();
                        writer.write("statusCode: $L,", respTest.getStatusCode());
                        writer.write("headers: $L,", respTest.getHeaders());
                        writer.write("body: $L", respTest.getBody());
                        writer.dedent();
                        writer.write("};");
                        writer.write("");
                        writer.write("const output = parseResponse(response);");
                        writer.write("expect(output).toEqual($L);", respTest.getExpectedOutput());
                    });
                }
            });
        }
    }
}

/// Protocol test case
public class ProtocolTestCase {
    private final String name;
    private final OperationShape operation;
    private final Optional<RequestTest> requestTest;
    private final Optional<ResponseTest> responseTest;
    
    // Getters...
}

/// Request test definition
public class RequestTest {
    private final String input;           // Input object as code
    private final String method;          // Expected HTTP method
    private final String path;            // Expected path
    private final Map<String, String> expectedHeaders;
    private final String expectedBody;    // Expected body JSON
    
    // Getters...
}

/// Response test definition
public class ResponseTest {
    private final int statusCode;
    private final Map<String, String> headers;
    private final String body;
    private final String expectedOutput;  // Expected output object
    
    // Getters...
}
```

## Conclusion

Protocol generation in Smithy provides:

1. **HTTP Bindings**: Path, query, header, body parameter mapping
2. **Serialization**: Type-safe request serialization
3. **Deserialization**: Response parsing with error handling
4. **Error Mapping**: HTTP status code to error type mapping
5. **Protocol Tests**: Generated test cases for validation
