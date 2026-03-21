# Hoarder Service - Distributed Storage Deep Dive

## Overview

**Hoarder** is Taubyte's distributed object storage service. It provides content-addressable storage (CAS), artifact management for WASM modules, and P2P replication capabilities.

---

## Service Architecture

### Core Components

```
tau/services/hoarder/
├── service.go           # Main service implementation
├── type.go              # Service type definitions
├── api.go               # HTTP API endpoints
├── pubsub.go            # Pub/sub integration
├── stream.go            # P2P stream handling
├── helpers.go           # Utility functions
├── common/
│   └── iface.go         # Interface definitions
├── dream/
│   └── init.go          # Dream integration
└── tests/
    └── [integration tests]
```

### Service Structure

```go
// tau/services/hoarder/type.go
type Service struct {
    ctx         context.Context
    node        peer.Node
    clientNode  peer.Node
    config      *tauConfig.Node
    stream      *streams.Service
    store       *ContentStore
    rareCache   *RareCache
    stash       *Stash
    replication *ReplicationManager
}

type ContentStore struct {
    path    string
    blocks  sync.Map  // map[string]*Block
    ds      ds.Datastore
}

type Block struct {
    CID     string
    Data    []byte
    Size    int
    Refs    int
    Created time.Time
}
```

---

## Content-Addressable Storage

### CID-Based Storage

```go
// tau/services/hoarder/service.go
type ContentID struct {
    codec  uint64
    hash   string
    format string
}

func (s *ContentStore) Put(data []byte) (string, error) {
    // Calculate CID (Content Identifier)
    hash := sha256.Sum256(data)
    cid := encodeCID(hash[:], CIDRaw)

    // Store block
    block := &Block{
        CID:     cid,
        Data:    data,
        Size:    len(data),
        Created: time.Now(),
    }

    s.blocks.Store(cid, block)

    // Persist to datastore
    s.ds.Put([]byte(cid), data)

    return cid, nil
}

func (s *ContentStore) Get(cid string) ([]byte, error) {
    // Check memory cache
    if block, ok := s.blocks.Load(cid); ok {
        return block.(*Block).Data, nil
    }

    // Load from datastore
    data, err := s.ds.Get([]byte(cid))
    if err != nil {
        return nil, err
    }

    // Verify integrity
    hash := sha256.Sum256(data)
    expectedCID := encodeCID(hash[:], CIDRaw)
    if expectedCID != cid {
        return nil, fmt.Errorf("content integrity check failed")
    }

    // Cache in memory
    s.blocks.Store(cid, &Block{
        CID:  cid,
        Data: data,
        Size: len(data),
    })

    return data, nil
}
```

### CID Encoding

```go
// CID format: multihash + codec
func encodeCID(hash []byte, codec uint64) string {
    // Use multihash format
    mh := multihash.Encode(hash, multihash.SHA2_256)
    return cid.NewCIDV1(codec, mh).String()
}

func decodeCID(cidStr string) (*ContentID, error) {
    c, err := cid.Decode(cidStr)
    if err != nil {
        return nil, err
    }

    return &ContentID{
        codec:  c.Type(),
        hash:   c.Hash().B58String(),
        format: c.Version(),
    }, nil
}
```

---

## Rare Cache (Hot Storage)

### LRU Cache Implementation

```go
// tau/services/hoarder/rare.go
type RareCache struct {
    maxSize    int64
    currentSize int64
    items      *list.List
    lookup     map[string]*list.Element
    mutex      sync.RWMutex
}

type cacheItem struct {
    key   string
    value []byte
    size  int64
}

func NewRareCache(maxSize int64) *RareCache {
    return &RareCache{
        maxSize: maxSize,
        items:   list.New(),
        lookup:  make(map[string]*list.Element),
    }
}

func (rc *RareCache) Get(key string) ([]byte, bool) {
    rc.mutex.Lock()
    defer rc.mutex.Unlock()

    if elem, ok := rc.lookup[key]; ok {
        // Move to front (most recently used)
        rc.items.MoveToFront(elem)
        item := elem.Value.(*cacheItem)
        return item.value, true
    }

    return nil, false
}

func (rc *RareCache) Put(key string, value []byte) {
    rc.mutex.Lock()
    defer rc.mutex.Unlock()

    size := int64(len(value))

    // Evict if necessary
    for rc.currentSize+size > rc.maxSize {
        rc.evict()
    }

    // Add new item
    item := &cacheItem{key: key, value: value, size: size}
    elem := rc.items.PushFront(item)
    rc.lookup[key] = elem
    rc.currentSize += size
}

func (rc *RareCache) evict() {
    elem := rc.items.Back()
    if elem == nil {
        return
    }

    item := elem.Value.(*cacheItem)
    rc.items.Remove(elem)
    delete(rc.lookup, item.key)
    rc.currentSize -= item.size
}
```

---

## Stash (Cold Storage)

### Long-term Storage

```go
// tau/services/hoarder/stash.go
type Stash struct {
    path    string
    index   ds.Datastore
    gc      *GarbageCollector
}

func (s *Stash) Store(cid string, data []byte) error {
    // Write to file system
    filePath := s.getPathForCID(cid)
    dir := filepath.Dir(filePath)

    if err := os.MkdirAll(dir, 0755); err != nil {
        return err
    }

    if err := os.WriteFile(filePath, data, 0644); err != nil {
        return err
    }

    // Update index
    s.index.Put([]byte(cid), []byte(filePath))

    return nil
}

func (s *Stash) Retrieve(cid string) ([]byte, error) {
    // Get file path from index
    filePathData, err := s.index.Get([]byte(cid))
    if err != nil {
        return nil, err
    }

    filePath := string(filePathData)

    // Read from file system
    return os.ReadFile(filePath)
}

func (s *Stash) getPathForCID(cid string) string {
    // Use CID prefix for directory structure
    // e.g., QmABC123... -> stash/Qm/AB/QmABC123...
    return filepath.Join(s.path, cid[:2], cid[2:4], cid)
}
```

---

## P2P Replication

### Block Exchange

```go
// tau/services/hoarder/client.go
type ReplicationManager struct {
    node     peer.Node
    store    *ContentStore
    peers    map[peer.ID]*PeerState
}

func (rm *ReplicationManager) replicateBlock(cid string, data []byte) {
    // Find peers who might want this block
    interestedPeers := rm.findInterestedPeers(cid)

    for _, peer := range interestedPeers {
        go rm.sendBlock(peer, cid, data)
    }
}

func (rm *ReplicationManager) requestBlock(peer peer.ID, cid string) ([]byte, error) {
    stream, err := rm.node.NewStream(peer, HoarderProtocol)
    if err != nil {
        return nil, err
    }
    defer stream.Close()

    // Send request
    json.NewEncoder(stream).Encode(BlockRequest{CID: cid})

    // Read response
    var resp BlockResponse
    json.NewDecoder(stream).Decode(&resp)

    if resp.Error != "" {
        return nil, fmt.Errorf(resp.Error)
    }

    return resp.Data, nil
}

func (rm *ReplicationManager) handleBlockRequest(stream network.Stream) {
    var req BlockRequest
    json.NewDecoder(stream).Decode(&req)

    data, err := rm.store.Get(req.CID)
    if err != nil {
        json.NewEncoder(stream).Encode(BlockResponse{Error: err.Error()})
        return
    }

    json.NewEncoder(stream).Encode(BlockResponse{
        CID:  req.CID,
        Data: data,
    })
}
```

---

## HTTP API

### API Endpoints

```go
// tau/services/hoarder/api.go
func (srv *Service) setupHTTPRoutes() {
    // Content operations
    srv.http.HandleFunc("/api/hoarder/put", srv.handlePut)
    srv.http.HandleFunc("/api/hoarder/get/{cid}", srv.handleGet)
    srv.http.HandleFunc("/api/hoarder/delete/{cid}", srv.handleDelete)

    // List operations
    srv.http.HandleFunc("/api/hoarder/list", srv.handleList)
    srv.http.HandleFunc("/api/hoarder/list/{prefix}", srv.handleListPrefix)

    // Stats
    srv.http.HandleFunc("/api/hoarder/stats", srv.handleStats)
}

func (srv *Service) handlePut(w http.ResponseWriter, r *http.Request) {
    data, err := io.ReadAll(r.Body)
    if err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }

    cid, err := srv.store.Put(data)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    json.NewEncoder(w).Encode(PutResponse{CID: cid})
}

func (srv *Service) handleGet(w http.ResponseWriter, r *http.Request) {
    cid := chi.URLParam(r, "cid")

    data, err := srv.store.Get(cid)
    if err != nil {
        http.Error(w, err.Error(), http.StatusNotFound)
        return
    }

    w.Header().Set("Content-Type", "application/octet-stream")
    w.Header().Set("X-Content-CID", cid)
    w.Write(data)
}
```

---

## P2P Client

### Client Implementation

```go
// tau/clients/p2p/hoarder/client.go
type Client struct {
    node     peer.Node
    protocol string
}

func New(ctx context.Context, node peer.Node) (*Client, error) {
    return &Client{
        node:     node,
        protocol: HoarderProtocol,
    }, nil
}

func (c *Client) Put(data []byte) (string, error) {
    stream, err := c.node.NewStream(c.protocol)
    if err != nil {
        return "", err
    }
    defer stream.Close()

    json.NewEncoder(stream).Encode(PutRequest{Data: data})

    var resp PutResponse
    json.NewDecoder(stream).Decode(&resp)
    return resp.CID, nil
}

func (c *Client) Get(cid string) ([]byte, error) {
    stream, err := c.node.NewStream(c.protocol)
    if err != nil {
        return nil, err
    }
    defer stream.Close()

    json.NewEncoder(stream).Encode(GetRequest{CID: cid})

    var resp GetResponse
    json.NewDecoder(stream).Decode(&resp)

    if resp.Error != "" {
        return nil, fmt.Errorf(resp.Error)
    }

    return resp.Data, nil
}

func (c *Client) List(prefix string) ([]string, error) {
    stream, err := c.node.NewStream(c.protocol)
    if err != nil {
        return nil, err
    }
    defer stream.Close()

    json.NewEncoder(stream).Encode(ListRequest{Prefix: prefix})

    var resp ListResponse
    json.NewDecoder(stream).Decode(&resp)

    return resp.CIDs, nil
}
```

---

## Testing

### Integration Tests

```go
// tau/clients/p2p/hoarder/tests/p2p_test.go
func TestHoarderP2P(t *testing.T) {
    // Start Hoarder service
    config := createTestConfig()
    hoarder, err := New(ctx, config)

    // Create client
    client, err := p2p.NewClient(ctx, node)

    // Put content
    testData := []byte("Hello, Taubyte!")
    cid, err := client.Put(testData)
    if err != nil {
        t.Fatal(err)
    }

    // Get content
    retrieved, err := client.Get(cid)
    if err != nil {
        t.Fatal(err)
    }

    if string(retrieved) != string(testData) {
        t.Error("Retrieved data doesn't match")
    }
}
```

---

## Related Documents

- `../exploration.md` - Main exploration
- `monkey.md` - Function execution (WASM storage)
- `patrick.md` - Build scheduler (source storage)
- `../rust-revision.md` - Rust implementation guide
