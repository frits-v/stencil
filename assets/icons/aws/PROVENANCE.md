# AWS icon pack provenance

Where the files in `assets/icons/aws/` came from and what may be done with them.

## Icons (`assets/icons/aws/*.svg`)

Source: the AWS Architecture Icons asset package, release 24-2026.07.31 (folder
suffix 07312026), published at <https://aws.amazon.com/architecture/icons/>,
specifically the "Icon package" ZIP that page links:

- <https://d1.awsstatic.com/onedam/marketing-channels/website/public/shared/architecture-icon-release/Icon-package_07312026.5846e92413caa21490223536cc97f1269e44fa92.zip>
  (SHA-256 `d2d166c453526471749d520e0db022c459abef759d2946cf2dd1d1c992dc6526`)

All 46 SVGs in `assets/icons/aws/` are byte-identical (SHA-256) to a file inside
that archive. Nothing was recolored, resized, cropped, or re-exported. The files
are the 64 px SVGs (`Arch_<Category>/64/` and `Arch-Category_64/`), the largest
size the package ships; each is an 80 by 80 viewBox with the white glyph on a
square in its category color. The `Architecture-Service-Icons` folder maps to
class product (36 icons) and the `Category-Icons` folder to class category (10
icons). The local filenames are shortened (`ec2.svg` rather than
`Architecture-Service-Icons_07312026/Arch_Compute/64/Arch_Amazon-EC2_64.svg`);
file content is untouched.

Four local names differ from the package's own: the package calls the
categories stored as `machine-learning.svg`, `management-governance.svg` and
`security-identity-compliance.svg` Artificial-Intelligence, Management-Tools and
Security-Identity, and `sagemaker.svg` is the Amazon SageMaker AI icon from
`Arch_Artificial-Intelligence`, not the separate Amazon SageMaker icon in
`Arch_Analytics`.

Mapping, verified 2026-10-25:

| Local file | Archive path |
|---|---|
| `analytics.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Analytics_64.svg` |
| `api-gateway.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_Amazon-API-Gateway_64.svg` |
| `application-integration.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Application-Integration_64.svg` |
| `athena.svg` | `Architecture-Service-Icons_07312026/Arch_Analytics/64/Arch_Amazon-Athena_64.svg` |
| `aurora.svg` | `Architecture-Service-Icons_07312026/Arch_Databases/64/Arch_Amazon-Aurora_64.svg` |
| `bedrock.svg` | `Architecture-Service-Icons_07312026/Arch_Artificial-Intelligence/64/Arch_Amazon-Bedrock_64.svg` |
| `cloudfront.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_Amazon-CloudFront_64.svg` |
| `cloudwatch.svg` | `Architecture-Service-Icons_07312026/Arch_Management-Tools/64/Arch_Amazon-CloudWatch_64.svg` |
| `compute.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Compute_64.svg` |
| `containers.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Containers_64.svg` |
| `database.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Databases_64.svg` |
| `direct-connect.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_AWS-Direct-Connect_64.svg` |
| `dynamodb.svg` | `Architecture-Service-Icons_07312026/Arch_Databases/64/Arch_Amazon-DynamoDB_64.svg` |
| `ebs.svg` | `Architecture-Service-Icons_07312026/Arch_Storage/64/Arch_Amazon-Elastic-Block-Store_64.svg` |
| `ec2.svg` | `Architecture-Service-Icons_07312026/Arch_Compute/64/Arch_Amazon-EC2_64.svg` |
| `ecr.svg` | `Architecture-Service-Icons_07312026/Arch_Containers/64/Arch_Amazon-Elastic-Container-Registry_64.svg` |
| `ecs.svg` | `Architecture-Service-Icons_07312026/Arch_Containers/64/Arch_Amazon-Elastic-Container-Service_64.svg` |
| `efs.svg` | `Architecture-Service-Icons_07312026/Arch_Storage/64/Arch_Amazon-EFS_64.svg` |
| `eks.svg` | `Architecture-Service-Icons_07312026/Arch_Containers/64/Arch_Amazon-Elastic-Kubernetes-Service_64.svg` |
| `elasticache.svg` | `Architecture-Service-Icons_07312026/Arch_Databases/64/Arch_Amazon-ElastiCache_64.svg` |
| `elb.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_Elastic-Load-Balancing_64.svg` |
| `eventbridge.svg` | `Architecture-Service-Icons_07312026/Arch_Application-Integration/64/Arch_Amazon-EventBridge_64.svg` |
| `fargate.svg` | `Architecture-Service-Icons_07312026/Arch_Containers/64/Arch_AWS-Fargate_64.svg` |
| `glue.svg` | `Architecture-Service-Icons_07312026/Arch_Analytics/64/Arch_AWS-Glue_64.svg` |
| `iam.svg` | `Architecture-Service-Icons_07312026/Arch_Security-Identity/64/Arch_AWS-Identity-and-Access-Management_64.svg` |
| `kinesis.svg` | `Architecture-Service-Icons_07312026/Arch_Analytics/64/Arch_Amazon-Kinesis_64.svg` |
| `kms.svg` | `Architecture-Service-Icons_07312026/Arch_Security-Identity/64/Arch_AWS-Key-Management-Service_64.svg` |
| `lambda.svg` | `Architecture-Service-Icons_07312026/Arch_Compute/64/Arch_AWS-Lambda_64.svg` |
| `machine-learning.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Artificial-Intelligence_64.svg` |
| `management-governance.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Management-Tools_64.svg` |
| `networking-content-delivery.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Networking-Content-Delivery_64.svg` |
| `privatelink.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_AWS-PrivateLink_64.svg` |
| `rds.svg` | `Architecture-Service-Icons_07312026/Arch_Databases/64/Arch_Amazon-RDS_64.svg` |
| `redshift.svg` | `Architecture-Service-Icons_07312026/Arch_Analytics/64/Arch_Amazon-Redshift_64.svg` |
| `route-53.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_Amazon-Route-53_64.svg` |
| `s3.svg` | `Architecture-Service-Icons_07312026/Arch_Storage/64/Arch_Amazon-Simple-Storage-Service_64.svg` |
| `sagemaker.svg` | `Architecture-Service-Icons_07312026/Arch_Artificial-Intelligence/64/Arch_Amazon-SageMaker-AI_64.svg` |
| `secrets-manager.svg` | `Architecture-Service-Icons_07312026/Arch_Security-Identity/64/Arch_AWS-Secrets-Manager_64.svg` |
| `security-identity-compliance.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Security-Identity_64.svg` |
| `sns.svg` | `Architecture-Service-Icons_07312026/Arch_Application-Integration/64/Arch_Amazon-Simple-Notification-Service_64.svg` |
| `sqs.svg` | `Architecture-Service-Icons_07312026/Arch_Application-Integration/64/Arch_Amazon-Simple-Queue-Service_64.svg` |
| `step-functions.svg` | `Architecture-Service-Icons_07312026/Arch_Application-Integration/64/Arch_AWS-Step-Functions_64.svg` |
| `storage.svg` | `Category-Icons_07312026/Arch-Category_64/Arch-Category_Storage_64.svg` |
| `transit-gateway.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_AWS-Transit-Gateway_64.svg` |
| `vpc.svg` | `Architecture-Service-Icons_07312026/Arch_Networking-Content-Delivery/64/Arch_Amazon-Virtual-Private-Cloud_64.svg` |
| `waf.svg` | `Architecture-Service-Icons_07312026/Arch_Security-Identity/64/Arch_AWS-WAF_64.svg` |

Regenerate the check by unzipping the archive and comparing SHA-256 sums against
this directory.

### Stated terms

The ZIP contains no LICENSE, README, or terms file. The usage guidance is in the
companion PowerPoint toolkit the icons page links,
`Microsoft-PPTx-toolkits_07312026.zip`, whose deck
`AWS-Architecture-Icons-Deck_For-Light-BG_07312026.pptx` (Release
24-2026.07.31) was read as slide text. Both sources were fetched 2026-10-25.

<https://aws.amazon.com/architecture/icons/>, on the purpose of the download:

> Architecture diagrams are a great way to communicate your design, deployment,
> and topology. To help you build diagrams, this page has Amazon Web Services
> (AWS) product icons, resources, and tools you can use. We allow customers and
> partners to use these toolkits and assets to create architecture diagrams.
> AWS architecture icons are designed to be simple, so you can easily use them
> in diagrams. You can also put icons in materials like whitepapers,
> presentations, data sheets, and posters.

The icons deck, slide 5 ("About AWS Architecture Icons"):

> Customers and partners can use AWS icons to create architecture diagrams. The
> icons are basic in design so that you can incorporate them directly into your
> whitepapers, presentations, datasheets, posters, or any technical material
> that you want.

The icons deck, slide 15 ("Icons"), on unaltered use:

> DO: Use icons at their predefined size, color and format in diagrams. Scale
> icons as needed for use in presentations. Hold down Shift while resizing.
> DON'T: Crop service icons. Flip or rotate icons. Change icon shapes.

The icons deck, slide 13, for training and certification diagrams:

> All icon sizes are fixed and should not be altered. All colors for icons have
> been accessibility tested and should not be altered.

The icons deck, slide 17, on naming:

> AWS or Amazon should always be accompanied by the service name.
>
> DON'T: Use short forms without first mentioning the full service name
> somewhere in the document.

### Conclusion

The terms let customers and partners use the icons to create architecture
diagrams and to place them in whitepapers, presentations, data sheets, posters
and other technical material, with each icon kept at its color and shape,
uncropped, unflipped and unrotated, and scaled only in proportion. Stencil keeps
the 46 files unaltered, places each one's original bytes in a data URI and
scales it with the `<image>` element's equal width and height, on a product item
whose subtitle names the service. The aws grammar's products table lists each
service's full name before its short form, as slide 17 asks of a document.

The terms grant use in diagrams and give no separate licence to redistribute the
files on their own. The files are kept in this repository as the pack the aws
grammar draws from, the same footing as the gcp pack. Re-check the icons page
and the deck before any use outside architecture and technical figures; the
package is reissued each quarter and icons are renamed between releases.
