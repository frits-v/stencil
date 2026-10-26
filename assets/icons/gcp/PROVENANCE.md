# Google Cloud icon pack provenance

Where each file under `assets/icons/gcp/` came from, so its bytes can be re-verified.

## Icons (`assets/icons/gcp/*.svg`)

Source: the Google Cloud product icon library published at
<https://cloud.google.com/icons>, specifically the two ZIP downloads that page
links, both downloaded 2026-09-02:

- Core product icons: <https://services.google.com/fh/files/misc/core-products-icons.zip>
- Product category icons: <https://services.google.com/fh/files/misc/category-icons.zip>

All 23 SVGs in `assets/icons/gcp/` are byte-identical (MD5) to a file inside one of
those two archives. Nothing was recolored, resized, cropped, or re-exported. The
local filenames are shortened (`gke.svg` rather than
`Unique Icons/GKE/SVG/GKE-512-color.svg`); file content is untouched.

Mapping, verified 2026-09-02:

| Local file | Archive path |
|---|---|
| `agents.svg` | `Category Icons/Agents/SVG/Agents-512-color.svg` |
| `ai-ml.svg` | `Category Icons/AI _ Machine Learning/SVG/AIMachineLearning-512-color.svg` |
| `bigquery.svg` | `Unique Icons/BigQuery/SVG/BigQuery-512-color.svg` |
| `cloud-run-flat.svg` | `Unique Icons/Cloud Run/SVG/CloudRun-512-color-rgb.svg` |
| `cloud-run.svg` | `Unique Icons/Cloud Run/SVG/CloudRun-512-color-rgb.svg` |
| `cloud-sql.svg` | `Unique Icons/Cloud SQL/SVG/CloudSQL-512-color.svg` |
| `cloud-storage.svg` | `Unique Icons/Cloud Storage/SVG/Cloud_Storage-512-color.svg` |
| `compute-engine.svg` | `Unique Icons/Compute Engine/SVG/ComputeEngine-512-color-rgb.svg` |
| `compute.svg` | `Category Icons/Compute/SVG/Compute-512-color.svg` |
| `containers.svg` | `Category Icons/Containers/SVG/Containers-512-color.svg` |
| `data-analytics.svg` | `Category Icons/Data Analytics/SVG/DataAnalytics-512-color.svg` |
| `databases.svg` | `Category Icons/Databases/SVG/Databases-512-color.svg` |
| `devops.svg` | `Category Icons/DevOps/SVG/DevOps-512-color.svg` |
| `gke.svg` | `Unique Icons/GKE/SVG/GKE-512-color.svg` |
| `hybrid.svg` | `Category Icons/Hybrid & Multicloud/SVG/HybridMulticloud-512-color.svg` |
| `integration.svg` | `Category Icons/Integration Services/SVG/IntegrationServices-512-color.svg` |
| `networking.svg` | `Category Icons/Networking/SVG/Networking-512-color-rgb.svg` |
| `observability.svg` | `Category Icons/Observability/SVG/Observability-512-color.svg` |
| `scc.svg` | `Unique Icons/Security Command Center/SVG/SecurityCommandCenter-512-color.svg` |
| `security-identity.svg` | `Category Icons/Security Identity/SVG/SecurityIdentity-512-color.svg` |
| `serverless.svg` | `Category Icons/Serverless Computing/SVG/ServerlessComputing-512-color.svg` |
| `storage.svg` | `Category Icons/Storage/SVG/Storage-512-color.svg` |
| `vertex-ai.svg` | `Unique Icons/Vertex AI/SVG/VertexAI-512-color.svg` |

Regenerate the check by unzipping both archives and comparing MD5 sums against
this directory.
