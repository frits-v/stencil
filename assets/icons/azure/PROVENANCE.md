# Azure icon pack provenance

Where the files under `assets/icons/azure/` came from and what may be done with them.

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

### Stated terms

Terms come from the icons page and from `Microsoft_Terms_of_Use.pdf` and
`Azure_Icons_FAQ.pdf` inside the ZIP. The page was fetched 2026-10-25 (page
last updated 2026-10-09); the two PDFs were read from the archive downloaded the
same day.

<https://learn.microsoft.com/en-us/azure/architecture/icons/>, on the purpose of the download:

> This page provides an official collection of Azure architecture icons including Azure product icons to help you build a custom architecture diagram for your next solution.

Same page, "Icon terms":

> Microsoft permits the use of these icons in architectural diagrams, training materials, or documentation. You can copy, distribute, and display the icons only for the permitted use unless granted explicit permission by Microsoft. Microsoft reserves all other rights.

Same page, "General guidelines":

> Use the icon to illustrate how products can work together.
>
> In diagrams, we recommend including the product name somewhere close to the icon.
>
> Use the icons as they would appear within Azure.
>
> Don't crop, flip, or rotate icons.
>
> Don't distort or change icon shape in any way.
>
> Don't use Microsoft product icons to represent your product or service.

`Microsoft_Terms_of_Use.pdf` in the ZIP repeats the same terms:

> Microsoft permits the use of these icons in architectural diagrams, training materials, or documentation. You may copy, distribute, and display the icons only for the permitted use unless granted explicit permission by Microsoft. Microsoft reserves all other rights.

`Azure_Icons_FAQ.pdf` in the ZIP, on modification and labeling:

> Icons may not be cropped, flipped or rotated, and their shape may not be distorted or changed.
>
> Yes, the full name of the service should always appear near the icon, but not so close that it looks like it's overlapping the icon.
>
> No, Azure icons may only be used to represent the Microsoft product that it was designed for.

The Entra package comes from <https://learn.microsoft.com/en-us/entra/architecture/architecture-icons>, fetched 2026-10-25 (page last updated 2023-10-23). Its "Icon terms" repeat the Azure wording:

> Microsoft permits the use of these icons in architectural diagrams, training materials, or documentation. You might copy, distribute, and display the icons only for the permitted use unless granted explicit permission by Microsoft. Microsoft reserves all other rights.

Its guidelines add one prohibition. The page and the package's `Microsoft Terms of Use.docx` both say:

> Don't use Microsoft product icons in Marketing communications.

### Conclusion

Microsoft publishes this icon set for use in architecture diagrams, training
materials and documentation, and asks that each icon be reproduced unaltered,
labeled with the service name and used only for the Microsoft service it was
designed for. The terms permit that use; they grant no separate licence to
redistribute the files outside it, the same posture as the gcp pack. Stencil
keeps the 41 SVGs unaltered, places the bytes of one in a data URI on the item
of the service it depicts, beside the item's name, and scales it with the
`<image>` element's width and height.

What the terms do not cover: altering an icon (crop, flip, rotate, distort,
recolor), using one to represent a product or service other than the Microsoft
service it was designed for, use beyond diagrams, training materials and
documentation, and, for the Entra icon, marketing communications.

The Azure archive has no Microsoft Entra ID service icon, so `entra-id.svg`
comes from the Entra archive, and it has no Blob Storage icon: a figure draws
Blob Storage with `storage-accounts.svg`.
