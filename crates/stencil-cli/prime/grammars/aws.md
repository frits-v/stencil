# Grammar aws

The AWS vocabulary: set "grammar": "aws" on the page. A design sheet under aws: take `stencil prime onepager`, a plain sheet, set "grammar": "aws", make the system Box an aws Box and each item a product with its icon; vet names each one missed as kind-unknown.

{{kinds}}

{{icons}}

| Kind | Means |
|---|---|
| aws | the AWS Cloud frame |
| account | an AWS account or organizational unit: pale yellow |
| region | a region, tinted: tint 1 (default) blue, tint 2 pink |
| vpc | a VPC: dashed gray border, no fill; under iso a ring |
| az | an availability zone: lavender |
| subnet | a subnet: dashed border, lavender |
| onprem | an on-prem site or another cloud, warm border; tint 1 pairs with region tint 1 |
| optional | an optional or proposed component: dashed blue |
| k8s | an EKS cluster or namespace: pink, no border |
| services | regional services reached from the VPC over endpoints, beside every vpc |
| assembly | the parts one actor runs, on a design sheet only |

Nesting inside AWS: aws, then account, then region, then vpc, then az, then subnet. A VPC is regional; a subnet sits in one az and an az in one region. A Box whose parent is not in its kind's list fails vet with kind-parent-not-allowed; Row, Col and Lanes are transparent.

Tint pairing: an on-prem site and the region it reaches share a slot, end to end. A solid pipe of one slot sits beside or inside no tinted region or onprem of another slot; dash, gray and deny stay outside the rule. CUE enforces it (`stencil prime cue`).

Services that never sit inside a VPC: S3, DynamoDB, SQS, SNS, EventBridge, ECR, Secrets Manager, KMS, Step Functions, Kinesis, Athena, Bedrock, CloudWatch and API Gateway are regional services, not addresses in a VPC. Draw them in a services Box beside the vpc, with the gateway or interface endpoint inside it. A product whose subtitle names an endpoint is the endpoint. A VPC-attached Lambda function, RDS, Aurora, ElastiCache, Redshift, OpenSearch and MSK sit in subnets. IAM, Route 53, CloudFront, WAF, Organizations and Global Accelerator are global: outside every region Box. A Route 53 Resolver endpoint, IAM Identity Center or a WAF whose subtitle says regional is regional. vet rejects both placements.

remembered-constants rejects 64512, 10.0.0.0/16, 10.0.1.0/24 and 169.254.169.254 in every text field: a settable default, doc examples and a platform fact, not design values.

Lines: gray for internal calls, solid tint 1 and tint 2 for the two paired paths, dash for failover, control plane or identity, deny for a blocked path.

Rules no check enforces:

- Every product on a hop carries a fact read from the live doc at authoring time (source doc), or an explicit ask; never a remembered value. A built fact holds as-built names (bucket names, account ids, VLAN IDs) and needs no live doc.
- The full service name before any short form: Amazon S3, then S3; AWS Lambda, then Lambda.
- An endpoint's subtitle names the endpoint (gateway endpoint, interface endpoint) and a fact names the service behind it.
