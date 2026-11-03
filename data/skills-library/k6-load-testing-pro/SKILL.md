---
name: k6-load-testing-pro
description: High-concurrency load & stress testing: Grafana k6, Artillery, spike/soak testing scenarios, p95/p99 latency thresholds, bottleneck detection, CI performance gates.
---

# k6 Load & Concurrency Testing Pro

Specialist in simulating realistic user traffic, identifying server bottlenecks, and enforcing performance SLOs in CI/CD.

## Core Directives
1. **Scenario Architectures**:
   - Smoke Testing: 1-5 Virtual Users (VUs) to verify test script validity and base response times.
   - Load Testing: gradual ramp-up to peak expected traffic to measure p95/p99 latency stability.
   - Stress & Spike Testing: aggressive VU surges beyond normal capacity to determine failure thresholds and recovery characteristics.
   - Soak Testing: sustained load over hours to detect memory leaks and connection pool starvation.
2. **Thresholds & Performance Budgets**:
   - Enforce pass/fail criteria directly in script: `thresholds: { http_req_duration: ['p(95)<300', 'p(99)<600'], http_req_failed: ['rate<0.01'] }`.
3. **Realistic Traffic Modeling**:
   - Simulate realistic user think time with `sleep(Math.random() * 2 + 1)`.
   - Parameterize test data using shared data arrays (unique login credentials, valid query IDs).
4. **Metric Analysis & Root Cause Identification**:
   - Distinguish between client-side queueing (`http_req_connecting`), TLS handshake overhead, and server-side processing delay (`http_req_waiting`).
5. **CI/CD Performance Gates**: run lightweight regression performance tests on PR merges to prevent latency regressions.
