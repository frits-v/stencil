//! The oss pack, `assets/icons/oss/`: each project's own logo file.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OssIcon {
    Airflow,
    Argo,
    Cassandra,
    Clickhouse,
    Docker,
    Envoy,
    Etcd,
    Grafana,
    Haproxy,
    Helm,
    Istio,
    Jaeger,
    Jenkins,
    Kafka,
    Keycloak,
    Kubernetes,
    Linkerd,
    Loki,
    Mariadb,
    Minio,
    Mongodb,
    Nats,
    Nginx,
    Opensearch,
    Opentelemetry,
    Postgresql,
    Prometheus,
    Pulsar,
    Rabbitmq,
    Redis,
    Spark,
    Traefik,
    Valkey,
    Vault,
}

impl OssIcon {
    pub const ALL: [OssIcon; 34] = [
        OssIcon::Airflow,
        OssIcon::Argo,
        OssIcon::Cassandra,
        OssIcon::Clickhouse,
        OssIcon::Docker,
        OssIcon::Envoy,
        OssIcon::Etcd,
        OssIcon::Grafana,
        OssIcon::Haproxy,
        OssIcon::Helm,
        OssIcon::Istio,
        OssIcon::Jaeger,
        OssIcon::Jenkins,
        OssIcon::Kafka,
        OssIcon::Keycloak,
        OssIcon::Kubernetes,
        OssIcon::Linkerd,
        OssIcon::Loki,
        OssIcon::Mariadb,
        OssIcon::Minio,
        OssIcon::Mongodb,
        OssIcon::Nats,
        OssIcon::Nginx,
        OssIcon::Opensearch,
        OssIcon::Opentelemetry,
        OssIcon::Postgresql,
        OssIcon::Prometheus,
        OssIcon::Pulsar,
        OssIcon::Rabbitmq,
        OssIcon::Redis,
        OssIcon::Spark,
        OssIcon::Traefik,
        OssIcon::Valkey,
        OssIcon::Vault,
    ];

    /// The icon's file name in `assets/icons/oss/`.
    pub fn file_name(self) -> &'static str {
        match self {
            OssIcon::Airflow => "airflow.svg",
            OssIcon::Argo => "argo.svg",
            OssIcon::Cassandra => "cassandra.svg",
            OssIcon::Clickhouse => "clickhouse.svg",
            OssIcon::Docker => "docker.svg",
            OssIcon::Envoy => "envoy.svg",
            OssIcon::Etcd => "etcd.svg",
            OssIcon::Grafana => "grafana.svg",
            OssIcon::Haproxy => "haproxy.svg",
            OssIcon::Helm => "helm.svg",
            OssIcon::Istio => "istio.svg",
            OssIcon::Jaeger => "jaeger.svg",
            OssIcon::Jenkins => "jenkins.svg",
            OssIcon::Kafka => "kafka.svg",
            OssIcon::Keycloak => "keycloak.svg",
            OssIcon::Kubernetes => "kubernetes.svg",
            OssIcon::Linkerd => "linkerd.svg",
            OssIcon::Loki => "loki.svg",
            OssIcon::Mariadb => "mariadb.svg",
            OssIcon::Minio => "minio.svg",
            OssIcon::Mongodb => "mongodb.svg",
            OssIcon::Nats => "nats.svg",
            OssIcon::Nginx => "nginx.svg",
            OssIcon::Opensearch => "opensearch.svg",
            OssIcon::Opentelemetry => "opentelemetry.svg",
            OssIcon::Postgresql => "postgresql.svg",
            OssIcon::Prometheus => "prometheus.svg",
            OssIcon::Pulsar => "pulsar.svg",
            OssIcon::Rabbitmq => "rabbitmq.svg",
            OssIcon::Redis => "redis.svg",
            OssIcon::Spark => "spark.svg",
            OssIcon::Traefik => "traefik.svg",
            OssIcon::Valkey => "valkey.svg",
            OssIcon::Vault => "vault.svg",
        }
    }

    /// The file stem, for example "postgresql".
    pub fn as_str(self) -> &'static str {
        let file_name = self.file_name();
        file_name.strip_suffix(".svg").unwrap_or(file_name)
    }

    /// The icon whose file stem is `stem`.
    pub fn from_stem(stem: &str) -> Option<OssIcon> {
        OssIcon::ALL.into_iter().find(|icon| icon.as_str() == stem)
    }
}
