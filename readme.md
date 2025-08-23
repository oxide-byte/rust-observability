# Observability

## Introduction

A service that is not observed, is lost in the universe of distributed systems. This POC-project is a small sample to show how keep track on your Rust Service, build on Axum. The Service and all the infrastructure is dockerized, you can simply start up all with "docker-compose up"

A small Screenshot the observed service in Grafana:

![alt text](doc/images/sample.png "sample")

```text
INFO: Still experimenting with different Cargo Libraries
```

## Links

The current links available by the application:

The Rust Service:

* Hosted on port 8080: http://127.0.0.1:8080/
* API: http://127.0.0.1:8080/api/demo for generating log entries
* Metrics: http://127.0.0.1:8080/metrics for collecting the current metrics of the Rust Service

Prometheus: 

* http://127.0.0.1:9090/

Grafana: 

* http://127.0.0.1:3000/

Loki: 

* A simple query on Loki: http://127.0.0.1:3100/loki/api/v1/query?query={job="docker-logs"}

* A query with a range option:
  - http://127.0.0.1:3100/loki/api/v1/query_range?query={job="docker-logs"}&limit=1000&direction=backward&start=<rfc3339_or_unix_ns>&end=<rfc3339_or_unix_ns>

## Architecture: Logs and Metrics Flow

A little Flowchart for illustrating how the different logs are collected and transmitted to the different systems:

```mermaid
flowchart LR
  subgraph Docker Network
    A[Rust-Axum-Server]
    D[(Docker JSON logs<br/>/var/lib/docker/containers/*/*-json.log)]
    PT[Promtail]
    L[Loki]
    PR[Prometheus]
    G[Grafana]
  end

  %% Logs flow (1)
  A -- stdout logs --> D
  D -- tail --> PT
  PT -- push /loki/api/v1/push --> L
  G -- query (Loki datasource) --> L

  %% Logs flow (2)
  A -- push (tracing-loki) --> L

  %% Metrics flow
  A -- scrape /metrics --> PR
  PT -- scrape /metrics (9080) --> PR
  L -- scrape /metrics (3100) --> PR
  G -- query (Prometheus datasource) --> PR
```

Notes:
- Promtail collects logs from Docker JSON log files and pushes them to Loki. Loki does not pull logs from the app. (Solution 1)
- Cargo tracing-loki, declare appender, push to Loki (Solution 2)
- Prometheus scrapes metrics from the app, Promtail, and Loki; logs are not stored in Prometheus.
- Grafana uses the Loki datasource for logs and the Prometheus datasource for metrics.

## Docker-Compose

Starting the application:

```shell
docker-compose up
```