//! The Azure pack, `assets/icons/azure/`.

use stencil_model::AzureIcon;

pub(super) fn svg_bytes(icon: AzureIcon) -> &'static [u8] {
    match icon {
        AzureIcon::Aks => include_bytes!("../../../../assets/icons/azure/aks.svg"),
        AzureIcon::ApiManagement => {
            include_bytes!("../../../../assets/icons/azure/api-management.svg")
        }
        AzureIcon::AppService => include_bytes!("../../../../assets/icons/azure/app-service.svg"),
        AzureIcon::ApplicationGateway => {
            include_bytes!("../../../../assets/icons/azure/application-gateway.svg")
        }
        AzureIcon::AzureFiles => include_bytes!("../../../../assets/icons/azure/azure-files.svg"),
        AzureIcon::AzureFirewall => {
            include_bytes!("../../../../assets/icons/azure/azure-firewall.svg")
        }
        AzureIcon::AzureOpenai => include_bytes!("../../../../assets/icons/azure/azure-openai.svg"),
        AzureIcon::Bastion => include_bytes!("../../../../assets/icons/azure/bastion.svg"),
        AzureIcon::CacheRedis => include_bytes!("../../../../assets/icons/azure/cache-redis.svg"),
        AzureIcon::ContainerAppsEnvironments => {
            include_bytes!("../../../../assets/icons/azure/container-apps-environments.svg")
        }
        AzureIcon::CosmosDb => include_bytes!("../../../../assets/icons/azure/cosmos-db.svg"),
        AzureIcon::DataFactory => include_bytes!("../../../../assets/icons/azure/data-factory.svg"),
        AzureIcon::Databricks => include_bytes!("../../../../assets/icons/azure/databricks.svg"),
        AzureIcon::Dns => include_bytes!("../../../../assets/icons/azure/dns.svg"),
        AzureIcon::EntraId => include_bytes!("../../../../assets/icons/azure/entra-id.svg"),
        AzureIcon::EventGridDomains => {
            include_bytes!("../../../../assets/icons/azure/event-grid-domains.svg")
        }
        AzureIcon::EventGridTopics => {
            include_bytes!("../../../../assets/icons/azure/event-grid-topics.svg")
        }
        AzureIcon::EventHubs => include_bytes!("../../../../assets/icons/azure/event-hubs.svg"),
        AzureIcon::Expressroute => {
            include_bytes!("../../../../assets/icons/azure/expressroute.svg")
        }
        AzureIcon::FrontDoor => include_bytes!("../../../../assets/icons/azure/front-door.svg"),
        AzureIcon::Functions => include_bytes!("../../../../assets/icons/azure/functions.svg"),
        AzureIcon::KeyVault => include_bytes!("../../../../assets/icons/azure/key-vault.svg"),
        AzureIcon::LoadBalancer => {
            include_bytes!("../../../../assets/icons/azure/load-balancer.svg")
        }
        AzureIcon::LogAnalytics => {
            include_bytes!("../../../../assets/icons/azure/log-analytics.svg")
        }
        AzureIcon::LogicApps => include_bytes!("../../../../assets/icons/azure/logic-apps.svg"),
        AzureIcon::MachineLearning => {
            include_bytes!("../../../../assets/icons/azure/machine-learning.svg")
        }
        AzureIcon::Monitor => include_bytes!("../../../../assets/icons/azure/monitor.svg"),
        AzureIcon::Policy => include_bytes!("../../../../assets/icons/azure/policy.svg"),
        AzureIcon::Postgresql => include_bytes!("../../../../assets/icons/azure/postgresql.svg"),
        AzureIcon::PrivateEndpoint => {
            include_bytes!("../../../../assets/icons/azure/private-endpoint.svg")
        }
        AzureIcon::PrivateLink => include_bytes!("../../../../assets/icons/azure/private-link.svg"),
        AzureIcon::ServiceBus => include_bytes!("../../../../assets/icons/azure/service-bus.svg"),
        AzureIcon::SqlDatabase => include_bytes!("../../../../assets/icons/azure/sql-database.svg"),
        AzureIcon::SqlManagedInstance => {
            include_bytes!("../../../../assets/icons/azure/sql-managed-instance.svg")
        }
        AzureIcon::StorageAccounts => {
            include_bytes!("../../../../assets/icons/azure/storage-accounts.svg")
        }
        AzureIcon::Synapse => include_bytes!("../../../../assets/icons/azure/synapse.svg"),
        AzureIcon::TrafficManager => {
            include_bytes!("../../../../assets/icons/azure/traffic-manager.svg")
        }
        AzureIcon::VirtualMachines => {
            include_bytes!("../../../../assets/icons/azure/virtual-machines.svg")
        }
        AzureIcon::VirtualNetwork => {
            include_bytes!("../../../../assets/icons/azure/virtual-network.svg")
        }
        AzureIcon::VmScaleSets => {
            include_bytes!("../../../../assets/icons/azure/vm-scale-sets.svg")
        }
        AzureIcon::VpnGateway => include_bytes!("../../../../assets/icons/azure/vpn-gateway.svg"),
    }
}
