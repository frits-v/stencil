//! The oss pack, `assets/icons/oss/`.

use stencil_model::OssIcon;

pub(super) fn svg_bytes(icon: OssIcon) -> &'static [u8] {
    match icon {
        OssIcon::Airflow => include_bytes!("../../../../assets/icons/oss/airflow.svg"),
        OssIcon::Argo => include_bytes!("../../../../assets/icons/oss/argo.svg"),
        OssIcon::Cassandra => include_bytes!("../../../../assets/icons/oss/cassandra.svg"),
        OssIcon::Clickhouse => include_bytes!("../../../../assets/icons/oss/clickhouse.svg"),
        OssIcon::Docker => include_bytes!("../../../../assets/icons/oss/docker.svg"),
        OssIcon::Envoy => include_bytes!("../../../../assets/icons/oss/envoy.svg"),
        OssIcon::Etcd => include_bytes!("../../../../assets/icons/oss/etcd.svg"),
        OssIcon::Grafana => include_bytes!("../../../../assets/icons/oss/grafana.svg"),
        OssIcon::Haproxy => include_bytes!("../../../../assets/icons/oss/haproxy.svg"),
        OssIcon::Helm => include_bytes!("../../../../assets/icons/oss/helm.svg"),
        OssIcon::Istio => include_bytes!("../../../../assets/icons/oss/istio.svg"),
        OssIcon::Jaeger => include_bytes!("../../../../assets/icons/oss/jaeger.svg"),
        OssIcon::Jenkins => include_bytes!("../../../../assets/icons/oss/jenkins.svg"),
        OssIcon::Kafka => include_bytes!("../../../../assets/icons/oss/kafka.svg"),
        OssIcon::Keycloak => include_bytes!("../../../../assets/icons/oss/keycloak.svg"),
        OssIcon::Kubernetes => include_bytes!("../../../../assets/icons/oss/kubernetes.svg"),
        OssIcon::Linkerd => include_bytes!("../../../../assets/icons/oss/linkerd.svg"),
        OssIcon::Loki => include_bytes!("../../../../assets/icons/oss/loki.svg"),
        OssIcon::Mongodb => include_bytes!("../../../../assets/icons/oss/mongodb.svg"),
        OssIcon::Nats => include_bytes!("../../../../assets/icons/oss/nats.svg"),
        OssIcon::Nginx => include_bytes!("../../../../assets/icons/oss/nginx.svg"),
        OssIcon::Opensearch => include_bytes!("../../../../assets/icons/oss/opensearch.svg"),
        OssIcon::Opentelemetry => include_bytes!("../../../../assets/icons/oss/opentelemetry.svg"),
        OssIcon::Postgresql => include_bytes!("../../../../assets/icons/oss/postgresql.svg"),
        OssIcon::Prometheus => include_bytes!("../../../../assets/icons/oss/prometheus.svg"),
        OssIcon::Rabbitmq => include_bytes!("../../../../assets/icons/oss/rabbitmq.svg"),
        OssIcon::Redis => include_bytes!("../../../../assets/icons/oss/redis.svg"),
        OssIcon::Traefik => include_bytes!("../../../../assets/icons/oss/traefik.svg"),
        OssIcon::Valkey => include_bytes!("../../../../assets/icons/oss/valkey.svg"),
        OssIcon::Vault => include_bytes!("../../../../assets/icons/oss/vault.svg"),
    }
}
