//! The Google Cloud pack, `assets/icons/gcp/`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GcpIcon {
    Agents,
    AiMl,
    Bigquery,
    CloudRunFlat,
    CloudRun,
    CloudSql,
    CloudStorage,
    ComputeEngine,
    Compute,
    Containers,
    DataAnalytics,
    Databases,
    Devops,
    Gke,
    Hybrid,
    Integration,
    Networking,
    Observability,
    Scc,
    SecurityIdentity,
    Serverless,
    Storage,
    VertexAi,
}

impl GcpIcon {
    pub const ALL: [GcpIcon; 23] = [
        GcpIcon::Agents,
        GcpIcon::AiMl,
        GcpIcon::Bigquery,
        GcpIcon::CloudRunFlat,
        GcpIcon::CloudRun,
        GcpIcon::CloudSql,
        GcpIcon::CloudStorage,
        GcpIcon::ComputeEngine,
        GcpIcon::Compute,
        GcpIcon::Containers,
        GcpIcon::DataAnalytics,
        GcpIcon::Databases,
        GcpIcon::Devops,
        GcpIcon::Gke,
        GcpIcon::Hybrid,
        GcpIcon::Integration,
        GcpIcon::Networking,
        GcpIcon::Observability,
        GcpIcon::Scc,
        GcpIcon::SecurityIdentity,
        GcpIcon::Serverless,
        GcpIcon::Storage,
        GcpIcon::VertexAi,
    ];

    /// The icon's file name in `assets/icons/gcp/`.
    pub fn file_name(self) -> &'static str {
        match self {
            GcpIcon::Agents => "agents.svg",
            GcpIcon::AiMl => "ai-ml.svg",
            GcpIcon::Bigquery => "bigquery.svg",
            GcpIcon::CloudRunFlat => "cloud-run-flat.svg",
            GcpIcon::CloudRun => "cloud-run.svg",
            GcpIcon::CloudSql => "cloud-sql.svg",
            GcpIcon::CloudStorage => "cloud-storage.svg",
            GcpIcon::ComputeEngine => "compute-engine.svg",
            GcpIcon::Compute => "compute.svg",
            GcpIcon::Containers => "containers.svg",
            GcpIcon::DataAnalytics => "data-analytics.svg",
            GcpIcon::Databases => "databases.svg",
            GcpIcon::Devops => "devops.svg",
            GcpIcon::Gke => "gke.svg",
            GcpIcon::Hybrid => "hybrid.svg",
            GcpIcon::Integration => "integration.svg",
            GcpIcon::Networking => "networking.svg",
            GcpIcon::Observability => "observability.svg",
            GcpIcon::Scc => "scc.svg",
            GcpIcon::SecurityIdentity => "security-identity.svg",
            GcpIcon::Serverless => "serverless.svg",
            GcpIcon::Storage => "storage.svg",
            GcpIcon::VertexAi => "vertex-ai.svg",
        }
    }

    /// The file stem, for example "cloud-run".
    pub fn as_str(self) -> &'static str {
        let file_name = self.file_name();
        file_name.strip_suffix(".svg").unwrap_or(file_name)
    }

    /// The icon whose file stem is `stem`.
    pub fn from_stem(stem: &str) -> Option<GcpIcon> {
        GcpIcon::ALL.into_iter().find(|icon| icon.as_str() == stem)
    }
}
