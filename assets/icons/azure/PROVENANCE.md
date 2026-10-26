# Azure icon pack provenance

Where each file under `assets/icons/azure/` came from, so its bytes can be re-verified.

## Icons (`assets/icons/azure/*.svg`)

Source: the Azure architecture icon library published at
<https://learn.microsoft.com/en-us/azure/architecture/icons/>, specifically the
ZIP download that page links:

- Azure Public Service Icons, V24: <https://arch-center.azureedge.net/icons/Azure_Public_Service_Icons_V24.zip>

A second ZIP supplies `entra-id.svg`, because the Azure archive has no Microsoft Entra ID icon:

- Microsoft Entra architecture icons, Oct 2023: <https://download.microsoft.com/download/3/1/a/31a56038-856a-4489-88e4-ee5a1c4352be/Microsoft%20Entra%20architecture%20icons%20-%20Oct%202023.zip>, linked from <https://learn.microsoft.com/en-us/entra/architecture/architecture-icons>, downloaded 2026-10-25, sha256 `4e07536706a2d092e6524e5417e2c861333fdbdb41c36f78d44dfa07ccc5eedc`.

The Azure archive, downloaded 2026-10-25, is dated 2026-07-08 and has sha256
`921594ccd1bf3d9c0a1bd7b6d924e050551a59342f2b353bb74bdcf761c35141`.

All 41 SVGs in `assets/icons/azure/` are byte-identical (sha256) to a file inside one of those
two archives. Nothing was recolored, resized, cropped or re-exported. The local
filenames are shortened (`aks.svg` rather than
`Icons/containers/10023-icon-service-Kubernetes-Services.svg`); file content is
untouched. The archive folders are service categories, but every file is a
service icon, so every row is class product. The archive offers one size per icon
(an 18 px viewBox), so there was no size to choose.

Mapping, verified 2026-10-25:

| Local file | Archive path |
|---|---|
| `aks.svg` | `Icons/containers/10023-icon-service-Kubernetes-Services.svg` |
| `api-management.svg` | `Icons/integration/10042-icon-service-API-Management-Services.svg` |
| `app-service.svg` | `Icons/app services/10035-icon-service-App-Services.svg` |
| `application-gateway.svg` | `Icons/networking/10076-icon-service-Application-Gateways.svg` |
| `azure-files.svg` | `Icons/storage/10400-icon-service-Azure-Fileshares.svg` |
| `azure-firewall.svg` | `Icons/networking/10084-icon-service-Firewalls.svg` |
| `azure-openai.svg` | `Icons/ai + machine learning/03438-icon-service-Azure-OpenAI.svg` |
| `bastion.svg` | `Icons/networking/02422-icon-service-Bastions.svg` |
| `cache-redis.svg` | `Icons/databases/10137-icon-service-Cache-Redis.svg` |
| `container-apps-environments.svg` | `Icons/other/02989-icon-service-Container-Apps-Environments.svg` |
| `cosmos-db.svg` | `Icons/databases/10121-icon-service-Azure-Cosmos-DB.svg` |
| `data-factory.svg` | `Icons/analytics/10126-icon-service-Data-Factories.svg` |
| `databricks.svg` | `Icons/analytics/10787-icon-service-Azure-Databricks.svg` |
| `dns.svg` | `Icons/networking/10064-icon-service-DNS-Zones.svg` |
| `entra-id.svg` | `Microsoft Entra architecture icons - Oct 2023/Microsoft Entra color icons SVG/Microsoft Entra ID color icon.svg` (Entra archive) |
| `event-grid-domains.svg` | `Icons/integration/10215-icon-service-Event-Grid-Domains.svg` |
| `event-grid-topics.svg` | `Icons/integration/10206-icon-service-Event-Grid-Topics.svg` |
| `event-hubs.svg` | `Icons/analytics/00039-icon-service-Event-Hubs.svg` |
| `expressroute.svg` | `Icons/networking/10079-icon-service-ExpressRoute-Circuits.svg` |
| `front-door.svg` | `Icons/networking/10073-icon-service-Front-Door-and-CDN-Profiles.svg` |
| `functions.svg` | `Icons/compute/10029-icon-service-Function-Apps.svg` |
| `key-vault.svg` | `Icons/security/10245-icon-service-Key-Vaults.svg` |
| `load-balancer.svg` | `Icons/networking/10062-icon-service-Load-Balancers.svg` |
| `log-analytics.svg` | `Icons/monitor/00009-icon-service-Log-Analytics-Workspaces.svg` |
| `logic-apps.svg` | `Icons/integration/02631-icon-service-Logic-Apps.svg` |
| `machine-learning.svg` | `Icons/ai + machine learning/038470510-icon-service-Azure-Machine-Learning.svg` |
| `monitor.svg` | `Icons/monitor/00001-icon-service-Monitor.svg` |
| `policy.svg` | `Icons/management + governance/10316-icon-service-Policy.svg` |
| `postgresql.svg` | `Icons/databases/10131-icon-service-Azure-Database-PostgreSQL-Server.svg` |
| `private-endpoint.svg` | `Icons/other/02579-icon-service-Private-Endpoints.svg` |
| `private-link.svg` | `Icons/networking/00427-icon-service-Private-Link.svg` |
| `service-bus.svg` | `Icons/integration/10836-icon-service-Azure-Service-Bus.svg` |
| `sql-database.svg` | `Icons/databases/10130-icon-service-SQL-Database.svg` |
| `sql-managed-instance.svg` | `Icons/databases/10136-icon-service-SQL-Managed-Instance.svg` |
| `storage-accounts.svg` | `Icons/storage/10086-icon-service-Storage-Accounts.svg` |
| `synapse.svg` | `Icons/analytics/00606-icon-service-Azure-Synapse-Analytics.svg` |
| `traffic-manager.svg` | `Icons/networking/10065-icon-service-Traffic-Manager-Profiles.svg` |
| `virtual-machines.svg` | `Icons/compute/10021-icon-service-Virtual-Machine.svg` |
| `virtual-network.svg` | `Icons/networking/10061-icon-service-Virtual-Networks.svg` |
| `vm-scale-sets.svg` | `Icons/compute/10034-icon-service-VM-Scale-Sets.svg` |
| `vpn-gateway.svg` | `Icons/networking/10063-icon-service-Virtual-Network-Gateways.svg` |

Several icons appear in more than one category folder with identical content; the
path listed is the most specific folder.

Azure archive paths are relative to the archive's top folder,
`Azure_Public_Service_Icons/`. Regenerate the check by unzipping both archives and
comparing sha256 sums of each file against this directory:

```
shasum -a 256 assets/icons/azure/*.svg
shasum -a 256 "Azure_Public_Service_Icons/<archive path>"
```
