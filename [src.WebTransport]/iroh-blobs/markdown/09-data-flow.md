---
title: Data Flow — End-to-End Blob Transfer Sequences
---

# Data Flow — End-to-End Blob Transfer Sequences

This document traces the complete data flow from blob addition to remote retrieval.

## Adding a Blob to the Store

```mermaid
sequenceDiagram
    participant App as Application
    participant Blobs as Blobs API
    participant Store as FsStore
    import Import as Import Pipeline
    participant Bao as bao-tree
    participant Db as Database Actor

    App->>Blobs: add_path("/file.txt")
    Blobs->>Import: import_path("/file.txt")
    Import->>Bao: compute BLAKE3 hash + outboard
    Bao-->>Import: (hash, outboard)
    Import->>Store: write content to blob dir
    Import->>Db: write EntryState (Complete)
    Db-->>Store: entry committed
    Store-->>Blobs: AddResult { hash, size }
    Blobs-->>App: AddResult
```

Source: `iroh-blobs/src/api/blobs.rs:1` (add_path), `iroh-blobs/src/store/fs/import.rs:1` (import_path).

## Getting a Blob from Remote

```mermaid
sequenceDiagram
    participant App as Application
    participant Store as Store
    participant Client as Client FSM
    participant Provider as Provider (remote)
    participant RemoteStore as Remote Store

    App->>Store: get_blob(hash)
    Store->>Client: initiate GetRequest
    Client->>Provider: GetRequest { hash, ranges: All }
    Provider->>RemoteStore: lookup hash
    RemoteStore-->>Provider: blob found
    Provider->>Client: BlobHeader { hash, size }
    loop per chunk
        Provider->>Client: chunk + outboard
        Client->>Bao: verify chunk
        Bao-->>Client: valid
        Client->>Client: store chunk
    end
    Client->>Client: blob complete
    Client-->>Store: blob stored
    Store-->>App: GetResult { hash, size }
```

Source: `iroh-blobs/src/get.rs:1` (client FSM), `iroh-blobs/src/provider.rs:1` (provider).

## Partial Transfer with Range Request

```mermaid
sequenceDiagram
    participant Client as Client
    participant Provider as Provider
    participant Store as Store

    Client->>Provider: GetRequest { hash, ranges: Sparse }
    Provider->>Store: lookup hash
    Provider->>Client: BlobHeader { hash, size }
    Provider->>Client: chunks at requested positions
    Client->>Client: verify each chunk
    Note over Client: partial blob stored<br/>bitfield updated
    Client-->>Client: transfer complete
```

Source: `iroh-blobs/src/protocol/range_spec.rs:1` (RangeSpec), `iroh-blobs/src/get.rs:1` (partial transfer).

## HashSeq (Collection) Transfer

```mermaid
sequenceDiagram
    participant Client as Client
    participant Provider as Provider

    Client->>Provider: GetRequest { hash, format: HashSeq }
    Provider->>Client: BlobHeader (root hash, HashSeq)
    Provider->>Client: HashSeq content (list of child hashes)
    Client->>Client: parse HashSeq
    loop per child hash
        Client->>Provider: GetRequest { child_hash }
        Provider->>Client: child BlobHeader + content
        Client->>Client: verify and store child
    end
    Client->>Client: all children received
```

Source: `iroh-blobs/src/hashseq.rs:1` (HashSeq), `iroh-blobs/src/get/request.rs:1` (get_hash_seq_and_sizes).

## Push (Reverse Transfer)

```mermaid
sequenceDiagram
    participant Sender as Sender (pushing)
    participant Provider as Provider (receiving)
    participant Store as Store

    Sender->>Provider: PushRequest { hash }
    Sender->>Provider: BlobHeader { hash, size }
    loop per chunk
        Sender->>Provider: chunk + outboard
        Provider->>Bao: verify chunk
        Bao-->>Provider: valid
        Provider->>Store: store chunk
    end
    Provider->>Sender: acknowledgment
    Store->>Store: EntryState written
```

Source: `iroh-blobs/src/provider.rs:1` (handle_push).

## Related Documents

- [Get Client](../markdown/07-get-client.md) — Client FSM details
- [Provider](../markdown/08-provider.md) — Server-side handling
- [Protocol](../markdown/03-protocol.md) — Request format
