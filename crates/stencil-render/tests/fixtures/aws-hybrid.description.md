# The data center reaches a two-zone VPC over Direct Connect; S3, DynamoDB and the secrets stay outside it `/title`

Customer · AWS Direct Connect · one region, two Availability Zones `/kicker`

The orders database runs in both zones behind private subnets. Amazon S3, Amazon DynamoDB and AWS Secrets Manager are regional services the VPC reaches over endpoints, never addresses inside it. `/lede`

## Figure

Drawn elements: 39 (39 nodes, 0 links). Canvas 1480 x 857.8 px, projection flat, grammar aws. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Row, left to right. `/body/0`
  - Box Data center; kind onprem, tint 1. `/body/0/children/0`
    - Item Customer router: port toward AWS; kind product, icon networking-content-delivery. `/body/0/children/0/children/0`
    - Item Core firewall: inspects traffic toward AWS; kind product, icon networking-content-delivery. `/body/0/children/0/children/1`
    - Item Operators: run the release; kind person. `/body/0/children/0/children/2`
    - Item Order archive: nightly export source; kind product, icon storage. `/body/0/children/0/children/3`
    - Item Legacy orders database: moves to the cloud database; kind product, icon database. `/body/0/children/0/children/4`
    - Item Order desk: clients of the orders API; kind device. `/body/0/children/0/children/5`
    - Fact, doc: Prefixes advertised from the IP plan. `/body/0/children/0/children/6`
  - Pipe Direct Connect · private VIF · BGP.
  - Box AWS Cloud; kind aws. `/body/0/children/2`
    - Item Direct Connect gateway: AWS Direct Connect; kind product, icon direct-connect, product AWS Direct Connect. `/body/0/children/2/children/0`
    - Box Region; kind region, tint 1. `/body/0/children/2/children/1`
      - Fact, doc: Region and zones from the landing-zone design. `/body/0/children/2/children/1/children/0`
      - Row, left to right. `/body/0/children/2/children/1/children/1`
        - Box Application VPC; kind vpc. `/body/0/children/2/children/1/children/1/children/0`
          - Row, left to right. `/body/0/children/2/children/1/children/1/children/0/children/0`
            - Item Virtual private gateway: Amazon-side ASN from the gateway configuration; kind product, icon vpc. `/body/0/children/2/children/1/children/1/children/0/children/0/children/0`
            - Item S3 gateway endpoint: gateway endpoint · Amazon S3; kind product, icon vpc, product gateway endpoint. `/body/0/children/2/children/1/children/1/children/0/children/0/children/1`
              - Fact, doc: on the private route tables.
          - Row, left to right. `/body/0/children/2/children/1/children/1/children/0/children/1`
            - Box Availability Zone A; kind az. `/body/0/children/2/children/1/children/1/children/0/children/1/children/0`
              - Box Private subnet A; kind subnet. `/body/0/children/2/children/1/children/1/children/0/children/1/children/0/children/0`
                - Item Orders database: Amazon RDS for PostgreSQL · primary; kind product, icon rds, product Amazon RDS. `/body/0/children/2/children/1/children/1/children/0/children/1/children/0/children/0/children/0`
                - Item Order worker: AWS Lambda · VPC-attached; kind product, icon lambda, product AWS Lambda. `/body/0/children/2/children/1/children/1/children/0/children/1/children/0/children/0/children/1`
                - Item Secrets endpoint: interface endpoint; kind product, icon privatelink, product interface endpoint. `/body/0/children/2/children/1/children/1/children/0/children/1/children/0/children/0/children/2`
                  - Fact, doc: for AWS Secrets Manager.
            - Box Availability Zone B; kind az. `/body/0/children/2/children/1/children/1/children/0/children/1/children/1`
              - Box Private subnet B; kind subnet. `/body/0/children/2/children/1/children/1/children/0/children/1/children/1/children/0`
                - Item Orders standby: Amazon RDS for PostgreSQL · Multi-AZ standby; kind product, icon rds, product Amazon RDS. `/body/0/children/2/children/1/children/1/children/0/children/1/children/1/children/0/children/0`
                - Item Inbound resolver: Route 53 Resolver inbound endpoint; kind product, icon route-53, product Route 53. `/body/0/children/2/children/1/children/1/children/0/children/1/children/1/children/0/children/1`
                - Fact, doc: No route to an internet gateway. `/body/0/children/2/children/1/children/1/children/0/children/1/children/1/children/0/children/2`
        - Pipe Endpoints · gateway and interface.
        - Box Regional services; kind services. `/body/0/children/2/children/1/children/1/children/2`
          - Item Order exports: Amazon S3; kind product, icon s3, product Amazon S3. `/body/0/children/2/children/1/children/1/children/2/children/0`
          - Item Session table: Amazon DynamoDB; kind product, icon dynamodb, product Amazon DynamoDB. `/body/0/children/2/children/1/children/1/children/2/children/1`
          - Item Database credentials: AWS Secrets Manager; kind product, icon secrets-manager, product AWS Secrets Manager. `/body/0/children/2/children/1/children/1/children/2/children/2`
          - Item Order queue: Amazon SQS; kind product, icon sqs, product Amazon SQS. `/body/0/children/2/children/1/children/1/children/2/children/3`
          - Item Database key: AWS Key Management Service; kind product, icon kms, product AWS Key Management Service. `/body/0/children/2/children/1/children/1/children/2/children/4`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| Direct Connect · private VIF · BGP | pipe h | Customer router (customer-router) | Direct Connect gateway (dx-gateway) | solid, tint 1; blue in center | no arrowheads | Data center ↔ Region | `/body/0/children/1` |
| Endpoints · gateway and interface | pipe h | Application VPC (vpc) | Regional services (services) | gray; gray in center | no arrowheads | VPC endpoints to regional services | `/body/0/children/2/children/1/children/1/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| blue | Solid blue | Data center ↔ Region | 1 | `/legend/0` |
| gray | Solid gray | VPC endpoints to regional services | 1 | `/legend/1` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
