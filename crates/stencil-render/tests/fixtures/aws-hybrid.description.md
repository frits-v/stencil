# The order API runs in two zones and reaches menus, carts and keys over endpoints `/title`

Customer · Food delivery ordering backend · one region, two Availability Zones `/kicker`

Amazon CloudFront takes orders from the apps and sits outside every region. Behind it, the order API runs as AWS Fargate tasks in both zones and writes orders to an Amazon RDS primary with a standby in the second zone. Menus in Amazon S3, carts in Amazon DynamoDB and the API's keys in AWS Secrets Manager are regional services the VPC reaches over endpoints, never addresses inside it. `/lede`

## Figure

Drawn elements: 31 (31 nodes, 0 links). Canvas 1480 x 844 px, projection flat, grammar aws. Line styles name a tint slot by number; the colour word is the one the center theme draws it in.

## Drawing `/body`

- Box AWS Cloud; kind aws. `/body/0`
  - Row, left to right. `/body/0/children/0`
    - Item Public zone: Amazon Route 53; kind product, icon route-53, product Amazon Route 53. `/body/0/children/0/children/0`
      - Fact, doc: alias to the distribution.
    - Item Order site and API edge: Amazon CloudFront; kind product, icon cloudfront, product Amazon CloudFront. `/body/0/children/0/children/1`
      - Fact, doc: VPC origin: the internal load balancer.
  - Box Region; kind region, tint 1. `/body/0/children/1`
    - Fact, doc: Region and zones from the landing-zone design. `/body/0/children/1/children/0`
    - Row, left to right. `/body/0/children/1/children/1`
      - Box Application VPC; kind vpc. `/body/0/children/1/children/1/children/0`
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/0`
          - Item Order API load balancer: Application Load Balancer · internal; kind product, icon elb, product Application Load Balancer. `/body/0/children/1/children/1/children/0/children/0/children/0`
            - Fact, doc: origin of the CloudFront distribution.
          - Item S3 gateway endpoint: gateway endpoint · Amazon S3; kind product, icon vpc, product gateway endpoint. `/body/0/children/1/children/1/children/0/children/0/children/1`
            - Fact, doc: on the private route tables.
          - Item DynamoDB gateway endpoint: gateway endpoint · Amazon DynamoDB; kind product, icon vpc, product gateway endpoint. `/body/0/children/1/children/1/children/0/children/0/children/2`
            - Fact, doc: on the private route tables.
        - Row, left to right. `/body/0/children/1/children/1/children/0/children/1`
          - Box Availability Zone A; kind az. `/body/0/children/1/children/1/children/0/children/1/children/0`
            - Box Private subnet A; kind subnet. `/body/0/children/1/children/1/children/0/children/1/children/0/children/0`
              - Item Order API: AWS Fargate · task in zone A; kind product, icon fargate, product AWS Fargate. `/body/0/children/1/children/1/children/0/children/1/children/0/children/0/children/0`
              - Item Orders database: Amazon RDS for PostgreSQL · primary; kind product, icon rds, product Amazon RDS. `/body/0/children/1/children/1/children/0/children/1/children/0/children/0/children/1`
              - Item Secrets endpoint: interface endpoint; kind product, icon privatelink, product interface endpoint. `/body/0/children/1/children/1/children/0/children/1/children/0/children/0/children/2`
                - Fact, doc: for AWS Secrets Manager.
          - Box Availability Zone B; kind az. `/body/0/children/1/children/1/children/0/children/1/children/1`
            - Box Private subnet B; kind subnet. `/body/0/children/1/children/1/children/0/children/1/children/1/children/0`
              - Item Order API: AWS Fargate · task in zone B; kind product, icon fargate, product AWS Fargate. `/body/0/children/1/children/1/children/0/children/1/children/1/children/0/children/0`
              - Item Orders standby: Amazon RDS for PostgreSQL · Multi-AZ standby; kind product, icon rds, product Amazon RDS. `/body/0/children/1/children/1/children/0/children/1/children/1/children/0/children/1`
              - Fact, doc: No route to an internet gateway. `/body/0/children/1/children/1/children/0/children/1/children/1/children/0/children/2`
      - Pipe Endpoints · gateway and interface.
      - Box Regional services; kind services. `/body/0/children/1/children/1/children/2`
        - Item Menus: Amazon S3; kind product, icon s3, product Amazon S3. `/body/0/children/1/children/1/children/2/children/0`
        - Item Carts: Amazon DynamoDB; kind product, icon dynamodb, product Amazon DynamoDB. `/body/0/children/1/children/1/children/2/children/1`
        - Item API keys: AWS Secrets Manager; kind product, icon secrets-manager, product AWS Secrets Manager. `/body/0/children/1/children/1/children/2/children/2`
        - Item Courier dispatch queue: Amazon SQS; kind product, icon sqs, product Amazon SQS. `/body/0/children/1/children/1/children/2/children/3`
        - Item Order data key: AWS Key Management Service; kind product, icon kms, product AWS Key Management Service. `/body/0/children/1/children/1/children/2/children/4`

## Lines

| Line | Kind | From | To | Style | Ends | Means | Pointer |
|---|---|---|---|---|---|---|---|
| Endpoints · gateway and interface | pipe h | Application VPC (vpc) | Regional services (services) | gray; gray in center | no arrowheads | VPC endpoints to regional services | `/body/0/children/1/children/1/children/1` |

## Legend `/legend`

| Key | Center label | Means | Uses | Pointer |
|---|---|---|---|---|
| gray | Solid gray | VPC endpoints to regional services | 1 | `/legend/0` |

Foot: Illustrative topology · hop facts are placeholders, not looked-up values `/foot`
