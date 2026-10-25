//! The Google Cloud pack, `assets/icons/gcp/`.

use stencil_model::GcpIcon;

pub(super) fn svg_bytes(icon: GcpIcon) -> &'static [u8] {
    match icon {
        GcpIcon::Agents => include_bytes!("../../../../assets/icons/gcp/agents.svg"),
        GcpIcon::AiMl => include_bytes!("../../../../assets/icons/gcp/ai-ml.svg"),
        GcpIcon::Bigquery => include_bytes!("../../../../assets/icons/gcp/bigquery.svg"),
        GcpIcon::CloudRunFlat => include_bytes!("../../../../assets/icons/gcp/cloud-run-flat.svg"),
        GcpIcon::CloudRun => include_bytes!("../../../../assets/icons/gcp/cloud-run.svg"),
        GcpIcon::CloudSql => include_bytes!("../../../../assets/icons/gcp/cloud-sql.svg"),
        GcpIcon::CloudStorage => include_bytes!("../../../../assets/icons/gcp/cloud-storage.svg"),
        GcpIcon::ComputeEngine => include_bytes!("../../../../assets/icons/gcp/compute-engine.svg"),
        GcpIcon::Compute => include_bytes!("../../../../assets/icons/gcp/compute.svg"),
        GcpIcon::Containers => include_bytes!("../../../../assets/icons/gcp/containers.svg"),
        GcpIcon::DataAnalytics => include_bytes!("../../../../assets/icons/gcp/data-analytics.svg"),
        GcpIcon::Databases => include_bytes!("../../../../assets/icons/gcp/databases.svg"),
        GcpIcon::Devops => include_bytes!("../../../../assets/icons/gcp/devops.svg"),
        GcpIcon::Gke => include_bytes!("../../../../assets/icons/gcp/gke.svg"),
        GcpIcon::Hybrid => include_bytes!("../../../../assets/icons/gcp/hybrid.svg"),
        GcpIcon::Integration => include_bytes!("../../../../assets/icons/gcp/integration.svg"),
        GcpIcon::Networking => include_bytes!("../../../../assets/icons/gcp/networking.svg"),
        GcpIcon::Observability => include_bytes!("../../../../assets/icons/gcp/observability.svg"),
        GcpIcon::Scc => include_bytes!("../../../../assets/icons/gcp/scc.svg"),
        GcpIcon::SecurityIdentity => {
            include_bytes!("../../../../assets/icons/gcp/security-identity.svg")
        }
        GcpIcon::Serverless => include_bytes!("../../../../assets/icons/gcp/serverless.svg"),
        GcpIcon::Storage => include_bytes!("../../../../assets/icons/gcp/storage.svg"),
        GcpIcon::VertexAi => include_bytes!("../../../../assets/icons/gcp/vertex-ai.svg"),
    }
}
