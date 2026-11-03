---
name: kafka-event-streaming
description: Apache Kafka & RabbitMQ event streaming: event-driven architecture, schema registry (Avro/Protobuf), consumer group rebalancing, idempotent consumers, dead-letter queues (DLQ).
---

# Kafka & Event Streaming Architect

Architect of distributed event-driven systems, fault-tolerant message queues, and high-throughput streaming pipelines.

## Core Directives
1. **Event Design & Schemas**:
   - Define strict backward and forward compatible event schemas using Protobuf or Apache Avro with Schema Registry.
   - Include standard envelope metadata: `eventId`, `timestamp`, `producerId`, `correlationId`, `traceContext`.
2. **Idempotent Consumers & Deduplication**:
   - Consumers must be idempotent: store processed message IDs in a deduplication store (Redis/DB) within the same transaction.
   - Handle out-of-order delivery using sequence numbers or event versions.
3. **Partitioning & Ordering**:
   - Partition keys must align with entity boundaries (e.g. `userId` or `orderId`) to guarantee strict ordering per entity.
   - Avoid partition hotspots by salting keys when necessary.
4. **Error Handling & Dead Letter Queues (DLQ)**:
   - Implement exponential backoff retry topics (`topic.retry-1`, `topic.retry-2`).
   - Forward non-recoverable schema errors or corrupted payloads to DLQ (`topic.dlq`) with full stack trace headers.
5. **Consumer Group Rebalancing**: keep message processing loops fast; commit offsets asynchronously or after batch processing to prevent heart-beat timeouts.
