---
source: /home/darkvoid/Boxxed/@formulas/src.pheonixLiveView/phoenix
repository: github.com:phoenixframework/phoenix
explored_at: 2026-04-05
focus: Production deployment, monitoring, scaling, cluster configuration, security
---

# Production-Grade Phoenix LiveView Applications

## Overview

This document covers production deployment patterns for Phoenix LiveView applications including horizontal scaling, monitoring with Prometheus, cluster configuration, security hardening, and performance optimization.

## Architecture

```mermaid
flowchart TB
    subgraph Edge
        LB[Load Balancer]
        CDN[CDN]
    end
    
    subgraph Cluster["Phoenix Cluster (N nodes)"]
        Node1[Node 1<br/>Phoenix Endpoint]
        Node2[Node 2<br/>Phoenix Endpoint]
        Node3[Node 3<br/>Phoenix Endpoint]
    end
    
    subgraph Services
        Redis[Redis<br/>PubSub + Cache]
        DB[PostgreSQL<br/>Primary + Replicas]
        Prometheus[Prometheus]
        Grafana[Grafana]
    end
    
    CDN --> LB
    LB --> Node1
    LB --> Node2
    LB --> Node3
    
    Node1 <--> Redis
    Node2 <--> Redis
    Node3 <--> Redis
    
    Node1 <--> DB
    Node2 <--> DB
    Node3 <--> DB
    
    Node1 -.metrics.-> Prometheus
    Node2 -.metrics.-> Prometheus
    Node3 -.metrics.-> Prometheus
    
    Prometheus --> Grafana
    
    subgraph K8s
        Node1
        Node2
        Node3
    end
```

## Cluster Configuration

### Node Discovery

```elixir
# config/runtime.exs

import Config

# Configure clustering
if System.get_env("CLUSTER_ENABLED") == "true" do
  config :libcluster,
    topologies: [
      k8s: [
        strategy: Cluster.Strategy.Kubernetes.DNS,
        config: [
          service: "phoenix-app",
          application_name: "phoenix_app",
          node_basename: "phoenix_app"
        ]
      ],
      # Alternative: Erlang Distribution
      erlang_dist: [
        strategy: Cluster.Strategy.ErlangDistribution,
        config: [
          hosts: [:"node1@192.168.1.10", :"node2@192.168.1.11"]
        ]
      ]
    ]
end

# Configure distributed Erlang cookie
cookie = System.get_env("RELEASE_COOKIE", "default-cookie")
config :phoenix_app, :cookie, cookie

# Configure node name
node_name = System.get_env("POD_NAME") || "phoenix_app"
node_host = System.get_env("POD_IP") || "127.0.0.1"
config :phoenix_app, :node_name, "#{node_name}@#{node_host}"
```

### Endpoint Configuration

```elixir
# config/prod.exs

import Config

config :phoenix_app, PhoenixAppWeb.Endpoint,
  server: true,
  https: [
    port: System.get_env("PORT") || "443",
    cipher_suite: :strong,
    certfile: System.get_env("CERT_PATH"),
    keyfile: System.get_env("KEY_PATH")
  ],
  http: [
    port: System.get_env("HTTP_PORT") || "80",
    protocol_options: [
      max_header_value_length: 64_000,
      max_keepalive: 65_000
    ]
  ],
  url: [host: "app.example.com", port: 443],
  static_url: [host: "static.example.com"],
  check_origin: ["https://example.com", "https://app.example.com"],
  code_reloader: false,
  gzip: true,
  cache_static_manifest: "priv/static/cache_manifest.json",
  input_gzip: true,
  debug_errors: false,
  render_errors: [
    formats: [html: PhoenixAppWeb.ErrorHTML, json: PhoenixAppWeb.ErrorJSON],
    layout: false
  ],
  live_view: [
    signing_salt: System.get_env("LIVE_VIEW_SALT"),
    transport_options: [
      socket_opts: [
        :inet6,
        keepalive: true,
        keepalive_interval: 30_000,
        keepalive_count: 5
      ]
    ]
  ],
  pubsub_server: PhoenixApp.PubSub

# Configure cluster healing
config :phoenix_app, cluster_healing_delay: :timer.seconds(30)
```

### Docker Configuration

```dockerfile
# Dockerfile

# Build stage
FROM hexpm/elixir:1.15.7-erlang-26.1.2 AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

ENV MIX_ENV="prod"
ENV HEX_HTTP_TIMEOUT=120

RUN mix local.hex --force && \
    mix local.rebar --force

COPY mix.exs mix.lock ./
RUN mix deps.get --only $MIX_ENV

COPY config config
COPY lib lib
COPY priv priv

RUN mix compile

# Build assets
RUN npm install --prefix assets && \
    npm run deploy --prefix assets

# Create release
RUN mix phx.gen.release

# Runtime stage
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libstdc++1 \
    openssl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/_build/prod/rel/phoenix_app ./

ENV MIX_ENV="prod"
ENV PORT="8080"

EXPOSE 8080

ENTRYPOINT ["/app/bin/server"]
CMD ["start"]
```

```yaml
# docker-compose.yml

version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://postgres:postgres@db:5432/phoenix_app_prod
      - SECRET_KEY_BASE=$(SECRET_KEY_BASE)
      - LIVE_VIEW_SALT=$(LIVE_VIEW_SALT)
      - RELEASE_COOKIE=$(RELEASE_COOKIE)
      - CLUSTER_ENABLED=true
      - POD_NAME=${HOSTNAME}
      - POD_IP=${HOST_IP}
    depends_on:
      - db
      - redis
    networks:
      - app-network
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M

  db:
    image: postgres:15
    volumes:
      - postgres-data:/var/lib/postgresql/data
    environment:
      - POSTGRES_DB=phoenix_app_prod
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=postgres
    networks:
      - app-network

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    networks:
      - app-network

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    ports:
      - "9090:9090"
    networks:
      - app-network

  grafana:
    image: grafana/grafana:latest
    volumes:
      - grafana-data:/var/lib/grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - app-network

volumes:
  postgres-data:
  redis-data:
  prometheus-data:
  grafana-data:

networks:
  app-network:
    driver: bridge
```

### Kubernetes Deployment

```yaml
# k8s/deployment.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: phoenix-app
  labels:
    app: phoenix-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: phoenix-app
  template:
    metadata:
      labels:
        app: phoenix-app
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: phoenix-app
      containers:
        - name: phoenix-app
          image: phoenix-app:latest
          imagePullPolicy: Always
          ports:
            - name: http
              containerPort: 8080
              protocol: TCP
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: phoenix-app-secrets
                  key: database-url
            - name: SECRET_KEY_BASE
              valueFrom:
                secretKeyRef:
                  name: phoenix-app-secrets
                  key: secret-key-base
            - name: LIVE_VIEW_SALT
              valueFrom:
                secretKeyRef:
                  name: phoenix-app-secrets
                  key: live-view-salt
            - name: RELEASE_COOKIE
              valueFrom:
                secretKeyRef:
                  name: phoenix-app-secrets
                  key: release-cookie
            - name: CLUSTER_ENABLED
              value: "true"
            - name: POD_NAME
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
            - name: POD_IP
              valueFrom:
                fieldRef:
                  fieldPath: status.podIP
            - name: HOSTNAME
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
          readinessProbe:
            httpGet:
              path: /health/readiness
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
            timeoutSeconds: 5
            failureThreshold: 3
          livenessProbe:
            httpGet:
              path: /health/liveness
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 20
            timeoutSeconds: 5
            failureThreshold: 3
          resources:
            requests:
              memory: "512Mi"
              cpu: "250m"
            limits:
              memory: "2Gi"
              cpu: "2000m"
          volumeMounts:
            - name: tmp
              mountPath: /app/tmp
            - name: uploads
              mountPath: /app/priv/static/uploads
      volumes:
        - name: tmp
          emptyDir: {}
        - name: uploads
          persistentVolumeClaim:
            claimName: phoenix-app-uploads-pvc
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: phoenix-app
                topologyKey: kubernetes.io/hostname
---
apiVersion: v1
kind: Service
metadata:
  name: phoenix-app
  labels:
    app: phoenix-app
spec:
  type: ClusterIP
  ports:
    - port: 8080
      targetPort: http
      protocol: TCP
      name: http
  selector:
    app: phoenix-app
---
apiVersion: v1
kind: Service
metadata:
  name: phoenix-app-headless
  labels:
    app: phoenix-app
spec:
  clusterIP: None
  ports:
    - port: 8080
      targetPort: http
      protocol: TCP
      name: http
  selector:
    app: phoenix-app
```

## Monitoring

### Prometheus Integration

```elixir
# lib/phoenix_app/telemetry.ex

defmodule PhoenixApp.Telemetry do
  use Supervisor

  import Telemetry.Metrics

  def start_link(arg) do
    Supervisor.start_link(__MODULE__, arg, name: __MODULE__)
  end

  @impl true
  def init(_arg) do
    children = [
      # Telemetry poller for VM stats
      {:telemetry_poller, measurements: periodic_measurements(), period: 10_000},
      
      # Prometheus exporter
      {TelemetryMetricsPrometheus.Core, metrics: metrics()},
      
      # Add reporters
      TelemetryMetricsPrometheusCore,
      TelemetryMetricsStatsD
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end

  def metrics do
    [
      # VM metrics
      summary("vm.memory.total", unit: {:byte, :megabyte}),
      summary("vm.total_run_queue_lengths.total", tags: [:system]),
      summary("vm.total_run_queue_lengths.cpu", tags: [:system]),
      summary("vm.total_run_queue_lengths.io", tags: [:system]),

      # Phoenix LiveView metrics
      summary("phoenix.live_view.mount.stop.duration",
        unit: {:native, :millisecond},
        tags: [:view]
      ),
      summary("phoenix.live_view.render.stop.duration",
        unit: {:native, :millisecond},
        tags: [:view]
      ),
      summary("phoenix.live_view.handle_event.stop.duration",
        unit: {:native, :millisecond},
        tags: [:event]
      ),
      summary("phoenix.live_view.handle_info.stop.duration",
        unit: {:native, :millisecond}
      ),

      # Phoenix endpoint metrics
      summary("phoenix.endpoint.start.system_time",
        unit: {:native, :millisecond}
      ),
      summary("phoenix.endpoint.stop.duration",
        unit: {:native, :millisecond}
      ),
      summary("phoenix.socket_connected.stop.duration",
        unit: {:native, :millisecond}
      ),
      summary("phoenix.channel_join.stop.duration",
        unit: {:native, :millisecond},
        tags: [:channel]
      ),
      summary("phoenix.channel_handled_in.stop.duration",
        unit: {:native, :millisecond},
        tags: [:event]
      ),

      # Database metrics
      summary("my_app.repo.query.total_time",
        unit: {:native, :millisecond},
        tags: [:source, :operation]
      ),
      summary("my_app.repo.query.decode_time",
        unit: {:native, :millisecond},
        tags: [:source, :operation]
      ),
      summary("my_app.repo.query.query_time",
        unit: {:native, :millisecond},
        tags: [:source, :operation]
      ),
      summary("my_app.repo.query.queue_time",
        unit: {:native, :millisecond},
        tags: [:source, :operation]
      ),
      summary("my_app.repo.query.idle_time",
        unit: {:native, :millisecond},
        tags: [:source, :operation]
      ),

      # HTTP metrics
      distribution("phoenix.router_dispatch.stop.duration",
        tags: [:route],
        unit: {:native, :millisecond}
      ),
      distribution("plug_router.dispatch.stop.duration",
        tags: [:route],
        unit: {:native, :millisecond}
      ),
      counter("phoenix.router_dispatch.exception", tags: [:route]),

      # Custom business metrics
      counter("phoenix_app.users.registered", tags: [:source]),
      summary("phoenix_app.users.registration_time",
        unit: {:native, :millisecond}
      )
    ]
  end

  defp periodic_measurements do
    [
      # Measure system stats
      {:system_info, :process_count},
      {:system_info, :port_count},
      {:system_info, :atom_count},
      {:erlang, :memory, []},
      {:erlang, :system_info, [:process_count, :port_count]}
    ]
  end
end
```

### Prometheus Configuration

```yaml
# prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'phoenix-app'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        regex: ([^:]+)(?::\d+)?;(\d+)
        replacement: $1:$2
        target_label: __address__
      - source_labels: [__meta_kubernetes_namespace]
        action: replace
        target_label: namespace
      - source_labels: [__meta_kubernetes_pod_name]
        action: replace
        target_label: pod
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: replace
        target_label: app

  - job_name: 'phoenix-app-cluster'
    static_configs:
      - targets:
          - 'phoenix-app-headless.default.svc.cluster.local:8080'
    metrics_path: /metrics

alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - alertmanager:9093

rule_files:
  - "alerts.yml"
```

### Alerting Rules

```yaml
# alerts.yml

groups:
  - name: phoenix_app
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(phoenix_router_dispatch_exception_total[5m])) 
          / sum(rate(phoenix_router_dispatch_stop_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }} over the last 5 minutes"

      - alert: HighLatency
        expr: |
          histogram_quantile(0.95, 
            sum(rate(phoenix_endpoint_stop_duration_seconds_bucket[5m])) by (le)
          ) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
          description: "95th percentile latency is {{ $value | humanizeDuration }}"

      - alert: LiveViewMemoryHigh
        expr: |
          phoenix_live_view_mount_memory_bytes / 1024 / 1024 > 500
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "LiveView memory usage high"
          description: "LiveView using {{ $value | humanize }}MB of memory"

      - alert: WebSocketConnectionsHigh
        expr: |
          phoenix_socket_connected_total - phoenix_channel_close_total > 10000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High WebSocket connection count"
          description: "{{ $value }} active WebSocket connections"

      - alert: PubSubBacklog
        expr: |
          phoenix_pubsub_broadcast_queue_length > 1000
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "PubSub broadcast backlog"
          description: "PubSub queue has {{ $value }} pending messages"

      - alert: DatabaseConnectionPoolExhausted
        expr: |
          my_app_repo_pool_available == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database connection pool exhausted"
          description: "No database connections available"

      - alert: NodeDown
        expr: |
          up{job="phoenix-app"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Phoenix node is down"
          description: "Node {{ $labels.pod }} has been down for more than 1 minute"

      - alert: ClusterPartition
        expr: |
          count(libcluster_node_view) < 3
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Possible cluster partition"
          description: "Only {{ $value }} nodes visible in cluster"
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Phoenix LiveView Dashboard",
    "panels": [
      {
        "title": "WebSocket Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "phoenix_socket_connected_total - phoenix_channel_close_total",
            "legendFormat": "Active Connections"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
      },
      {
        "title": "LiveView Mount Duration (p95)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(phoenix_live_view_mount_stop_duration_seconds_bucket[5m])) by (le, view))",
            "legendFormat": "{{ view }}"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
      },
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(phoenix_router_dispatch_stop_total[1m])) by (route)",
            "legendFormat": "{{ route }}"
          }
        ],
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 8}
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(phoenix_router_dispatch_exception_total[5m])) / sum(rate(phoenix_router_dispatch_stop_total[5m]))",
            "legendFormat": "Error Rate"
          }
        ],
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 16}
      },
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "vm_memory_total / 1024 / 1024",
            "legendFormat": "Total Memory (MB)"
          },
          {
            "expr": "phoenix_live_view_mount_memory_bytes / 1024 / 1024",
            "legendFormat": "LiveView Memory (MB)"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 24}
      },
      {
        "title": "PubSub Messages",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(phoenix_pubsub_broadcast_total[5m])",
            "legendFormat": "Broadcasts/s"
          },
          {
            "expr": "rate(phoenix_pubsub_subscribe_total[5m])",
            "legendFormat": "Subscribes/s"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 24}
      }
    ]
  }
}
```

## Performance Optimization

### Connection Pooling

```elixir
# config/prod.exs

import Config

# Database pool configuration
config :phoenix_app, PhoenixApp.Repo,
  url: System.get_env("DATABASE_URL"),
  pool_size: String.to_integer(System.get_env("POOL_SIZE") || "10"),
  pool_max_overflow: 10,
  queue_target: 50,
  queue_interval: 1000,
  timeout: 30_000,
  ssl: true,
  ssl_opts: [
    verify: :verify_peer,
    cacertfile: "/etc/ssl/certs/ca-certificates.crt"
  ]

# Redis connection pool
config :phoenix_app, :redis,
  url: System.get_env("REDIS_URL"),
  pool_size: 10,
  max_overflow: 5,
  queue_target: 50,
  queue_interval: 1000

# Configure Phoenix.PubSub with Redis adapter for distributed setup
config :phoenix_app, PhoenixApp.PubSub,
  adapter: Phoenix.PubSub.Redis,
  name: PhoenixApp.PubSub,
  redis_url: System.get_env("REDIS_URL"),
  redis_opts: [
    ssl: false,
    pool_size: 10
  ]
```

### LiveView Optimization

```elixir
# lib/phoenix_app_web/live/optimized_live.ex

defmodule PhoenixAppWeb.OptimizedLive do
  use Phoenix.LiveView

  @impl true
  def mount(_params, _session, socket) do
    # Optimize: Use streaming for large lists
    if connected?(socket) do
      # Only subscribe when connected
      Phoenix.PubSub.subscribe(PhoenixApp.PubSub, "updates")
    end

    {:ok, 
     socket
     |> stream(:items, [])
     |> assign(live_action: :index)}
  end

  @impl true
  def handle_info({:new_item, item}, socket) do
    # Optimize: Stream insert instead of full re-render
    {:noreply, stream_insert(socket, :items, item, at: 0)}
  end

  @impl true
  def handle_info(:full_refresh, socket) do
    # Rate limit full refreshes
    if rate_limit_allowed?(:full_refresh, socket.assigns.current_user.id) do
      items = fetch_items()
      {:noreply, stream(socket, :items, items, reset: true)}
    else
      {:noreply, socket}
    end
  end

  defp rate_limit_allowed?(action, user_id) do
    # Implement rate limiting logic
    true
  end
end
```

### Caching Strategies

```elixir
# lib/phoenix_app/caching.ex

defmodule PhoenixApp.Caching do
  @moduledoc """
  Caching strategies for LiveView
  """

  use GenServer
  alias Phoenix.HTML

  @doc """
  Cache rendered LiveView output
  """
  def cache_rendered_view(view_module, assigns, ttl_ms \\ 60_000) do
    key = render_cache_key(view_module, assigns)
    
    case Cachex.get(:render_cache, key) do
      {:ok, nil} ->
        # Cache miss - render
        rendered = view_module.render(assigns)
        Cachex.set(:render_cache, key, rendered, ttl: ttl_ms)
        rendered
      
      {:ok, cached} ->
        # Cache hit
        cached
    end
  end

  @doc """
  Generate cache key from assigns
  """
  defp render_cache_key(module, assigns) do
    :crypto.hash(:sha256, :erlang.term_to_binary({module, assigns}))
    |> Base.encode16(case: :lower)
  end

  @doc """
  Cache database query results
  """
  def cache_query(query_key, query_fn, ttl_ms \\ 300_000) do
    case Cachex.get(:query_cache, query_key) do
      {:ok, nil} ->
        result = query_fn.()
        Cachex.set(:query_cache, query_key, result, ttl: ttl_ms)
        result
      
      {:ok, cached} ->
        cached
    end
  end

  @doc """
  Invalidate cache by pattern
  """
  def invalidate_pattern(pattern) do
    Cachex.clear(:render_cache, pattern)
    Cachex.clear(:query_cache, pattern)
  end
end
```

## Security Hardening

### Content Security Policy

```elixir
# lib/phoenix_app_web/router.ex

defmodule PhoenixAppWeb.Router do
  use PhoenixAppWeb, :router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :put_root_layout, {PhoenixAppWeb.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers, %{"content-security-policy" => csp()}
    plug :put_secure_browser_headers
  end

  pipeline :api do
    plug :accepts, ["json"]
  end

  # Add CSP plug
  defp csp do
    "default-src 'self'; " <>
    "script-src 'self' 'unsafe-inline' https://cdn.example.com; " <>
    "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; " <>
    "font-src 'self' https://fonts.gstatic.com; " <>
    "img-src 'self' data: https:; " <>
    "connect-src 'self' wss://#{Application.get_env(:phoenix_app, :host)}; " <>
    "frame-ancestors 'none';"
  end

  scope "/", PhoenixAppWeb do
    pipe_through :browser
    live "/", HomeLive, :index
  end
end
```

### Rate Limiting

```elixir
# lib/phoenix_app_web/plugs/rate_limit.ex

defmodule PhoenixAppWeb.Plugs.RateLimit do
  @moduledoc """
  Rate limiting plug for LiveView
  """

  import Plug.Conn
  require Logger

  def init(opts), do: opts

  def call(conn, _opts) do
    user_id = get_user_id(conn)
    action = conn.request_path

    case check_rate_limit(user_id, action) do
      :allowed ->
        conn
      
      :blocked ->
        Logger.warn("Rate limit exceeded for user #{user_id} on #{action}")
        
        conn
        |> put_resp_header("x-ratelimit-remaining", "0")
        |> send_resp(429, "Rate limit exceeded")
        |> halt()
    end
  end

  defp get_user_id(conn) do
    # Get user ID from session or IP
    conn.assigns[:current_user]&.id || conn.remote_ip |> :inet.ntoa() |> to_string()
  end

  defp check_rate_limit(user_id, action) do
    # Using ExRated or custom implementation
    key = "#{user_id}:#{action}"
    
    case ExRated.check_rate(key, 1000, 10) do
      {:ok, _remaining} -> :allowed
      {:error, _limit} -> :blocked
    end
  end
end
```

### WebSocket Security

```elixir
# lib/phoenix_app_web/endpoint.ex

defmodule PhoenixAppWeb.Endpoint do
  use Phoenix.Endpoint, otp_app: :phoenix_app

  @session_options [
    store: :cookie,
    key: "_phoenix_app_key",
    signing_salt: "changing_this_is_dangerous",
    same_site: "Lax",
    secure: true,
    http_only: true
  ]

  socket "/live", Phoenix.LiveView.Socket,
    websocket: [
      connect_info: [:peer_data, :trace_context_headers, :uri, :user_agent, :x_forwarded_for, :cert, @session_options],
      timeout: 60_000,
      compress: true,
      check_origin: ["https://example.com"],
      transport_options: [
        socket_opts: [
          :inet6,
          keepalive: true,
          keepalive_interval: 30_000
        ]
      ]
    ],
    longpoll: false

  # ... rest of endpoint config
end
```

## Scaling Strategies

### Horizontal Scaling

```elixir
# lib/phoenix_app/scaling.ex

defmodule PhoenixApp.Scaling do
  @moduledoc """
  Horizontal scaling utilities
  """

  @doc """
  Get current cluster size
  """
  def cluster_size do
    Node.list()
    |> length()
    |> Kernel.+(1) # Include current node
  end

  @doc """
  Check if node should handle write operations
  Use for leader election in distributed scenarios
  """
  def is_leader_node? do
    nodes = [Node.self() | Node.list()] |> Enum.sort()
    hd(nodes) == Node.self()
  end

  @doc """
  Broadcast to all nodes in cluster
  """
  def broadcast_to_cluster(message) do
    for node <- Node.list() do
      :rpc.cast(node, __MODULE__, :receive_broadcast, [message])
    end
  end

  def receive_broadcast(message) do
    # Handle broadcast message
    Phoenix.PubSub.broadcast(PhoenixApp.PubSub, "cluster:events", message)
  end

  @doc """
  Distribute work across cluster nodes
  """
  def distribute_work(items) when is_list(items) do
    nodes = [Node.self() | Node.list()]
    num_nodes = length(nodes)
    
    items
    |> Enum.with_index()
    |> Enum.group_by(fn {_, i} -> rem(i, num_nodes) end)
    |> Enum.map(fn {node_idx, items} ->
      node = Enum.at(nodes, node_idx)
      {node, Enum.map(items, fn {item, _} -> item end)}
    end)
  end
end
```

### Load Balancer Configuration

```nginx
# nginx.conf

upstream phoenix_app {
    least_conn;
    server phoenix-app-1.phoenix-app.default.svc.cluster.local:8080 max_fails=3 fail_timeout=30s;
    server phoenix-app-2.phoenix-app.default.svc.cluster.local:8080 max_fails=3 fail_timeout=30s;
    server phoenix-app-3.phoenix-app.default.svc.cluster.local:8080 max_fails=3 fail_timeout=30s;
    keepalive 32;
}

server {
    listen 80;
    server_name app.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name app.example.com;

    ssl_certificate /etc/ssl/certs/app.example.com.crt;
    ssl_certificate_key /etc/ssl/private/app.example.com.key;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers on;

    # WebSocket support
    location /live/websocket {
        proxy_pass http://phoenix_app;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }

    location / {
        proxy_pass http://phoenix_app;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_cache off;
    }

    location /static {
        alias /var/www/phoenix_app/static;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

## Deployment Pipeline

### CI/CD with GitHub Actions

```yaml
# .github/workflows/deploy.yml

name: Deploy

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  MIX_ENV: prod
  ELIXIR_VERSION: '1.15'
  OTP_VERSION: '26.1'

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Elixir
        uses: erlef/setup-beam@v1
        with:
          elixir-version: ${{ env.ELIXIR_VERSION }}
          otp-version: ${{ env.OTP_VERSION }}
      
      - name: Cache deps
        uses: actions/cache@v3
        with:
          path: deps
          key: ${{ runner.os }}-mix-${{ hashFiles('mix.lock') }}
      
      - name: Cache _build
        uses: actions/cache@v3
        with:
          path: _build
          key: ${{ runner.os }}-mix-${{ hashFiles('mix.lock') }}
      
      - name: Install deps
        run: mix deps.get
      
      - name: Compile
        run: mix compile
      
      - name: Run tests
        run: mix test
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/phoenix_app_test
      
      - name: Run dialyzer
        run: mix dialyzer
      
      - name: Run credo
        run: mix credo --strict

  build:
    needs: test
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2
      
      - name: Login to Container Registry
        uses: docker/login-action@v2
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Build and push
        uses: docker/build-push-action@v4
        with:
          context: .
          push: true
          tags: ghcr.io/${{ github.repository }}:latest,ghcr.io/${{ github.repository }}:${{ github.sha }}
          cache-from: type=registry,ref=ghcr.io/${{ github.repository }}:buildcache
          cache-to: type=registry,ref=ghcr.io/${{ github.repository }}:buildcache,mode=max

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up kubectl
        uses: azure/setup-kubectl@v3
        with:
          version: 'v1.28.0'
      
      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig
      
      - name: Deploy to Kubernetes
        run: |
          kubectl apply -f k8s/
          kubectl rollout restart deployment/phoenix-app
          kubectl rollout status deployment/phoenix-app
      
      - name: Run smoke tests
        run: |
          curl -f https://app.example.com/health/readiness
```

## Conclusion

Production-grade Phoenix LiveView applications require:

1. **Cluster Configuration**: Proper node discovery, distributed Erlang, cookie management
2. **Monitoring**: Prometheus metrics, Grafana dashboards, alerting rules
3. **Performance**: Connection pooling, caching, streaming, rate limiting
4. **Security**: CSP headers, WebSocket security, session hardening
5. **Scaling**: Horizontal scaling with load balancers, Kubernetes deployments
6. **CI/CD**: Automated testing, container builds, deployment pipelines
