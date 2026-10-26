# oss icon pack provenance

Where each file under `assets/icons/oss/` came from, so its bytes can be re-verified.

## Sources

Each file is byte-identical (sha256) to the file at its source. Nothing was recolored, resized, cropped, minified or re-exported; local names are shortened to a stem and file content is untouched.

- Git repositories, pinned to the commit named in the row: the file is `https://raw.githubusercontent.com/<owner>/<repository>/<commit>/<path>`, and its `git hash-object` equals the blob sha GitHub reports for that path at that commit. The CNCF files come from <https://github.com/cncf/artwork> at `b4b5989332c822ef09d5b38a512e2b39c63e32f2`, each the `icon/color` variant.
- The Docker logo kit <https://www.docker.com/static/Docker-Logos-1.zip>, linked from <https://www.docker.com/company/newsroom/media-resources/>, sha256 `590790ecdaaa577e55ce3dcad3a6c67a04ca7f8cc53a4a2fa21ac69d6767d476`.
- Website files, which cannot be pinned: the ASF logo collection under <https://www.apache.org/logos/originals/>, and the files the project or vendor site serves (opensearch.org, docs.nginx.com, wiki.postgresql.org, cdn.haproxy.com, traefik.io). `opensearch.svg` is also the entry `SVG/Mark/opensearch_mark_default.svg` of <https://opensearch.org/wp-content/uploads/2025/01/opensearch-assets_SVG.zip> (sha256 `853dff05ffdc3931e8ef552166fd4463d7a45ae17ed3a83ec12d515b0ee21356`).

Fetched 2026-10-25 (16 files) and 2026-10-26 (14 files), as the Fetched column says per file. To re-verify, download each source and compare its sha256 with the file's row in section 8.2 of docs/SPEC.md; a file whose source no longer matches goes back through review before the pack ships again.

## Files

| Local file | Product | Source URL | Source path or commit | Fetched |
|---|---|---|---|---|
| `airflow.svg` | Apache Airflow | <https://github.com/apache/airflow-site> | `landing-pages/site/static/images/airflow-icon.svg` at `79a84030c75c1a4743f1d50c92218de54e353c37` | 2026-10-26 |
| `argo.svg` | Argo | <https://github.com/cncf/artwork> | `projects/argo/icon/color/argo-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `cassandra.svg` | Apache Cassandra | <https://www.apache.org/logos/originals/cassandra-4.svg> | website file, not pinnable | 2026-10-26 |
| `clickhouse.svg` | ClickHouse | <https://github.com/ClickHouse/ClickHouse> | `docs/img/clickhouse-logo-mark.svg` at `8822a146459fb4057a46e4e1e0247ed51634f257` | 2026-10-26 |
| `docker.svg` | Docker | <https://www.docker.com/static/Docker-Logos-1.zip> | `docker-logos/SVG/docker-mark-ocean-blue.svg` in the archive | 2026-10-26 |
| `envoy.svg` | Envoy | <https://github.com/cncf/artwork> | `projects/envoy/icon/color/envoy-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `etcd.svg` | etcd | <https://github.com/cncf/artwork> | `projects/etcd/icon/color/etcd-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `grafana.svg` | Grafana | <https://github.com/grafana/grafana> | `public/img/grafana_icon.svg` at `299371878b4bbd2651759ff9f4f7a56360959415` | 2026-10-26 |
| `haproxy.svg` | HAProxy | <https://cdn.haproxy.com/assets/our_logos/haproxy-icon.svg> | website file, not pinnable | 2026-10-26 |
| `istio.svg` | Istio | <https://github.com/cncf/artwork> | `projects/istio/icon/color/istio-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `jaeger.svg` | Jaeger | <https://github.com/cncf/artwork> | `projects/jaeger/icon/color/jaeger-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `jenkins.svg` | Jenkins | <https://github.com/jenkins-infra/jenkins.io> | `content/images/logos/jenkins/jenkins.svg` at `2888b59262a95fddb6ae02733abd2a94ebc66974` | 2026-10-25 |
| `kafka.svg` | Apache Kafka | <https://github.com/apache/kafka-site> | `assets/icons/logo.svg` at `242456fde7fa2384339b9fbb0e91673e5d2859e2` | 2026-10-26 |
| `keycloak.svg` | Keycloak | <https://github.com/cncf/artwork> | `projects/keycloak/icon/color/keycloak-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `kubernetes.svg` | Kubernetes | <https://github.com/cncf/artwork> | `projects/kubernetes/icon/color/kubernetes-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `linkerd.svg` | Linkerd | <https://github.com/cncf/artwork> | `projects/linkerd/icon/color/linkerd-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `loki.svg` | Grafana Loki | <https://github.com/grafana/grafana> | `public/app/features/loki-helpers/loki_icon.svg` at `299371878b4bbd2651759ff9f4f7a56360959415` | 2026-10-26 |
| `mongodb.svg` | MongoDB | <https://github.com/mongodb/mongo> | `docs/leaf.svg` at `6bfa1c14d4b2ccc0d5d9bbe876ee287647c775e2` | 2026-10-26 |
| `nats.svg` | NATS | <https://github.com/cncf/artwork> | `projects/nats/icon/color/nats-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `nginx.svg` | NGINX | <https://docs.nginx.com/images/icons/NGINX-Open-Source-product-icon.svg> | website file, not pinnable | 2026-10-26 |
| `opensearch.svg` | OpenSearch | <https://opensearch.org/wp-content/uploads/2025/01/opensearch_mark_default.svg> | website file, not pinnable | 2026-10-25 |
| `opentelemetry.svg` | OpenTelemetry | <https://github.com/cncf/artwork> | `projects/opentelemetry/icon/color/opentelemetry-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `postgresql.svg` | PostgreSQL | <https://wiki.postgresql.org/images/a/a4/PostgreSQL_logo.3colors.svg> | website file, not pinnable | 2026-10-26 |
| `prometheus.svg` | Prometheus | <https://github.com/cncf/artwork> | `projects/prometheus/icon/color/prometheus-icon-color.svg` at `b4b5989332c822ef09d5b38a512e2b39c63e32f2` | 2026-10-25 |
| `rabbitmq.svg` | RabbitMQ | <https://github.com/rabbitmq/rabbitmq-website> | `static/img/rabbitmq-logo.svg` at `a284a068ab720672b40ceed40f3f151d9829fb70` | 2026-10-26 |
| `redis.svg` | Redis | <https://github.com/redis/docs> | `assets/icons/redis-cube.svg` at `4fe762f8dab40f0b661207374378ed52113c9fe3` | 2026-10-25 |
| `traefik.svg` | Traefik Proxy | <https://traefik.io/images/header/products/traefik-proxy.svg> | website file, not pinnable | 2026-10-26 |
| `valkey.svg` | Valkey | <https://github.com/valkey-io/valkey-io.github.io> | `static/img/Valkey-logo.svg` at `5900f066c3d02fae66e87c9f59fbd4c704ef2845` | 2026-10-25 |
| `vault.svg` | HashiCorp Vault | <https://github.com/hashicorp/design-system> | `packages/flight-icons/svg-original/vault-color-24.svg` at `a160656e5e762f87baa57c4e6451a0c56ce1cdb4` | 2026-10-26 |
