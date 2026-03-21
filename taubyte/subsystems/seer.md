# Seer Service - DNS & Discovery Deep Dive

## Overview

**Seer** is Taubyte's DNS resolution, service discovery, and monitoring service. It provides authoritative DNS services, geo-location routing, heartbeat monitoring, and oracle capabilities for external data.

---

## Service Architecture

### Core Components

```
tau/services/seer/
├── service.go           # Main service implementation
├── type.go              # Service type definitions
├── dns.go               # DNS server implementation
├── dns_helpers.go       # DNS utility functions
├── dns_http.go          # DNS HTTP endpoints
├── geo_api.go           # Geo-location API
├── geo_http.go          # Geo HTTP endpoints
├── geo_math.go          # Geo calculations
├── gw_http.go           # Gateway HTTP integration
├── api_announce.go      # Service announcement API
├── api_heartbeat.go     # Heartbeat handling
├── pubsub.go            # Pub/sub integration
├── stream.go            # P2P stream handling
├── helpers.go           # Utility functions
├── http.go              # HTTP service
├── http_auth.go         # HTTP auth endpoints
├── options.go           # Service options
├── package.go           # Package definition
├── common/
│   └── iface.go         # Interface definitions
├── dream/
│   └── init.go          # Dream integration
├── tests/
│   └── [integration tests]
└── [other files]
```

### Service Structure

```go
// tau/services/seer/type.go
type Service struct {
    ctx          context.Context
    node         peer.Node
    clientNode   peer.Node
    config       *tauConfig.Node
    shape        string
    devMode      bool
    stream       *streams.Service
    ds           ds.Datastore
    tns          *tns.Client
    dns          *DNSServer
    http         http.Service
    geo          *geoService
    oracle       *oracleService
    poe          *poe.Engine
    dnsResolver  *net.Resolver
    hostUrl      string
    positiveCache *Cache
    negativeCache *Cache
}

type DNSServer struct {
    udpConn *net.UDPConn
    tcpConn *net.TCPListener
    zones   map[string]*Zone
}

type Zone struct {
    Name    string
    Records map[string][]Record
}

type Record struct {
    Name    string
    Type    uint16
    TTL     uint32
    Data    []byte
}
```

---

## Service Initialization

```go
// tau/services/seer/service.go
func New(ctx context.Context, config *tauConfig.Node, opts ...Options) (*Service, error) {
    srv := &Service{
        config: config,
        shape:  config.Shape,
    }

    // Initialize POE (Policy Engine)
    poeFolder := os.DirFS(path.Join(config.Root, "config", "poe", "star"))
    if _, err := poeFolder.Open("dns.star"); err == nil {
        srv.poe, err = poe.New(poeFolder, "dns.star")
    }

    srv.dnsResolver = net.DefaultResolver
    srv.hostUrl = config.NetworkFqdn

    // Apply options
    for _, op := range opts {
        err = op(srv)
        if err != nil {
            return nil, err
        }
    }

    // Initialize P2P node
    if config.Node == nil {
        srv.node, err = tauConfig.NewLiteNode(ctx, config,
            path.Join(config.Root, servicesCommon.Seer))
    } else {
        srv.node = config.Node
    }

    srv.devMode = config.DevMode

    // Initialize TNS client
    srv.tns, err = tnsClient.New(ctx, clientNode)

    // Initialize database (Pebble)
    srv.ds, err = pebbleds.NewDatastore(
        path.Join(config.Root, "storage", srv.shape, "seer"),
        nil,
    )

    // Initialize services
    srv.geo = &geoService{srv}
    srv.oracle = &oracleService{srv}

    // Setup P2P stream
    srv.stream, err = streams.New(srv.node, servicesCommon.Seer,
        servicesCommon.SeerProtocol)
    srv.setupStreamRoutes()
    srv.stream.Start()

    // Subscribe to pubsub
    err = srv.subscribe()

    // Initialize Seer client for beacon
    sc, err := seerClient.New(ctx, clientNode, config.SensorsRegistry())
    err = servicesCommon.StartSeerBeacon(config, sc, seerIface.ServiceTypeSeer,
        servicesCommon.SeerBeaconOptionMeta(map[string]string{"others": "dns"}))

    // Start DNS server
    err = srv.newDnsServer(config.DevMode, config.Ports["dns"])
    srv.dns.Start(ctx)

    // Initialize HTTP
    if config.Http == nil {
        srv.http, err = auto.New(ctx, srv.node, config)
    } else {
        srv.http = config.Http
    }
    srv.setupHTTPRoutes()
    srv.http.Start()

    return srv, nil
}

func (srv *Service) Close() error {
    srv.stream.Stop()
    time.Sleep(100 * time.Millisecond)
    srv.tns.Close()
    srv.ds.Close()
    srv.dns.Stop()
    srv.positiveCache.Stop()
    srv.negativeCache.Stop()
    return nil
}
```

---

## DNS Server Architecture

### DNS Server Implementation

```go
// tau/services/seer/dns.go
type DNSServer struct {
    srv      *Service
    udpConn  *net.UDPConn
    tcpConn  *net.TCPListener
    zones    sync.Map  // map[string]*Zone
    running  atomic.Bool
}

func (srv *Service) newDnsServer(devMode bool, port int) error {
    udpAddr, err := net.ResolveUDPAddr("udp", fmt.Sprintf(":%d", port))
    if err != nil {
        return err
    }

    udpConn, err := net.ListenUDP("udp", udpAddr)
    if err != nil {
        return err
    }

    tcpAddr, err := net.ResolveTCPAddr("tcp", fmt.Sprintf(":%d", port))
    if err != nil {
        return err
    }

    tcpConn, err := net.ListenTCP("tcp", tcpAddr)
    if err != nil {
        return err
    }

    srv.dns = &DNSServer{
        srv:     srv,
        udpConn: udpConn,
        tcpConn: tcpConn,
    }

    return nil
}

func (d *DNSServer) Start(ctx context.Context) {
    go d.serveUDP(ctx)
    go d.serveTCP(ctx)
}

func (d *DNSServer) serveUDP(ctx context.Context) {
    buf := make([]byte, 4096)
    for {
        n, addr, err := d.udpConn.ReadFromUDP(buf)
        if err != nil {
            if ctx.Err() != nil {
                return
            }
            continue
        }

        msg, err := dns.ParseMessage(buf[:n])
        if err != nil {
            continue
        }

        resp := d.handleQuery(msg)
        d.udpConn.WriteToUDP(resp.Pack(), addr)
    }
}

func (d *DNSServer) serveTCP(ctx context.Context) {
    for {
        conn, err := d.tcpConn.AcceptTCP()
        if err != nil {
            if ctx.Err() != nil {
                return
            }
            continue
        }

        go d.handleTCPConnection(conn)
    }
}
```

### DNS Query Handling

```go
// tau/services/seer/dns.go
func (d *DNSServer) handleQuery(msg *dns.Message) *dns.Message {
    resp := &dns.Message{
        ID:       msg.ID,
        Response: true,
        Opcode:   msg.Opcode,
        RecursionDesired: msg.RecursionDesired,
    }

    for _, question := range msg.Questions {
        answer := d.resolveQuestion(question)
        if answer != nil {
            resp.Answers = append(resp.Answers, *answer)
        } else {
            // Try recursive resolution
            if msg.RecursionDesired {
                records, err := d.srv.dnsResolver.LookupIPAddr(
                    context.Background(), question.Name)
                if err == nil {
                    for _, record := range records {
                        resp.Answers = append(resp.Answers, dns.ResourceRecord{
                            Header: dns.RR_Header{
                                Name:   question.Name,
                                Rrtype: dns.TypeA,
                                Class:  dns.ClassINET,
                                Ttl:    300,
                            },
                            Body: &dns.A{A: record.IP},
                        })
                    }
                }
            }
        }
    }

    return resp
}

func (d *DNSServer) resolveQuestion(question dns.Question) *dns.ResourceRecord {
    // Check positive cache first
    if records, ok := d.srv.positiveCache.Get(question.Name); ok {
        for _, record := range records {
            if record.Type == question.Qtype {
                return record
            }
        }
    }

    // Check zones
    zone := d.getZoneForName(question.Name)
    if zone != nil {
        records := zone.GetRecords(question.Name, question.Qtype)
        if len(records) > 0 {
            d.srv.positiveCache.Set(question.Name, records)
            return records[0]
        }
    }

    // Check TNS for function domains
    if strings.HasSuffix(question.Name, ".tau.local") {
        return d.resolveTauDomain(question.Name)
    }

    return nil
}
```

### DNS Zone Management

```go
// tau/services/seer/dns.go
type Zone struct {
    Name    string
    Records map[string][]Record
    mutex   sync.RWMutex
}

func (z *Zone) AddRecord(record Record) {
    z.mutex.Lock()
    defer z.mutex.Unlock()

    if z.Records == nil {
        z.Records = make(map[string][]Record)
    }

    key := recordKey(record.Name, record.Type)
    z.Records[key] = append(z.Records[key], record)
}

func (z *Zone) GetRecords(name string, rtype uint16) []Record {
    z.mutex.RLock()
    defer z.mutex.RUnlock()

    key := recordKey(name, rtype)
    return z.Records[key]
}

func (d *DNSServer) loadZones() error {
    // Load zones from TNS
    zones, err := d.srv.tns.ListZones()
    if err != nil {
        return err
    }

    for _, zoneData := range zones {
        zone := &Zone{
            Name:    zoneData.Name,
            Records: make(map[string][]Record),
        }

        for _, record := range zoneData.Records {
            zone.AddRecord(Record{
                Name: record.Name,
                Type: record.Type,
                TTL:  record.TTL,
                Data: record.Data,
            })
        }

        d.zones.Store(zone.Name, zone)
    }

    return nil
}
```

---

## DNS Cache System

### Cache Implementation

```go
// tau/services/seer/dns.go
type Cache struct {
    data  sync.Map  // map[string][]*Record
    ttl   time.Duration
    stop  chan struct{}
}

func NewCache(ttl time.Duration) *Cache {
    c := &Cache{
        ttl:  ttl,
        stop: make(chan struct{}),
    }
    go c.cleanup()
    return c
}

func (c *Cache) Get(name string) ([]*Record, bool) {
    if data, ok := c.data.Load(name); ok {
        records := data.([]*Record)
        // Check TTL
        if len(records) > 0 && time.Now().Before(records[0].Expires) {
            return records, true
        }
        c.data.Delete(name)
    }
    return nil, false
}

func (c *Cache) Set(name string, records []*Record) {
    for _, r := range records {
        r.Expires = time.Now().Add(c.ttl)
    }
    c.data.Store(name, records)
}

func (c *Cache) cleanup() {
    ticker := time.NewTicker(time.Minute)
    for {
        select {
        case <-ticker.C:
            c.data.Range(func(key, value interface{}) bool {
                records := value.([]*Record)
                if len(records) > 0 && time.Now().After(records[0].Expires) {
                    c.data.Delete(key)
                }
                return true
            })
        case <-c.stop:
            return
        }
    }
}

func (c *Cache) Stop() {
    close(c.stop)
}
```

---

## Heartbeat System

### Heartbeat Handling

```go
// tau/services/seer/api_heartbeat.go
type Heartbeat struct {
    ServiceType string            `json:"service_type"`
    ServiceID   string            `json:"service_id"`
    NodeID      string            `json:"node_id"`
    CPU         float64           `json:"cpu"`
    Memory      uint64            `json:"memory"`
    Disk        uint64            `json:"disk"`
    Meta        map[string]string `json:"meta"`
    Timestamp   time.Time         `json:"timestamp"`
}

func (srv *Service) handleHeartbeat(stream network.Stream) {
    var hb Heartbeat
    if err := json.NewDecoder(stream).Decode(&hb); err != nil {
        stream.Reset()
        return
    }

    // Store heartbeat
    key := heartbeatKey(hb.ServiceType, hb.ServiceID)
    data, _ := json.Marshal(hb)
    srv.ds.Put(key, data)

    // Update last seen
    srv.ds.Put(lastSeenKey(hb.NodeID), []byte(time.Now().Format(time.RFC3339)))

    // Send acknowledgment
    json.NewEncoder(stream).Encode(map[string]string{"status": "ok"})
}

func (srv *Service) getHealthyServices() ([]ServiceInfo, error) {
    now := time.Now()
    threshold := 5 * time.Minute

    var healthy []ServiceInfo
    results, _ := srv.ds.List([]byte("heartbeat:"))

    for _, data := range results {
        var hb Heartbeat
        json.Unmarshal(data, &hb)

        if now.Sub(hb.Timestamp) < threshold {
            healthy = append(healthy, ServiceInfo{
                Type:      hb.ServiceType,
                ID:        hb.ServiceID,
                NodeID:    hb.NodeID,
                CPU:       hb.CPU,
                Memory:    hb.Memory,
                Disk:      hb.Disk,
                LastSeen:  hb.Timestamp,
            })
        }
    }

    return healthy, nil
}
```

### Service Announcement

```go
// tau/services/seer/api_announce.go
type ServiceAnnouncement struct {
    Type      string            `json:"type"`
    ID        string            `json:"id"`
    Address   string            `json:"address"`
    Port      int               `json:"port"`
    Meta      map[string]string `json:"meta"`
    Timestamp time.Time         `json:"timestamp"`
}

func (srv *Service) handleAnnounce(stream network.Stream) {
    var announcement ServiceAnnouncement
    json.NewDecoder(stream).Decode(&announcement)

    // Store in database
    key := announceKey(announcement.Type, announcement.ID)
    data, _ := json.Marshal(announcement)
    srv.ds.Put(key, data)

    // Broadcast to subscribers
    srv.publishAnnouncement(&announcement)

    json.NewEncoder(stream).Encode(map[string]string{"status": "registered"})
}

func (srv *Service) DiscoverServices(serviceType string) ([]ServiceAnnouncement, error) {
    prefix := []byte("announce:" + serviceType + ":")
    results, err := srv.ds.List(prefix)

    var services []ServiceAnnouncement
    for _, data := range results {
        var svc ServiceAnnouncement
        json.Unmarshal(data, &svc)
        services = append(services, svc)
    }

    return services, nil
}
```

---

## Geo-Location Service

### Geo Service Implementation

```go
// tau/services/seer/geo_api.go
type geoService struct {
    srv *Service
}

type GeoLocation struct {
    Country     string  `json:"country"`
    Region      string  `json:"region"`
    City        string  `json:"city"`
    Latitude    float64 `json:"latitude"`
    Longitude   float64 `json:"longitude"`
    Timezone    string  `json:"timezone"`
    ISP         string  `json:"isp"`
}

func (g *geoService) LookupIP(ip string) (*GeoLocation, error) {
    // Check cache first
    if loc, ok := g.srv.geoCache.Get(ip); ok {
        return loc, nil
    }

    // Query geo database (MaxMind or similar)
    loc, err := g.queryGeoDB(ip)
    if err != nil {
        return nil, err
    }

    g.srv.geoCache.Set(ip, loc)
    return loc, nil
}

func (g *geoService) GetNearestNodes(lat, lon float64, serviceType string) ([]string, error) {
    // Get all services of this type
    services, err := g.srv.DiscoverServices(serviceType)
    if err != nil {
        return nil, err
    }

    // Calculate distances
    type nodeDistance struct {
        nodeID   string
        distance float64
    }

    var distances []nodeDistance
    for _, svc := range services {
        svcLoc, err := g.LookupIP(svc.Address)
        if err != nil {
            continue
        }

        dist := haversine(lat, lon, svcLoc.Latitude, svcLoc.Longitude)
        distances = append(distances, nodeDistance{
            nodeID:   svc.ID,
            distance: dist,
        })
    }

    // Sort by distance
    sort.Slice(distances, func(i, j int) bool {
        return distances[i].distance < distances[j].distance
    })

    // Return nearest nodes
    var nearest []string
    for i := 0; i < len(distances) && i < 3; i++ {
        nearest = append(nearest, distances[i].nodeID)
    }

    return nearest, nil
}
```

### Haversine Distance

```go
// tau/services/seer/geo_math.go
const earthRadius = 6371 // km

func haversine(lat1, lon1, lat2, lon2 float64) float64 {
    // Convert to radians
    lat1 = lat1 * math.Pi / 180
    lat2 = lat2 * math.Pi / 180
    lon1 = lon1 * math.Pi / 180
    lon2 = lon2 * math.Pi / 180

    // Haversine formula
    dlat := lat2 - lat1
    dlon := lon2 - lon1

    a := math.Sin(dlat/2)*math.Sin(dlat/2) +
         math.Cos(lat1)*math.Cos(lat2)*math.Sin(dlon/2)*math.Sin(dlon/2)

    c := 2 * math.Atan2(math.Sqrt(a), math.Sqrt(1-a))

    return earthRadius * c
}
```

---

## Oracle Service

### External Data Resolution

```go
// tau/services/seer/dns.go
type oracleService struct {
    srv *Service
}

type OracleQuery struct {
    Type    string                 `json:"type"`
    Source  string                 `json:"source"`
    Query   string                 `json:"query"`
    Params  map[string]interface{} `json:"params"`
}

type OracleResponse struct {
    Data    interface{} `json:"data"`
    Error   string      `json:"error,omitempty"`
    Cached  bool        `json:"cached"`
}

func (o *oracleService) Query(ctx context.Context, q *OracleQuery) (*OracleResponse, error) {
    // Check cache
    cacheKey := oracleCacheKey(q)
    if cached, ok := o.srv.oracleCache.Get(cacheKey); ok {
        return cached, nil
    }

    var resp OracleResponse
    switch q.Type {
    case "http":
        resp = o.queryHTTP(q)
    case "dns":
        resp = o.queryDNS(q)
    case "custom":
        resp = o.queryCustom(q)
    }

    if resp.Error == "" {
        o.srv.oracleCache.Set(cacheKey, resp)
    }

    return &resp, nil
}

func (o *oracleService) queryHTTP(q *OracleQuery) OracleResponse {
    client := &http.Client{Timeout: 10 * time.Second}

    req, err := http.NewRequest("GET", q.Source, nil)
    if err != nil {
        return OracleResponse{Error: err.Error()}
    }

    resp, err := client.Do(req)
    if err != nil {
        return OracleResponse{Error: err.Error()}
    }
    defer resp.Body.Close()

    var data interface{}
    json.NewDecoder(resp.Body).Decode(&data)

    return OracleResponse{Data: data}
}
```

---

## P2P Protocol

### Stream Handlers

```go
// tau/services/seer/stream.go
func (srv *Service) setupStreamRoutes() {
    srv.stream.HandleFunc("heartbeat", srv.handleHeartbeat)
    srv.stream.HandleFunc("announce", srv.handleAnnounce)
    srv.stream.HandleFunc("discover", srv.handleDiscover)
    srv.stream.HandleFunc("dns.query", srv.handleDNSQuery)
    srv.stream.HandleFunc("geo.lookup", srv.handleGeoLookup)
    srv.stream.HandleFunc("oracle.query", srv.handleOracleQuery)
}

func (srv *Service) handleDiscover(stream network.Stream) {
    var req DiscoverRequest
    json.NewDecoder(stream).Decode(&req)

    services, err := srv.DiscoverServices(req.ServiceType)
    if err != nil {
        json.NewEncoder(stream).Encode(ErrorResponse{Error: err.Error()})
        return
    }

    json.NewEncoder(stream).Encode(DiscoverResponse{Services: services})
}
```

---

## HTTP API

### API Endpoints

```go
// tau/services/seer/geo_http.go
func (srv *Service) setupHTTPRoutes() {
    // Geo-location
    srv.http.HandleFunc("/api/seer/geo/ip/{ip}", srv.handleGeoIP)
    srv.http.HandleFunc("/api/seer/geo/nearest", srv.handleNearestNodes)

    // DNS
    srv.http.HandleFunc("/api/seer/dns/resolve", srv.handleDNSResolve)
    srv.http.HandleFunc("/api/seer/dns/zone/{zone}", srv.handleGetZone)

    // Discovery
    srv.http.HandleFunc("/api/seer/services", srv.handleListServices)
    srv.http.HandleFunc("/api/seer/services/{type}", srv.handleGetServices)

    // Health
    srv.http.HandleFunc("/api/seer/health", srv.handleHealth)
    srv.http.HandleFunc("/api/seer/heartbeat", srv.handleHeartbeatHTTP)
}

func (srv *Service) handleGeoIP(w http.ResponseWriter, r *http.Request) {
    ip := chi.URLParam(r, "ip")
    loc, err := srv.geo.LookupIP(ip)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    json.NewEncoder(w).Encode(loc)
}

func (srv *Service) handleNearestNodes(w http.ResponseWriter, r *http.Request) {
    lat := r.URL.Query().Get("lat")
    lon := r.URL.Query().Get("lon")
    serviceType := r.URL.Query().Get("type")

    nodes, err := srv.geo.GetNearestNodes(lat, lon, serviceType)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    json.NewEncoder(w).Encode(nodes)
}
```

---

## Integration with Other Services

### Seer Beacon (Service Registration)

```go
// tau/services/common/seer_beacon.go
type BeaconConfig struct {
    ServiceType string
    Meta        map[string]string
    Interval    time.Duration
}

func StartSeerBeacon(config *Node, client *seerClient.Client, serviceType string, opts ...BeaconOption) error {
    cfg := &BeaconConfig{
        ServiceType: serviceType,
        Meta:        make(map[string]string),
        Interval:    30 * time.Second,
    }

    for _, opt := range opts {
        opt(cfg)
    }

    go func() {
        ticker := time.NewTicker(cfg.Interval)
        for range ticker.C {
            usage := collectUsage()
            usage.Meta = cfg.Meta
            client.Announce(usage)
        }
    }()

    return nil
}
```

### TNS Integration

```go
// tau/services/seer/dns.go
func (d *DNSServer) resolveTauDomain(name string) *dns.ResourceRecord {
    // Query TNS for domain mapping
    mapping, err := d.srv.tns.ResolveDomain(name)
    if err != nil {
        return nil
    }

    // Return A record pointing to function gateway
    return &dns.ResourceRecord{
        Header: dns.RR_Header{
            Name:   name,
            Rrtype: dns.TypeA,
            Class:  dns.ClassINET,
            Ttl:    300,
        },
        Body: &dns.A{A: net.ParseIP(mapping.GatewayIP)},
    }
}
```

---

## Testing

### DNS Tests

```go
// tau/services/seer/tests/dns_test.go
func TestDNSResolution(t *testing.T) {
    config := createTestConfig()
    srv, err := New(ctx, config)

    // Add test zone
    srv.dns.AddZone(&Zone{
        Name: "example.tau.local",
        Records: map[string][]Record{
            "api.example.tau.local": {{
                Type: dns.TypeA,
                Data: net.ParseIP("127.0.0.1"),
            }},
        },
    })

    // Query DNS
    msg := &dns.Message{
        Questions: []dns.Question{{
            Name:  "api.example.tau.local",
            Qtype: dns.TypeA,
        }},
    }

    resp := srv.dns.handleQuery(msg)

    if len(resp.Answers) == 0 {
        t.Error("Expected DNS answer")
    }
}
```

### Heartbeat Tests

```go
// tau/services/seer/tests/hearbeat_test.go
func TestHeartbeatProcessing(t *testing.T) {
    srv := createTestSeer()

    hb := &Heartbeat{
        ServiceType: "monkey",
        ServiceID:   "monkey-1",
        NodeID:      "node-1",
        CPU:         45.5,
        Memory:      1024 * 1024 * 512,
        Timestamp:   time.Now(),
    }

    // Send heartbeat
    err := srv.processHeartbeat(hb)
    if err != nil {
        t.Fatal(err)
    }

    // Verify stored
    healthy, _ := srv.getHealthyServices()
    if len(healthy) == 0 {
        t.Error("Expected healthy service")
    }
}
```

---

## Configuration

### Service Configuration

```yaml
# config/seer.yaml
seer:
  dns:
    enabled: true
    port: 53
    cache:
      positive_ttl: 5m
      negative_ttl: 1m
  geo:
    enabled: true
    database: maxmind
  oracle:
    enabled: true
    cache_ttl: 10m
  heartbeat:
    timeout: 5m
    check_interval: 1m
  database:
    type: pebble
    path: storage/seer
```

---

## Troubleshooting

### Common Issues

1. **DNS Resolution Failures**
   - Check zone configuration
   - Verify cache isn't stale
   - Check TNS connectivity

2. **Missing Heartbeats**
   - Verify service beacon is running
   - Check P2P connectivity
   - Review heartbeat timeout settings

3. **Geo-Location Errors**
   - Verify geo database is loaded
   - Check IP format
   - Review cache settings

---

## Related Documents

- `../exploration.md` - Main exploration
- `tns.md` - Name service
- `auth.md` - Authentication service (domain validation)
- `../production-grade.md` - Production considerations
