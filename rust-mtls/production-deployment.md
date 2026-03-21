# Production mTLS Deployment Guide

## Overview

This document covers Kubernetes deployment patterns, service mesh integration, and production operational considerations for mTLS-enabled Rust services.

## Kubernetes Deployment

### Basic mTLS Deployment

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: api-gateway-tls
  namespace: default
type: kubernetes.io/tls
data:
  tls.crt: <base64-encoded-cert>
  tls.key: <base64-encoded-key>
  ca.crt: <base64-encoded-ca>
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-gateway
  namespace: default
spec:
  replicas: 3
  selector:
    matchLabels:
      app: api-gateway
  template:
    metadata:
      labels:
        app: api-gateway
    spec:
      containers:
        - name: api-gateway
          image: myregistry/api-gateway:latest
          ports:
            - containerPort: 8443
              name: https
          volumeMounts:
            - name: tls-certs
              mountPath: /etc/tls
              readOnly: true
          env:
            - name: MTLS_CERT_PATH
              value: /etc/tls/tls.crt
            - name: MTLS_KEY_PATH
              value: /etc/tls/tls.key
            - name: MTLS_CA_PATH
              value: /etc/tls/ca.crt
          readinessProbe:
            httpGet:
              path: /health
              port: 8443
              scheme: HTTPS
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /health
              port: 8443
              scheme: HTTPS
            initialDelaySeconds: 15
            periodSeconds: 20
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "500m"
      volumes:
        - name: tls-certs
          secret:
            secretName: api-gateway-tls
---
apiVersion: v1
kind: Service
metadata:
  name: api-gateway
  namespace: default
spec:
  selector:
    app: api-gateway
  ports:
    - port: 443
      targetPort: 8443
      protocol: TCP
      name: https
  type: ClusterIP
```

### Certificate Mounting Security

```yaml
# Secure secret mounting with proper permissions
spec:
  containers:
    - name: api-gateway
      volumeMounts:
        - name: tls-certs
          mountPath: /etc/tls
          readOnly: true
        - name: tls-key
          mountPath: /etc/tls-key
          readOnly: true
  volumes:
    - name: tls-certs
      secret:
        secretName: api-gateway-tls
        items:
          - key: tls.crt
            path: tls.crt
          - key: ca.crt
            path: ca.crt
        defaultMode: 0644
    - name: tls-key
      secret:
        secretName: api-gateway-tls
        items:
          - key: tls.key
            path: tls.key
        defaultMode: 0600  # Restrictive permissions for private key
```

### Service-to-Service Communication

```yaml
# Client service configuration
apiVersion: apps/v1
kind: Deployment
metadata:
  name: order-service
spec:
  replicas: 2
  selector:
    matchLabels:
      app: order-service
  template:
    metadata:
      labels:
        app: order-service
    spec:
      serviceAccountName: order-service-sa
      containers:
        - name: order-service
          image: myregistry/order-service:latest
          env:
            - name: API_GATEWAY_URL
              value: "https://api-gateway.default.svc.cluster.local:443"
            - name: MTLS_CERT_PATH
              value: /etc/tls/tls.crt
            - name: MTLS_KEY_PATH
              value: /etc/tls/tls.key
            - name: MTLS_CA_PATH
              value: /etc/tls/ca.crt
          volumeMounts:
            - name: tls-certs
              mountPath: /etc/tls
              readOnly: true
      volumes:
        - name: tls-certs
          secret:
            secretName: order-service-tls
```

## Service Mesh Integration

### Linkerd mTLS

```yaml
# Enable automatic mTLS with Linkerd
apiVersion: linkerd.io/v1alpha2
kind: Server
metadata:
  name: api-gateway-server
  namespace: default
spec:
  podSelector:
    matchLabels:
      app: api-gateway
  port: 8443
  proxyProtocol: TLS
---
apiVersion: linkerd.io/v1alpha2
kind: ServerAuthorization
metadata:
  name: api-gateway-authz
  namespace: default
spec:
  server:
    name: api-gateway-server
  client:
    # Only allow traffic from specific service accounts
    serviceAccounts:
      - name: order-service
        namespace: default
      - name: user-service
        namespace: default
```

### Istio mTLS Policy

```yaml
# Enable strict mTLS for namespace
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default-mtls
  namespace: default
spec:
  mtls:
    mode: STRICT
---
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: api-gateway-policy
  namespace: default
spec:
  selector:
    matchLabels:
      app: api-gateway
  action: ALLOW
  rules:
    - from:
        - source:
            principals:
              - cluster.local/ns/default/sa/order-service
              - cluster.local/ns/default/sa/user-service
```

### Istio Sidecar Configuration

```yaml
apiVersion: networking.istio.io/v1beta1
kind: Sidecar
metadata:
  name: api-gateway-sidecar
  namespace: default
spec:
  workloadSelector:
    labels:
      app: api-gateway
  ingress:
    - port:
        number: 8443
        protocol: HTTPS
        name: https
      tls:
        mode: MUTUAL
        credentialName: api-gateway-tls
      defaultEndpoint: 127.0.0.1:8443
  egress:
    - hosts:
        - "./*"
```

## Certificate Management with cert-manager

### CA Issuer Setup

```yaml
# Secret containing CA key and cert
apiVersion: v1
kind: Secret
metadata:
  name: mtls-ca-secret
  namespace: cert-manager
type: kubernetes.io/tls
data:
  tls.crt: <base64-ca-cert>
  tls.key: <base64-ca-key>
---
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: mtls-cluster-issuer
spec:
  ca:
    secretName: mtls-ca-secret
```

### Certificate Resources

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: api-gateway-cert
  namespace: default
spec:
  secretName: api-gateway-tls
  duration: 168h  # 7 days
  renewBefore: 24h  # Renew 1 day before expiry
  issuerRef:
    name: mtls-cluster-issuer
    kind: ClusterIssuer
  commonName: api-gateway.default.svc.cluster.local
  dnsNames:
    - api-gateway
    - api-gateway.default.svc.cluster.local
  uriSANs:
    - spiffe://example.com/ns/default/sa/api-gateway
  usages:
    - digital signature
    - key encipherment
    - server auth
    - client auth
  privateKey:
    algorithm: RSA
    size: 2048
    encoding: PKCS1
  revisionHistoryLimit: 3
```

### Certificate Monitoring

```yaml
# Prometheus alerts for certificate expiry
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: certificate-expiry-alerts
  namespace: monitoring
spec:
  groups:
    - name: certificates
      rules:
        - alert: CertificateExpiringWarning
          expr: certmanager_certificate_expiration_timestamp_seconds - time() < 604800
          for: 1h
          labels:
            severity: warning
          annotations:
            summary: "Certificate expiring in less than 7 days"

        - alert: CertificateExpiringCritical
          expr: certmanager_certificate_expiration_timestamp_seconds - time() < 86400
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: "Certificate expiring in less than 24 hours"

        - alert: CertificateRenewalFailed
          expr: certmanager_certificate_ready_status{condition="False"} == 1
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: "Certificate renewal failed"
```

## Multi-Cluster mTLS

### Cross-Cluster Trust

```yaml
# Export CA bundle from cluster A
apiVersion: v1
kind: ConfigMap
metadata:
  name: cluster-a-ca-bundle
  namespace: istio-system
data:
  ca.crt: |
    -----BEGIN CERTIFICATE-----
    ... Cluster A CA certificate ...
    -----END CERTIFICATE-----
---
# In cluster B, create PeerAuthentication that trusts cluster A
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: cross-cluster-mtls
  namespace: istio-system
spec:
  mtls:
    mode: STRICT
  selector:
    matchLabels:
      app: cross-cluster-gateway
```

### Service Mesh Federation

```yaml
# ServiceEntry for cross-cluster service
apiVersion: networking.istio.io/v1beta1
kind: ServiceEntry
metadata:
  name: external-api-gateway
  namespace: default
spec:
  hosts:
    - api-gateway.cluster-a.svc
  location: MESH_INTERNAL
  ports:
    - number: 443
      name: https
      protocol: HTTPS
  resolution: DNS
  endpoints:
    - address: api-gateway.cluster-a.example.com
```

## Load Balancer Integration

### AWS NLB with mTLS Termination

```yaml
apiVersion: v1
kind: Service
metadata:
  name: api-gateway
  namespace: default
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
    service.beta.kubernetes.io/aws-load-balancer-ssl-cert: "arn:aws:acm:..."
    service.beta.kubernetes.io/aws-load-balancer-ssl-ports: "443"
    service.beta.kubernetes.io/aws-load-balancer-backend-protocol: "tcp"
spec:
  type: LoadBalancer
  ports:
    - port: 443
      targetPort: 8443
      protocol: TCP
  selector:
    app: api-gateway
```

### NGINX Ingress with mTLS

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: api-gateway-ingress
  namespace: default
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/auth-tls-verify-client: "on"
    nginx.ingress.kubernetes.io/auth-tls-secret: "default/ca-secret"
    nginx.ingress.kubernetes.io/auth-tls-verify-depth: "2"
    nginx.ingress.kubernetes.io/auth-tls-pass-certificate-to-upstream: "true"
spec:
  tls:
    - hosts:
        - api.example.com
      secretName: api-gateway-tls
  rules:
    - host: api.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: api-gateway
                port:
                  number: 443
```

## Blue-Green Deployment

### Certificate Rotation Strategy

```yaml
# Phase 1: Deploy new certificates to subset
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: api-gateway
spec:
  replicas: 5
  strategy:
    canary:
      canaryService: api-gateway-canary
      stableService: api-gateway-stable
      steps:
        - setWeight: 20  # 20% traffic to canary
        - pause: {duration: 5m}
        - setWeight: 50
        - pause: {duration: 5m}
        - setWeight: 100
  template:
    spec:
      containers:
        - name: api-gateway
          volumeMounts:
            - name: tls-certs
              mountPath: /etc/tls
      volumes:
        - name: tls-certs
          secret:
            secretName: api-gateway-tls-v2  # New certificate version
```

## Observability

### Metrics Collection

```yaml
# PodMonitor for Prometheus
apiVersion: monitoring.coreos.com/v1
kind: PodMonitor
metadata:
  name: api-gateway-monitor
  namespace: default
spec:
  selector:
    matchLabels:
      app: api-gateway
  podMetricsEndpoints:
    - port: metrics
      path: /metrics
      interval: 30s
```

### Distributed Tracing

```yaml
# OpenTelemetry Collector config
apiVersion: opentelemetry.io/v1alpha1
kind: OpenTelemetryCollector
metadata:
  name: otel-collector
  namespace: monitoring
spec:
  config: |
    receivers:
      otlp:
        protocols:
          grpc:
          http:
    processors:
      batch:
    exporters:
      jaeger:
        endpoint: jaeger-collector:14250
        tls:
          cert_file: /etc/tls/tls.crt
          key_file: /etc/tls/tls.key
          ca_file: /etc/tls/ca.crt
          insecure: false
          insecure_skip_verify: false
    service:
      pipelines:
        traces:
          receivers: [otlp]
          processors: [batch]
          exporters: [jaeger]
```

## Disaster Recovery

### CA Backup and Recovery

```yaml
# Velero backup for certificate secrets
apiVersion: velero.io/v1
kind: Backup
metadata:
  name: certificates-backup
spec:
  includedNamespaces:
    - default
    - cert-manager
  includedResources:
    - secrets
  labelSelector:
    matchLabels:
      backup-type: certificates
  ttl: 720h  # 30 days
---
# Scheduled backup
apiVersion: velero.io/v1
kind: Schedule
metadata:
  name: certificates-daily-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  template:
    includedNamespaces:
      - default
      - cert-manager
    includedResources:
      - secrets
    labelSelector:
      matchLabels:
        backup-type: certificates
```

### Certificate Recovery Procedure

```bash
#!/bin/bash
# recover-certificates.sh

# 1. Verify current certificate status
kubectl get certificates -n default

# 2. Check cert-manager logs
kubectl logs -n cert-manager -l app=cert-manager

# 3. Force certificate renewal
kubectl cert-manager renew api-gateway-cert -n default

# 4. If CA is compromised, issue new certificates
#    a. Generate new CA
#    b. Update ClusterIssuer
#    c. Delete and recreate all certificates
#    d. Restart all services

# 5. Verify services
kubectl rollout restart deployment/api-gateway -n default
```

## Environment Configuration

### Development vs Production

```yaml
# ConfigMap for environment-specific settings
apiVersion: v1
kind: ConfigMap
metadata:
  name: mtls-config
  namespace: default
data:
  # Development settings
  dev: |
    CERT_LIFETIME: 90d
    RENEW_BEFORE: 7d
    CLIENT_AUTH_REQUIRED: "false"
    LOG_LEVEL: "debug"

  # Production settings
  prod: |
    CERT_LIFETIME: 7d
    RENEW_BEFORE: 24h
    CLIENT_AUTH_REQUIRED: "true"
    LOG_LEVEL: "info"
    CIPHER_SUITES: "TLS_AES_256_GCM_SHA384,TLS_CHACHA20_POLY1305_SHA256"
```

## Operational Runbooks

### Certificate Rotation Runbook

```markdown
## Certificate Rotation Procedure

### Scheduled Rotation (Automated)
1. cert-manager automatically renews certificates based on `renewBefore` setting
2. Services reload certificates via file watch or graceful restart
3. Verify new certificates are deployed:
   ```bash
   kubectl get secret api-gateway-tls -o jsonpath='{.data.tls\.crt}' | base64 -d | openssl x509 -noout -dates
   ```

### Emergency Rotation (Compromised Key)
1. Revoke compromised certificate immediately
2. Generate new CA if root compromise
3. Issue new certificates to all services
4. Force restart all services
5. Verify mTLS connectivity
6. Update CRL/OCSP responders

### Troubleshooting Failed Rotation
1. Check cert-manager logs
2. Verify issuer configuration
3. Check secret permissions
4. Validate certificate template
5. Review admission webhook logs
```

### Incident Response

```markdown
## mTLS Incident Response

### Symptoms
- Increased 503 errors
- TLS handshake failures in logs
- Services unable to communicate

### Diagnosis
1. Check certificate expiry:
   ```bash
   for cert in $(kubectl get secrets -o name | grep tls); do
     echo "=== $cert ==="
     kubectl get $cert -o jsonpath='{.data.tls\.crt}' | base64 -d | \
       openssl x509 -noout -dates
   done
   ```

2. Check mTLS handshake metrics:
   ```bash
   kubectl port-forward svc/prometheus 9090
   # Query: rate(tls_handshake_total{status="failed"}[5m])
   ```

3. Verify CA chain:
   ```bash
   openssl verify -CAfile ca.crt -untrusted intermediate.crt service.crt
   ```

### Resolution
- If expired: Force renewal and restart
- If CA mismatch: Update trust bundle
- If configuration error: Fix and redeploy
```
