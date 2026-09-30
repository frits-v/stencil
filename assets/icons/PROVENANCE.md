# Stencil provenance

Where the files under `stencil/` came from and what may be done with them.

## Icons (`stencil/icons/*.svg`)

Source: the Google Cloud product icon library published at
<https://cloud.google.com/icons>, specifically the two ZIP downloads that page
links:

- Core product icons: <https://services.google.com/fh/files/misc/core-products-icons.zip>
- Product category icons: <https://services.google.com/fh/files/misc/category-icons.zip>

All 23 SVGs in `stencil/icons/` are byte-identical (MD5) to a file inside one of
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

### Stated terms

Neither ZIP contains a LICENSE, README, or terms file. The overview PDF the
icons page links, `google-cloud-product-icons.pdf` (Updated: May 2026), stamps
"Google Cloud Proprietary & Confidential" on every page and grants nothing. The
terms therefore come from three live Google pages, all fetched 2026-09-02.

<https://cloud.google.com/icons>, on the purpose of the download:

> Welcome to the official library for Google Cloud product icons. Here you can
> find the Google Cloud product icons you need for your diagrams, technical
> documentation, and more.

<https://about.google/brand-resource-center/brand-elements/>, section "Product
icons":

> We use product icons to represent specific Google products. You might see them
> when you open an app on your phone, or when using a Google product. To use a
> Google product icon in your work, create a Partner Marketing Hub account to
> find assets and request permission through our approval form. We're in the
> process of redesigning our product icons, so be sure to use the most up-to-date
> versions.
>
> DO Make sure your brand is more prominent than Google's product icon.
>
> DON'T Don't incorporate a Google product icon out of context, without the
> proper information for your audience to understand what it means.

<https://about.google/brand-resource-center/guidance/>, on unaltered use:

> Use our names, visuals, logos, or icons in teaching materials. You can use
> unaltered static screenshots or video captures of our products, the Google
> logo, or specific product icons for educational purposes in textbooks or other
> standard teaching materials.

<https://www.google.com/permissions/trademark/rules/>:

> DON'T Don't remove, distort, or alter any element of a Google Brand Feature.
> You may not modify Google Brand Features by hyphenation or combination, or by
> shortening, abbreviating, or creating acronyms from Google Brand Features.

The same page also forbids imitating the surrounding trade dress:

> DON'T Don't copy or imitate Google's trade dress, including the look and feel
> of Google web design properties or Google brand packaging, distinctive color
> combinations, typography, graphic designs, product icons, or imagery
> associated with Google.

### Conclusion

Google publishes this icon set for use in architecture diagrams and technical
documentation and asks that the icons be reproduced unaltered, in context, and
without letting Google's mark outweigh the author's own. Vendoring the 23 SVGs
byte-for-byte and placing them on labeled product cards is inside that grant.

What the grant does not cover: altering an icon (recolor, crop, restyle, combine
with another mark), putting one on merchandise, using one on a surface where it
reads as a Google endorsement, or copying the wider Google trade dress. The
Brand Resource Center also routes product-icon use "in your work" through a
Partner Marketing Hub approval request, so a marketing or co-branded surface, as
opposed to an architecture figure inside a deliverable, needs that request filed
before it ships.

Re-check these pages before any use outside architecture and technical figures.
The icon system was reworked in early 2025 and Google says it is still
redesigning icons, so the set turns over.

## CSS (`stencil/_gcp.css`)

Layout and pipe grammar written for the DataPiper kit's diagrams skill. No
third-party code and no license file: the kit ships exactly one LICENSE, an MIT
notice scoped to a separately vendored `stop-slop` skill, which does not cover
this file. Treat it as internal work product.

The stylesheet carries no Google brand assets. It names no Google font, reuses
no Google logo, and its colors are the ordinary Material greys and blues rather
than a lift of a Google page's trade dress. Two edits on vendoring:

- The Google Fonts `@import` for Inter was removed. The renderer runs with DNS
  blackholed, and a webfont request that never resolves leaves the page
  mid-layout when the screenshot fires.
- `Google Sans` was dropped from the `body` font stack, since Google's brand
  guidance reserves its brand fonts. Type now resolves through
  `"Helvetica Neue", Helvetica, Arial, system-ui, sans-serif`.

## Gold prototypes (`stencil/golds/*.html`)

Same origin and same status as the CSS. Two changes on vendoring: stylesheet and
icon `src` paths were rewritten one level up for the `stencil/golds/` layout,
and every hard-coded Google constant on a hop (probe ranges, VIP ranges, BGP
timers) was replaced with an instruction to read the value from the live product
doc. A gold that ships a remembered range teaches the exact habit the hop-fact
rule exists to stop, and `scripts/constants_lint.py` fires on it.
