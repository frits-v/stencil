//! The 23 icons of section 8.2, embedded byte for byte. They are never parsed, rewritten
//! or minified; `<image>` scales them.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use stencil_model::IconName;

pub fn icon_svg_bytes(icon: IconName) -> &'static [u8] {
    match icon {
        IconName::Agents => include_bytes!("../../../assets/icons/agents.svg"),
        IconName::AiMl => include_bytes!("../../../assets/icons/ai-ml.svg"),
        IconName::Bigquery => include_bytes!("../../../assets/icons/bigquery.svg"),
        IconName::CloudRunFlat => include_bytes!("../../../assets/icons/cloud-run-flat.svg"),
        IconName::CloudRun => include_bytes!("../../../assets/icons/cloud-run.svg"),
        IconName::CloudSql => include_bytes!("../../../assets/icons/cloud-sql.svg"),
        IconName::CloudStorage => include_bytes!("../../../assets/icons/cloud-storage.svg"),
        IconName::ComputeEngine => include_bytes!("../../../assets/icons/compute-engine.svg"),
        IconName::Compute => include_bytes!("../../../assets/icons/compute.svg"),
        IconName::Containers => include_bytes!("../../../assets/icons/containers.svg"),
        IconName::DataAnalytics => include_bytes!("../../../assets/icons/data-analytics.svg"),
        IconName::Databases => include_bytes!("../../../assets/icons/databases.svg"),
        IconName::Devops => include_bytes!("../../../assets/icons/devops.svg"),
        IconName::Gke => include_bytes!("../../../assets/icons/gke.svg"),
        IconName::Hybrid => include_bytes!("../../../assets/icons/hybrid.svg"),
        IconName::Integration => include_bytes!("../../../assets/icons/integration.svg"),
        IconName::Networking => include_bytes!("../../../assets/icons/networking.svg"),
        IconName::Observability => include_bytes!("../../../assets/icons/observability.svg"),
        IconName::Scc => include_bytes!("../../../assets/icons/scc.svg"),
        IconName::SecurityIdentity => include_bytes!("../../../assets/icons/security-identity.svg"),
        IconName::Serverless => include_bytes!("../../../assets/icons/serverless.svg"),
        IconName::Storage => include_bytes!("../../../assets/icons/storage.svg"),
        IconName::VertexAi => include_bytes!("../../../assets/icons/vertex-ai.svg"),
    }
}

/// `data:image/svg+xml;base64,` followed by the base64 of the icon file's exact bytes.
pub fn icon_data_uri(icon: IconName) -> String {
    let mut uri = String::from("data:image/svg+xml;base64,");
    STANDARD.encode_string(icon_svg_bytes(icon), &mut uri);
    uri
}
