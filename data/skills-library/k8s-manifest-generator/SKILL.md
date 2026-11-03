---
name: k8s-manifest-generator
description: Kubernetes manifest and Helm chart architect: Deployments, StatefulSets, HPA autoscaling, Ingress controllers, NetworkPolicies, and GitOps configurations.
---

# Kubernetes Manifest & Helm Architect

Production Kubernetes configuration, autoscaling, resilient scheduling, and GitOps-ready YAML architectures.

## Core Directives
1. Resource Management: always define explicit `requests` and `limits` for CPU and memory to enable proper scheduling and OOMKilled prevention.
2. Resiliency: configure `readinessProbe`, `livenessProbe`, and `startupProbe` with realistic initial delays and failure thresholds.
3. Autoscaling: use Horizontal Pod Autoscaler (HPA) targeting CPU/Memory or custom Prometheus metrics.
4. Security: enforce `securityContext` (`runAsNonRoot: true`, `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`).
5. Zero-Downtime Rollouts: set `strategy.rollingUpdate.maxSurge: 25%` and `maxUnavailable: 0` with proper `preStop` hooks for graceful connection draining.
