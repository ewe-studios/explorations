---
title: "copyparty Data Flows"
description: "End-to-end request flows with sequence diagrams"
---

# copyparty Data Flows

This document shows end-to-end request flows through the copyparty system.

## HTTP Request Flow

### Standard File Request

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant TcpSrv
    participant HttpSrv
    participant HttpConn
    participant HttpCli
    participant VFS
    participant Auth
    participant FS

    Client->>TcpSrv: TCP connect
    TcpSrv->>HttpSrv: accept()
    HttpSrv->>HttpConn: spawn(conn, addr)
    
    Client->>HttpConn: GET /pics/photo.jpg HTTP/1.1
    HttpConn->>HttpCli: spawn()
    
    HttpCli->>HttpCli: parse headers
    HttpCli->>VFS: get("/pics/photo.jpg")
    VFS-->>HttpCli: VFS node
    
    HttpCli->>Auth: can_read(user)
    Auth-->>HttpCli: True/False
    
    alt Not authorized
        HttpCli-->>Client: 403 Forbidden
    else Authorized
        HttpCli->>FS: open(fspath)
        FS-->>HttpCli: file descriptor
        
        HttpCli->>HttpCli: sendfile() or read/send
        HttpCli-->>Client: HTTP/1.1 200 OK<br/>Content-Type: image/jpeg<br/><br/>[file data]
        
        HttpCli->>FS: close()
    end
    
    opt Keep-Alive
        HttpConn->>Client: wait for next request
    end
```

### Upload Flow (up2k Protocol)

```mermaid
sequenceDiagram
    autonumber
    participant Browser
    participant HttpCli
    participant Up2k
    participant Registry
    participant FS
    participant DB

    Note over Browser,DB: Phase 1: Initialize Upload
    
    Browser->>HttpCli: POST /up2k?init<br/>{filename, size, hash}
    HttpCli->>Up2k: create_upload(wark)
    Up2k->>Registry: check_existing(hash)
    Registry-->>Up2k: exists? yes/no
    
    alt Duplicate file
        Up2k->>FS: hardlink(existing, newpath)
        Up2k-->>HttpCli: {status: "duplicate", path}
        HttpCli-->>Browser: 201 Created<br/>{complete: true}
    else New file
        Up2k->>FS: create temp directory
        Up2k-->>HttpCli: {session, chunks_needed}
        HttpCli-->>Browser: 200 OK<br/>{chunks: [0,1,2,...]}
    end
    
    Note over Browser,DB: Phase 2: Upload Chunks
    
    loop For each chunk
        Browser->>HttpCli: PUT /up2k?w=xxx&c=n<br/>[binary chunk]
        HttpCli->>Up2k: store_chunk(wark, n, data)
        Up2k->>FS: write(chunk_path, data)
        Up2k->>Registry: mark_received(n)
        HttpCli-->>Browser: 200 OK<br/>{received: n}
    end
    
    Note over Browser,DB: Phase 3: Finalize
    
    Browser->>HttpCli: POST /up2k?finalize<br/>{wark, filename}
    HttpCli->>Up2k: finalize(wark, filename)
    
    Up2k->>FS: hash_verify(chunks)
    Up2k->>FS: assemble(chunks) -> final_file
    
    Up2k->>Registry: add_file(hash, path)
    Up2k->>DB: INSERT metadata
    
    Up2k->>FS: remove temp chunks
    
    Up2k-->>HttpCli: {status: "complete", path}
    HttpCli-->>Browser: 201 Created<br/>{url: "/pics/photo.jpg"}
```

## Directory Listing Flow

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant HttpCli
    participant VFS
    participant FS
    participant Template

    Client->>HttpCli: GET /pics/?json
    
    HttpCli->>VFS: get("/pics")
    VFS-->>HttpCli: VFS node for /pics
    
    HttpCli->>FS: listdir(realpath)
    FS-->>HttpCli: [file1, file2, ...]
    
    HttpCli->>HttpCli: filter_dotfiles(files)
    HttpCli->>HttpCli: apply_permissions(files)
    HttpCli->>HttpCli: sort_files(files)
    
    alt JSON format
        HttpCli->>HttpCli: format_json(files)
        HttpCli-->>Client: 200 OK<br/>Content-Type: application/json<br/>{files: [...]}
    else HTML format
        HttpCli->>Template: j2s("browser", files)
        Template-->>HttpCli: rendered HTML
        HttpCli-->>Client: 200 OK<br/>Content-Type: text/html<br/><html>...</html>
    else XML format
        HttpCli->>HttpCli: format_xml(files)
        HttpCli-->>Client: 200 OK<br/>Content-Type: application/xml<br/><listing>...</listing>
    end
```

## Authentication Flow

### HTTP Basic Auth

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant HttpCli
    participant AuthSrv
    participant Session

    Client->>HttpCli: GET /pics/ (no auth)
    
    HttpCli->>AuthSrv: check_auth(ip)
    AuthSrv-->>HttpCli: {authorized: false, required: true}
    
    HttpCli-->>Client: 401 Unauthorized<br/>WWW-Authenticate: Basic realm="a"
    
    Client->>HttpCli: GET /pics/<br/>Authorization: Basic base64(user:pass)
    
    HttpCli->>AuthSrv: verify_creds(user, pass)
    AuthSrv->>AuthSrv: hash_password(pass)
    AuthSrv->>AuthSrv: compare(hash, stored_hash)
    AuthSrv-->>HttpCli: {valid: true, user}
    
    alt Valid credentials
        HttpCli->>Session: create_session(user)
        Session-->>HttpCli: session_id
        HttpCli-->>Client: 200 OK<br/>Set-Cookie: cppwd=session_id<br/><html>...</html>
    else Invalid credentials
        HttpCli-->>Client: 401 Unauthorized
    end
```

### Cookie-based Session

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant HttpCli
    participant Session

    Note over Client,Session: Subsequent requests with session cookie
    
    Client->>HttpCli: GET /pics/<br/>Cookie: cppwd=session_id
    
    HttpCli->>Session: lookup(session_id)
    Session-->>HttpCli: {user: "alice", expires: "..."}
    
    alt Valid session
        HttpCli->>HttpCli: set user context
        HttpCli->>HttpCli: process request
        HttpCli-->>Client: 200 OK<br/>[content]
    else Expired/invalid session
        HttpCli-->>Client: 302 Redirect<br/>Location: /?login
    end
```

## Thumbnail Flow

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant HttpCli
    participant ThumbCli
    participant Broker
    participant ThumbSrv
    participant FS

    Client->>HttpCli: GET /thumbs/pics/photo.jpg
    
    HttpCli->>FS: check thumbnail cache
    
    alt Thumbnail exists
        FS-->>HttpCli: cached thumbnail
        HttpCli-->>Client: 200 OK<br/>Content-Type: image/jpeg<br/>[thumbnail data]
    else Not cached
        HttpCli->>ThumbCli: get_thumb(fspath)
        
        ThumbCli->>Broker: ask("thumbsrv.get", fspath)
        Broker->>ThumbSrv: dispatch
        
        ThumbSrv->>FS: check source file
        FS-->>ThumbSrv: file data
        
        alt Image file
            ThumbSrv->>ThumbSrv: PIL.open() or vips.Image.new_from_file()
            ThumbSrv->>ThumbSrv: resize()
            ThumbSrv->>ThumbSrv: save(format)
        else Video file
            ThumbSrv->>ThumbSrv: FFmpeg -ss 1 -vframes 1
            ThumbSrv->>ThumbSrv: PIL/vips resize
        end
        
        ThumbSrv->>FS: save thumbnail
        ThumbSrv-->>Broker: thumbnail data
        Broker-->>ThumbCli: result
        ThumbCli-->>HttpCli: thumbnail bytes
        
        HttpCli-->>Client: 200 OK<br/>Content-Type: image/webp<br/>[thumbnail]
    end
```

## WebDAV Flow (PROPFIND)

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant HttpCli
    participant VFS
    participant FS

    Client->>HttpCli: PROPFIND /pics/<br/>Depth: 1
    
    HttpCli->>VFS: get("/pics")
    VFS-->>HttpCli: VFS node
    
    HttpCli->>HttpCli: check DAV permission
    
    HttpCli->>FS: listdir()
    FS-->>HttpCli: [entries]
    
    loop For each entry
        HttpCli->>FS: stat(entry)
        FS-->>HttpCli: stat_result
    end
    
    HttpCli->>HttpCli: generate_dav_xml(files)
    
    HttpCli-->>Client: 207 Multi-Status<br/>Content-Type: application/xml<br/>&lt;multistatus&gt;...&lt;/multistatus&gt;
```

## Alternative Protocol Flows

### SFTP Connection

```mermaid
sequenceDiagram
    autonumber
    participant Client as "SFTP Client"
    participant SftpSrv
    participant VFS
    participant FS

    Client->>SftpSrv: SSH connect
    SftpSrv->>SftpSrv: check_allow_ip()
    
    alt Anonymous mode
        SftpSrv->>VFS: authenticate("LEELOO_DALLAS")
    else Auth required
        Client->>SftpSrv: password_auth(user, pass)
        SftpSrv->>SftpSrv: verify_creds()
    end
    
    Client->>SftpSrv: opendir("/pics")
    SftpSrv->>VFS: get("/pics")
    VFS-->>SftpSrv: VFS node
    
    SftpSrv->>FS: listdir(realpath)
    FS-->>SftpSrv: [entries]
    
    SftpSrv-->>Client: [file list]
    
    Client->>SftpSrv: open("photo.jpg")
    SftpSrv->>FS: open(realpath)
    FS-->>SftpSrv: file handle
    
    loop Read chunks
        Client->>SftpSrv: read(handle, offset, length)
        SftpSrv->>FS: pread()
        FS-->>SftpSrv: data
        SftpSrv-->>Client: data
    end
```

## Aha: Parallel Upload Design

**Key insight:** The up2k protocol enables true parallel chunked uploads.

```mermaid
sequenceDiagram
    participant Client as "Browser (3 threads)"
    participant Server as "copyparty"
    participant FS

    Note over Client,FS: Traditional Upload (sequential)
    
    Client->>Server: PUT file [0-1MB]
    Server->>FS: write
    Server-->>Client: OK
    
    Client->>Server: PUT file [1-2MB]
    Server->>FS: write
    Server-->>Client: OK
    
    Note over Client,FS: up2k Upload (parallel)
    
    par Parallel chunk uploads
        Client->>Server: PUT chunk 0
        Server->>FS: write chunk0.tmp
    and
        Client->>Server: PUT chunk 2
        Server->>FS: write chunk2.tmp
    and
        Client->>Server: PUT chunk 1
        Server->>FS: write chunk1.tmp
    end
    
    Server-->>Client: All OK
    
    Client->>Server: POST finalize
    Server->>FS: assemble chunks
    Server-->>Client: Complete
```

This allows:
1. **Resumability**: Only upload missing chunks
2. **Parallelism**: Multiple chunks simultaneously
3. **Browser optimization**: Use multiple HTTP connections
4. **Network resilience**: Failed chunks don't restart whole upload

## Error Flows

### Permission Denied

```mermaid
sequenceDiagram
    participant Client
    participant HttpCli
    participant VFS

    Client->>HttpCli: PUT /pics/upload.jpg
    
    HttpCli->>VFS: get("/pics")
    VFS-->>HttpCli: VFS node
    
    HttpCli->>HttpCli: check write permission
    
    alt No write permission
        HttpCli->>HttpCli: log("403: %s tried to upload" % user)
        HttpCli-->>Client: 403 Forbidden<br/>&lt;html&gt;not allowed&lt;/html&gt;
    end
```

### File Not Found

```mermaid
sequenceDiagram
    participant Client
    participant HttpCli
    participant VFS
    participant FS

    Client->>HttpCli: GET /pics/deleted.jpg
    
    HttpCli->>VFS: get("/pics/deleted.jpg")
    VFS-->>HttpCli: VFS node
    
    HttpCli->>FS: exists(realpath)
    FS-->>HttpCli: False
    
    HttpCli->>HttpCli: check for moved/renamed in registry
    HttpCli-->>Client: 404 Not Found<br/>(or 410 Gone if known deleted)
```

## Complete Upload-to-Display Flow

```mermaid
flowchart TB
    subgraph Upload["Upload Phase"]
        A["Browser selects file"] --> B["Calculate file hash"]
        B --> C{"File exists?"}
        C -->|Yes| D["Instant dedup"]
        C -->|No| E["Chunk file"]
        E --> F["Upload chunks"]
        F --> G["Server assembles"]
        G --> H["Generate thumbnail"]
        H --> I["Extract metadata"]
        I --> J["Index in database"]
    end
    
    subgraph Display["Display Phase"]
        K["User browses folder"] --> L["Request file list"]
        L --> M["Query metadata DB"]
        M --> N["Generate HTML/JSON"]
        N --> O["Browser renders"]
        O --> P{"Thumbnail ready?"}
        P -->|Yes| Q["Show thumbnail"]
        P -->|No| R["Show icon"]
        R --> S["Request thumbnail"]
        S --> T["Generate thumbnail"]
        T --> U["Cache & return"]
        U --> Q
    end
    
    J --> K
```

## Next Document

[README.md](README.md) — Table of contents and index.
