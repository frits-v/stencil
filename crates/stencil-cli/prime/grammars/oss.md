# Grammar oss

Self-hosted open-source components drawn with each project's own logo: set "grammar": "oss" on the page. The kinds are plain's (`stencil prime grammar plain`) plus cluster, so a plain page becomes an oss page by setting the grammar and adding icons.

{{kinds}}

{{icons}}

| Kind | Means |
|---|---|
| cluster | a Kubernetes cluster or namespace: pink, no border, never tinted |
| service | a service the owner runs, and the tools that run it: Kubernetes, Docker, Argo CD, Jenkins, Helm, Keycloak, Vault, Airflow, Spark, Prometheus, Grafana, Loki, Jaeger, OpenTelemetry; a block, Kubernetes and Docker a tower |
| store | PostgreSQL, MySQL, MariaDB, MongoDB, Redis, Valkey, Cassandra, ClickHouse, Elasticsearch, OpenSearch, etcd; a cylinder, MinIO a stack |
| queue | Kafka, RabbitMQ, NATS, Pulsar; a stack |
| network | NGINX, Envoy, Istio, Linkerd, HAProxy, Traefik; a tile |

managed, external, person and device take no icon and mean what they mean under plain.

Icons: an item takes a stem from its own kind's line above, and its subtitle names the project (PostgreSQL, Apache Kafka). icon-matches-product reads the item's kind's table, so a queue whose subtitle says Kafka and whose icon is postgresql is a defect. A product with no bundled logo (MySQL, Elasticsearch) is named in its subtitle without an icon; another project's icon on it is a defect.

Rules: CUE rejects an external item inside a system Box, as under plain, and tint pairing holds. remembered is empty: a version or a host is a built fact.
