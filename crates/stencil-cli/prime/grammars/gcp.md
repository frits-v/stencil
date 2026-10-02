# Grammar gcp

The Google Cloud vocabulary: the default grammar, so a page without "grammar" is a gcp page. Boxes are the frame, networks and places of a Google Cloud figure; every Item is a product with an optional icon from the bundled Google Cloud icons.

{{kinds}}

| Kind | Means |
|---|---|
| gcp | the Google Cloud frame: blue bar holding the label, light body |
| vpc | a VPC network: dashed gray border, no fill; under iso a ring |
| region | a region, tinted: tint 1 (default) blue, tint 2 pink |
| subnet | a subnet: dashed border, lavender |
| onprem | an on-prem site or another cloud, warm border; tint 1 pairs with region tint 1, tint 2 with tint 2 |
| project | a project or folder: pale yellow |
| optional | an optional or proposed component: dashed blue |
| k8s | a cluster or namespace: pink, no border; never tinted |
| perimeter | a VPC Service Controls perimeter: dashed orange, orange label |
| apis | Google APIs reached over Private Google Access, inside the gcp frame and outside every vpc |

Nesting inside Google Cloud: gcp, then vpc, then region, then subnet. project, perimeter and k8s wrap whatever they scope. onprem sites and other clouds sit outside gcp, usually in the producer column. A Box whose parent is not in its kind's list fails vet with kind-parent-not-allowed; Row, Col and Lanes are transparent.

Tint pairing: Metro 1 and Region A are tint 1, Metro 2 and Region B tint 2, end to end. A solid pipe of one slot sits beside or inside no tinted region or onprem of another slot; dash, gray and deny stay outside the rule, which is why a failover between Region A and Region B is a dash line. CUE enforces it (`stencil prime cue`).

Products that never sit inside a VPC: Cloud Storage, BigQuery, Pub/Sub, Artifact Registry and Cloud Logging are Google APIs reached over Private Google Access, never addresses in a VPC. Draw them in an apis Box beside the vpc inside the gcp frame. Serverless products (Cloud Run, Cloud Functions, App Engine, Vertex AI) run outside the VPC too, reached over Private Service Connect, an internal load balancer or Direct VPC egress: draw them beside the vpc, with a Private Service Connect endpoint inside it when the path matters. A Network Connectivity Center hub, Cloud CDN, Cloud DNS and Cloud Armor are global: outside every region Box. vet rejects all three placements.

remembered-constants rejects 64512, 130.211.0.0/22, 35.191.0.0/16 and 10.8.0.0/28 in every text field: doc examples, not requirements.

Lines: gray for internal calls, solid tint 1 and tint 2 for the two paired paths, dash for failover, control plane or identity, deny for a blocked path.

Rules no check enforces:

- Every product on a hop carries a fact read from the live doc at authoring time (source doc), or an explicit ask; never a remembered value. A built fact holds as-built names (bucket names, VLAN IDs, project ids) and needs no live doc.
- One audience per figure: canvas customer or internal.
- Official product names in subtitle: Cloud Run, Cloud SQL, Pub/Sub.
