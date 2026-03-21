# Patrick Service - Build Scheduler Deep Dive

## Overview

**Patrick** is Taubyte's build scheduler and job queue service. It manages build jobs, tracks their status, and coordinates with Monkey for function compilation and deployment.

---

## Service Architecture

### Core Components

```
tau/services/patrick/
├── service.go           # Main service implementation
├── type.go              # Service type definitions
├── job.go               # Job handling
├── jobs.go              # Job queue management
├── api.go               # HTTP API endpoints
├── api_stats.go         # Statistics endpoints
├── pubsub.go            # Pub/sub integration
├── stream.go            # P2P stream handling
├── helpers.go           # Utility functions
├── database.go          # Database operations
├── common/
│   └── iface.go         # Interface definitions
├── dream/
│   └── init.go          # Dream integration
└── tests/
    └── [integration tests]
```

### Service Structure

```go
// tau/services/patrick/type.go
type Service struct {
    ctx         context.Context
    node        peer.Node
    clientNode  peer.Node
    config      *tauConfig.Node
    dev         bool
    stream      *streams.Service
    db          kvdb.KVDB
    jobs        *JobQueue
    hoarder     *hoarder.Client
    tns         *tns.Client
    stats       *Stats
}

type Job struct {
    ID          string            `json:"id"`
    Type        string            `json:"type"`  // build, deploy, delete
    Status      JobStatus         `json:"status"`
    FunctionID  string            `json:"function_id"`
    Language    string            `json:"language"`
    SourceCID   string            `json:"source_cid"`
    WasmCID     string            `json:"wasm_cid,omitempty"`
    CreatedAt   time.Time         `json:"created_at"`
    UpdatedAt   time.Time         `json:"updated_at"`
    Metadata    map[string]string `json:"metadata,omitempty"`
}

type JobStatus struct {
    State       string    `json:"state"`  // pending, running, completed, failed
    Error       string    `json:"error,omitempty"`
    StartedAt   time.Time `json:"started_at,omitempty"`
    CompletedAt time.Time `json:"completed_at,omitempty"`
}
```

---

## Service Initialization

```go
// tau/services/patrick/service.go
func New(ctx context.Context, config *tauConfig.Node) (*Service, error) {
    srv := &Service{
        ctx:    ctx,
        dev:    config.DevMode,
        config: config,
    }

    // Initialize P2P node
    if config.Node == nil {
        srv.node, err = tauConfig.NewLiteNode(ctx, config,
            path.Join(config.Root, protocolCommon.Patrick))
    } else {
        srv.node = config.Node
    }

    // Initialize database
    srv.db, err = pebbleds.NewDatastore(
        path.Join(config.Root, "storage", config.Shape, "patrick"),
        nil,
    )

    // Setup P2P stream
    srv.stream, err = streams.New(srv.node, protocolCommon.Patrick,
        protocolCommon.PatrickProtocol)
    srv.setupStreamRoutes()
    srv.stream.Start()

    // Initialize job queue
    srv.jobs = NewJobQueue(srv.db)
    srv.jobs.Start()

    // Initialize clients
    srv.hoarder, err = hoarder.New(ctx, srv.clientNode)
    srv.tns, err = tns.New(ctx, srv.clientNode)

    // Start stats collection
    srv.stats = NewStats()

    return srv, nil
}

func (srv *Service) Close() error {
    srv.stream.Stop()
    srv.jobs.Stop()
    srv.db.Close()
    return nil
}
```

---

## Job Queue Architecture

### Queue Structure

```
┌─────────────────────────────────────────────────────────────┐
│                      JOB QUEUE                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   PENDING                            │   │
│  │  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐            │   │
│  │  │ Job 1│  │ Job 2│  │ Job 3│  │ Job N│            │   │
│  │  └──────┘  └──────┘  └──────┘  └──────┘            │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   RUNNING                            │   │
│  │  ┌──────┐  ┌──────┐                                 │   │
│  │  │ Job A│  │ Job B│                                 │   │
│  │  └──────┘  └──────┘                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 COMPLETED                            │   │
│  │  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐            │   │
│  │  │Job X │  │Job Y │  │Job Z │  │ ...  │            │   │
│  │  └──────┘  └──────┘  └──────┘  └──────┘            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Job Queue Implementation

```go
// tau/services/patrick/jobs.go
type JobQueue struct {
    db           kvdb.KVDB
    pending      chan *Job
    workers      int
    maxRetries   int
    retryDelay   time.Duration
}

func NewJobQueue(db kvdb.KVDB) *JobQueue {
    return &JobQueue{
        db:         db,
        pending:    make(chan *Job, 1000),
        workers:    10,
        maxRetries: 3,
        retryDelay: 5 * time.Second,
    }
}

func (q *JobQueue) Start() {
    for i := 0; i < q.workers; i++ {
        go q.worker()
    }
}

func (q *JobQueue) worker() {
    for job := range q.pending {
        q.processJob(job)
    }
}

func (q *JobQueue) processJob(job *Job) {
    // Update status to running
    job.Status.State = "running"
    job.Status.StartedAt = time.Now()
    q.saveJob(job)

    // Execute job
    err := q.executeJob(job)

    if err != nil {
        if job.Retries < q.maxRetries {
            job.Retries++
            job.Status.Error = err.Error()
            q.saveJob(job)
            time.Sleep(q.retryDelay)
            q.pending <- job
            return
        }
        job.Status.State = "failed"
        job.Status.Error = err.Error()
    } else {
        job.Status.State = "completed"
        job.Status.CompletedAt = time.Now()
    }

    q.saveJob(job)
    q.publishStatus(job)
}
```

---

## Job Types

### Build Job

```go
// tau/services/patrick/job.go
type BuildJob struct {
    Job
    FunctionID string       `json:"function_id"`
    Language   string       `json:"language"`
    SourceCID  string       `json:"source_cid"`
    Config     BuildConfig  `json:"config"`
}

type BuildConfig struct {
    MemoryLimit   string            `json:"memory_limit"`
    Timeout       time.Duration     `json:"timeout"`
    Environment   map[string]string `json:"environment"`
    BuildFlags    []string          `json:"build_flags"`
}

func (j *BuildJob) Execute() error {
    // Fetch source from Hoarder
    source, err := hoarder.Get(j.SourceCID)
    if err != nil {
        return fmt.Errorf("fetching source: %w", err)
    }

    // Send build request to Monkey
    buildReq := &monkey.BuildRequest{
        Language: j.Language,
        Source:   source,
        Config:   j.Config,
    }

    // Wait for build completion
    wasmCID, err := monkeyClient.Build(buildReq)
    if err != nil {
        return fmt.Errorf("building: %w", err)
    }

    j.WasmCID = wasmCID
    return nil
}
```

### Deploy Job

```go
type DeployJob struct {
    Job
    FunctionID string       `json:"function_id"`
    WasmCID    string       `json:"wasm_cid"`
    Routes     []Route      `json:"routes"`
    Config     DeployConfig `json:"config"`
}

type Route struct {
    Path    string   `json:"path"`
    Methods []string `json:"methods"`
}

type DeployConfig struct {
    Replicas   int     `json:"replicas"`
    MemoryLimit string `json:"memory_limit"`
    Timeout     time.Duration `json:"timeout"`
}

func (j *DeployJob) Execute() error {
    // Update TNS with new deployment
    err := tnsClient.UpdateFunction(j.FunctionID, j.WasmCID)
    if err != nil {
        return fmt.Errorf("updating TNS: %w", err)
    }

    // Notify Monkey to preload function
    err = monkeyClient.Preload(j.FunctionID, j.WasmCID)
    if err != nil {
        return fmt.Errorf("preloading: %w", err)
    }

    // Update routes in Gateway
    err = gatewayClient.UpdateRoutes(j.FunctionID, j.Routes)

    return nil
}
```

### Delete Job

```go
type DeleteJob struct {
    Job
    FunctionID string `json:"function_id"`
    Cleanup    bool   `json:"cleanup"`
}

func (j *DeleteJob) Execute() error {
    // Remove from TNS
    err := tnsClient.DeleteFunction(j.FunctionID)
    if err != nil {
        return fmt.Errorf("deleting from TNS: %w", err)
    }

    // Stop Monkey containers
    err = monkeyClient.Stop(j.FunctionID)

    // Cleanup WASM from Hoarder (optional)
    if j.Cleanup {
        err = hoarder.Delete(j.WasmCID)
    }

    return nil
}
```

---

## Pub/Sub Integration

### Job Notifications

```go
// tau/services/patrick/pubsub.go
const PubSubIdent = "patrick.jobs"

func (srv *Service) publishJobEvent(job *Job) {
    event := JobEvent{
        JobID:     job.ID,
        Type:      job.Type,
        Status:    job.Status,
        Timestamp: time.Now(),
    }

    data, _ := json.Marshal(event)
    srv.node.PubSubPublish(PubSubIdent, data)
}

func (srv *Service) publishStatus(job *Job) {
    status := JobStatusUpdate{
        JobID:     job.ID,
        Status:    job.Status,
        WasmCID:   job.WasmCID,
        Timestamp: time.Now(),
    }

    data, _ := json.Marshal(status)
    srv.node.PubSubPublish(PubSubIdent, data)
}
```

### Subscription Pattern

```go
// Subscribers (Monkey) listen for job events
func subscribeToPatrick(ctx context.Context, node peer.Node) error {
    return node.PubSubSubscribe(PubSubIdent, func(msg *pubsub.Message) {
        var event JobEvent
        json.Unmarshal(msg.Data, &event)

        switch event.Type {
        case "build":
            handleBuildEvent(&event)
        case "deploy":
            handleDeployEvent(&event)
        }
    })
}
```

---

## HTTP API

### API Endpoints

```go
// tau/services/patrick/api.go
func (srv *Service) setupHTTPRoutes() {
    // Job management
    srv.http.HandleFunc("/api/patrick/jobs", srv.handleCreateJob)
    srv.http.HandleFunc("/api/patrick/jobs/{id}", srv.handleGetJob)
    srv.http.HandleFunc("/api/patrick/jobs/{id}/cancel", srv.handleCancelJob)

    // List operations
    srv.http.HandleFunc("/api/patrick/jobs/list", srv.handleListJobs)
    srv.http.HandleFunc("/api/patrick/jobs/pending", srv.handleListPending)
    srv.http.HandleFunc("/api/patrick/jobs/running", srv.handleListRunning)
    srv.http.HandleFunc("/api/patrick/jobs/completed", srv.handleListCompleted)

    // Statistics
    srv.http.HandleFunc("/api/patrick/stats", srv.handleStats)
    srv.http.HandleFunc("/api/patrick/stats/history", srv.handleStatsHistory)
}
```

### Job Creation

```go
func (srv *Service) handleCreateJob(w http.ResponseWriter, r *http.Request) {
    var req CreateJobRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }

    job := &Job{
        ID:         generateJobID(),
        Type:       req.Type,
        Status:     JobStatus{State: "pending"},
        FunctionID: req.FunctionID,
        CreatedAt:  time.Now(),
        UpdatedAt:  time.Now(),
    }

    // Save to database
    if err := srv.db.Put(jobKey(job.ID), job.Marshal()); err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    // Add to queue
    srv.jobs.pending <- job

    // Publish event
    srv.publishJobEvent(job)

    json.NewEncoder(w).Encode(job)
}
```

### Job Status

```go
func (srv *Service) handleGetJob(w http.ResponseWriter, r *http.Request) {
    jobID := chi.URLParam(r, "id")

    data, err := srv.db.Get(jobKey(jobID))
    if err != nil {
        http.Error(w, "Job not found", http.StatusNotFound)
        return
    }

    var job Job
    job.Unmarshal(data)

    json.NewEncoder(w).Encode(job)
}
```

---

## Database Schema

### Job Storage

```go
// tau/services/patrick/database.go
const (
    jobPrefix      = "job:"
    statusPrefix   = "status:"
    historyPrefix  = "history:"
)

func jobKey(jobID string) []byte {
    return []byte(jobPrefix + jobID)
}

func statusKey(jobID string) []byte {
    return []byte(statusPrefix + jobID)
}

func (srv *Service) saveJob(job *Job) error {
    job.UpdatedAt = time.Now()
    return srv.db.Put(jobKey(job.ID), job.Marshal())
}

func (srv *Service) getJob(jobID string) (*Job, error) {
    data, err := srv.db.Get(jobKey(jobID))
    if err != nil {
        return nil, err
    }

    var job Job
    job.Unmarshal(data)
    return &job, nil
}

func (srv *Service) listJobs(status string) ([]*Job, error) {
    prefix := []byte(jobPrefix)
    if status != "" {
        prefix = []byte(statusPrefix + status + ":")
    }

    results, err := srv.db.List(prefix)
    if err != nil {
        return nil, err
    }

    var jobs []*Job
    for _, data := range results {
        var job Job
        job.Unmarshal(data)
        jobs = append(jobs, &job)
    }

    return jobs, nil
}
```

---

## Statistics

### Stats Collection

```go
// tau/services/patrick/api_stats.go
type Stats struct {
    TotalJobs     int64         `json:"total_jobs"`
    PendingJobs   int64         `json:"pending_jobs"`
    RunningJobs   int64         `json:"running_jobs"`
    CompletedJobs int64         `json:"completed_jobs"`
    FailedJobs    int64         `json:"failed_jobs"`
    AvgBuildTime  time.Duration `json:"avg_build_time"`
    JobsByLanguage map[string]int64 `json:"jobs_by_language"`
}

func (srv *Service) updateStats(job *Job) {
    srv.stats.TotalJobs++

    switch job.Status.State {
    case "pending":
        srv.stats.PendingJobs++
    case "running":
        srv.stats.RunningJobs++
    case "completed":
        srv.stats.CompletedJobs++
        buildTime := job.Status.CompletedAt.Sub(job.Status.StartedAt)
        srv.stats.updateAvgBuildTime(buildTime)
    case "failed":
        srv.stats.FailedJobs++
    }

    srv.stats.JobsByLanguage[job.Language]++
}
```

### Stats History

```go
type StatsHistory struct {
    Timestamp time.Time `json:"timestamp"`
    Stats     Stats     `json:"stats"`
}

func (srv *Service) recordStats() {
    ticker := time.NewTicker(time.Minute)
    for range ticker.C {
        history := StatsHistory{
            Timestamp: time.Now(),
            Stats:     *srv.stats,
        }
        srv.saveStatsHistory(history)
    }
}
```

---

## P2P Protocol

### Stream Handlers

```go
// tau/services/patrick/stream.go
func (srv *Service) setupStreamRoutes() {
    srv.stream.HandleFunc("job.create", srv.handleCreateJob)
    srv.stream.HandleFunc("job.get", srv.handleGetJob)
    srv.stream.HandleFunc("job.cancel", srv.handleCancelJob)
    srv.stream.HandleFunc("job.list", srv.handleListJobs)
    srv.stream.HandleFunc("stats.get", srv.handleStats)
}

func (srv *Service) handleCreateJob(stream network.Stream) {
    var req CreateJobRequest
    json.NewDecoder(stream).Decode(&req)

    job := &Job{
        ID:         generateJobID(),
        Type:       req.Type,
        Status:     JobStatus{State: "pending"},
        FunctionID: req.FunctionID,
        CreatedAt:  time.Now(),
    }

    srv.saveJob(job)
    srv.jobs.pending <- job
    srv.publishJobEvent(job)

    json.NewEncoder(stream).Encode(job)
}
```

---

## Client Integration

### Patrick Client (P2P)

```go
// tau/clients/p2p/patrick/client.go
type Client struct {
    node     peer.Node
    protocol string
}

func New(ctx context.Context, node peer.Node) (*Client, error) {
    return &Client{
        node:     node,
        protocol: PatrickProtocol,
    }, nil
}

func (c *Client) CreateJob(req *CreateJobRequest) (*Job, error) {
    stream, err := c.node.NewStream(c.protocol)
    if err != nil {
        return nil, err
    }
    defer stream.Close()

    json.NewEncoder(stream).Encode(req)

    var job Job
    json.NewDecoder(stream).Decode(&job)
    return &job, nil
}

func (c *Client) GetJob(jobID string) (*Job, error) {
    stream, err := c.node.NewStream(c.protocol)
    if err != nil {
        return nil, err
    }
    defer stream.Close()

    json.NewEncoder(stream).Encode(GetJobRequest{ID: jobID})

    var job Job
    json.NewDecoder(stream).Decode(&job)
    return &job, nil
}
```

### HTTP Client

```go
// tau/clients/http/patrick/job.go
type Client struct {
    baseURL string
    client  *http.Client
}

func (c *Client) CreateJob(req *CreateJobRequest) (*Job, error) {
    resp, err := c.client.Post(c.baseURL+"/api/patrick/jobs", "application/json",
        json.Marshal(req))
    if err != nil {
        return nil, err
    }

    var job Job
    json.NewDecoder(resp.Body).Decode(&job)
    return &job, nil
}

func (c *Client) GetJobStatus(jobID string) (*Job, error) {
    resp, err := c.client.Get(c.baseURL + "/api/patrick/jobs/" + jobID)
    if err != nil {
        return nil, err
    }

    var job Job
    json.NewDecoder(resp.Body).Decode(&job)
    return &job, nil
}
```

---

## Testing

### Unit Tests

```go
// tau/services/patrick/service_test.go
func TestJobQueue(t *testing.T) {
    db := newTestDB(t)
    queue := NewJobQueue(db)

    job := &Job{
        ID:   "test-job-1",
        Type: "build",
    }

    queue.pending <- job

    // Wait for processing
    time.Sleep(100 * time.Millisecond)

    // Verify job was processed
    savedJob, err := queue.getJob("test-job-1")
    if err != nil {
        t.Fatal(err)
    }

    if savedJob.Status.State != "completed" {
        t.Errorf("Expected completed, got %s", savedJob.Status.State)
    }
}
```

### Integration Tests

```go
// tau/clients/p2p/patrick/tests/p2p_test.go
func TestPatrickP2P(t *testing.T) {
    // Start Patrick service
    config := createTestConfig()
    patrick, err := New(ctx, config)

    // Create client
    client, err := p2p.NewClient(ctx, node)

    // Create job via P2P
    job, err := client.CreateJob(&CreateJobRequest{
        Type:       "build",
        FunctionID: "test-func",
        Language:   "go",
        SourceCID:  "test-source",
    })

    // Verify job created
    if job.Status.State != "pending" {
        t.Error("Job should be pending")
    }
}
```

---

## Configuration

### Service Configuration

```yaml
# config/patrick.yaml
patrick:
  workers: 10
  max_retries: 3
  retry_delay: 5s
  queue_size: 1000
  database:
    type: pebble
    path: storage/patrick
  stats:
    retention: 24h
    interval: 1m
```

### Job Configuration

```yaml
# .tau/jobs/build.yaml
build:
  timeout: 30m
  memory_limit: 512MB
  languages:
    go:
      enabled: true
      version: "1.21"
    rust:
      enabled: true
      version: "1.75"
    zig:
      enabled: true
      version: "0.11"
```

---

## Troubleshooting

### Common Issues

1. **Job Stuck in Pending**
   - Check worker count
   - Verify queue isn't full
   - Check database connectivity

2. **Build Failures**
   - Verify source CID exists in Hoarder
   - Check Monkey service availability
   - Review build logs

3. **Database Errors**
   - Check Pebble datastore path
   - Verify disk space
   - Check for corruption

---

## Related Documents

- `../exploration.md` - Main exploration
- `monkey.md` - Function execution service
- `hoarder.md` - Storage service
- `tns.md` - Name service
