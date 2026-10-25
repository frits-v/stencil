// The AWS grammar: the container and item kinds of an AWS architecture
// figure, and the domain rules that need more than a kind list.
// `grammar` is exported to crates/stencil-model/grammars/aws.json.
package aws

import (
	"list"

	"github.com/frits-v/stencil/cue:stencil"
)

grammar: stencil.#Grammar & {
	name: "aws"
	containers: [
		{
			name:     "aws"
			role:     "frame"
			tintable: false
			border: {pattern: "solid", width: 3}
			padding: 0
			radius:  10
			label:   "bar"
			parents: ["page"]
		},
		{
			name:     "account"
			role:     "tile"
			tone:     "highlight"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "aws", "account"]
		},
		{
			name:         "region"
			role:         "group"
			tone:         "neutral"
			tintable:     true
			default_tint: 1
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["aws", "account"]
		},
		{
			// An AWS VPC is regional: a region holds it, and it holds the
			// availability zones its subnets sit in.
			name:     "vpc"
			role:     "boundary"
			tone:     "strong"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 10
			radius:  8
			label:   "plain"
			parents: ["region", "account"]
		},
		{
			name:     "az"
			role:     "group"
			tone:     "cool"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["region", "vpc"]
		},
		{
			name:     "subnet"
			role:     "group"
			tone:     "cool"
			tintable: false
			border: {pattern: "dashed", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["az", "vpc"]
		},
		{
			name:     "onprem"
			role:     "group"
			tone:     "warm"
			tintable: true
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "optional"]
		},
		{
			name:     "optional"
			role:     "group"
			tone:     "emphasis"
			tintable: false
			border: {pattern: "dashed", width: 2}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["page", "aws", "region", "vpc", "az", "subnet", "account"]
		},
		{
			name:     "k8s"
			role:     "group"
			tone:     "soft"
			tintable: false
			border: {pattern: "none", width: 0}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["aws", "region", "vpc", "az", "subnet", "account", "optional"]
		},
		{
			// The regional services a VPC reaches over gateway or interface
			// endpoints, drawn beside the vpc.
			name:     "services"
			role:     "group"
			tone:     "neutral"
			tintable: false
			border: {pattern: "solid", width: 1.5}
			padding: 12
			radius:  8
			label:   "plain"
			parents: ["aws", "region", "account"]
		},
		{
			// The parts one actor runs, on a design sheet only: a dash-dot outline
			// with its name on a tab.
			name:     "assembly"
			role:     "group"
			tone:     "neutral"
			tintable: true
			border: {pattern: "dash-dot", width: 1.1}
			padding: 14
			radius:  0
			label:   "tab"
			parents: ["page", "aws", "account", "region", "vpc", "az", "subnet", "onprem", "optional", "k8s", "services"]
		},
	]
	items: [{
		name:  "product"
		icons: "aws"
		// One row per icon, in AwsIcon::ALL order. product: an
		// Architecture-Service-Icons archive path, standing for one service.
		// category: a Category-Icons path, standing for a service family. A
		// service row lists the full service name first, then its short forms.
		products: [
			{icon: "analytics", class: "category", names: ["Amazon Athena", "AWS Glue", "Amazon Redshift", "Amazon Kinesis", "Amazon Data Firehose", "Amazon EMR", "Amazon OpenSearch Service", "Amazon QuickSight", "AWS Lake Formation", "Amazon Managed Streaming for Apache Kafka", "Amazon MSK"], shape: "cylinder"},
			{icon: "api-gateway", class: "product", names: ["Amazon API Gateway", "API Gateway"]},
			{icon: "application-integration", class: "category", names: ["Amazon Simple Queue Service", "Amazon SQS", "Amazon Simple Notification Service", "Amazon SNS", "Amazon EventBridge", "AWS Step Functions", "Amazon MQ", "Amazon AppFlow"]},
			{icon: "athena", class: "product", names: ["Amazon Athena", "Athena"]},
			{icon: "aurora", class: "product", names: ["Amazon Aurora", "Aurora"], shape: "cylinder"},
			{icon: "bedrock", class: "product", names: ["Amazon Bedrock", "Bedrock"]},
			{icon: "cloudfront", class: "product", names: ["Amazon CloudFront", "CloudFront"], shape: "tile"},
			{icon: "cloudwatch", class: "product", names: ["Amazon CloudWatch", "CloudWatch"]},
			{icon: "compute", class: "category", names: ["Amazon EC2", "AWS Lambda", "AWS Fargate", "AWS Batch", "Amazon Lightsail", "AWS Elastic Beanstalk", "AWS Outposts"], shape: "tower"},
			{icon: "containers", class: "category", names: ["Amazon Elastic Container Service", "Amazon Elastic Kubernetes Service", "AWS Fargate", "Amazon Elastic Container Registry", "AWS App Runner", "Red Hat OpenShift Service on AWS"], shape: "tower"},
			{icon: "database", class: "category", names: ["Amazon Relational Database Service", "Amazon Aurora", "Amazon DynamoDB", "Amazon ElastiCache", "Amazon DocumentDB", "Amazon Neptune", "Amazon Keyspaces", "Amazon MemoryDB", "Amazon Timestream", "AWS Database Migration Service"], shape: "cylinder"},
			{icon: "direct-connect", class: "product", names: ["AWS Direct Connect", "Direct Connect", "DX"], shape: "tile"},
			{icon: "dynamodb", class: "product", names: ["Amazon DynamoDB", "DynamoDB"], shape: "cylinder"},
			{icon: "ebs", class: "product", names: ["Amazon Elastic Block Store", "Amazon EBS", "EBS"], shape: "stack"},
			{icon: "ec2", class: "product", names: ["Amazon Elastic Compute Cloud", "Amazon EC2", "EC2"], shape: "tower"},
			{icon: "ecr", class: "product", names: ["Amazon Elastic Container Registry", "Amazon ECR", "ECR"]},
			{icon: "ecs", class: "product", names: ["Amazon Elastic Container Service", "Amazon ECS", "ECS"], shape: "tower"},
			{icon: "efs", class: "product", names: ["Amazon Elastic File System", "Amazon EFS", "EFS"], shape: "stack"},
			{icon: "eks", class: "product", names: ["Amazon Elastic Kubernetes Service", "Amazon EKS", "EKS"], shape: "tower"},
			{icon: "elasticache", class: "product", names: ["Amazon ElastiCache", "ElastiCache"], shape: "cylinder"},
			{icon: "elb", class: "product", names: ["Elastic Load Balancing", "ELB", "Application Load Balancer", "Network Load Balancer", "Gateway Load Balancer", "ALB", "NLB", "GWLB"], shape: "tile"},
			{icon: "eventbridge", class: "product", names: ["Amazon EventBridge", "EventBridge"]},
			{icon: "fargate", class: "product", names: ["AWS Fargate", "Fargate"]},
			{icon: "glue", class: "product", names: ["AWS Glue", "Glue"]},
			{icon: "iam", class: "product", names: ["AWS Identity and Access Management", "AWS IAM", "IAM"]},
			{icon: "kinesis", class: "product", names: ["Amazon Kinesis", "Kinesis"]},
			{icon: "kms", class: "product", names: ["AWS Key Management Service", "AWS KMS", "KMS"]},
			{icon: "lambda", class: "product", names: ["AWS Lambda", "Lambda"]},
			{icon: "machine-learning", class: "category", names: ["Amazon SageMaker AI", "Amazon Bedrock", "Amazon Q", "Amazon Rekognition", "Amazon Textract", "Amazon Comprehend", "Amazon Transcribe", "Amazon Polly", "Amazon Translate"]},
			{icon: "management-governance", class: "category", names: ["Amazon CloudWatch", "AWS CloudTrail", "AWS Config", "AWS Organizations", "AWS Systems Manager", "AWS CloudFormation", "AWS Control Tower", "AWS Trusted Advisor"]},
			{icon: "networking-content-delivery", class: "category", names: ["Amazon Virtual Private Cloud", "Amazon CloudFront", "Amazon Route 53", "Elastic Load Balancing", "AWS Direct Connect", "AWS Transit Gateway", "AWS PrivateLink", "Amazon API Gateway", "AWS Global Accelerator", "AWS Site-to-Site VPN", "AWS Cloud WAN", "AWS Client VPN", "Amazon VPC Lattice"], shape: "tile"},
			{icon: "privatelink", class: "product", names: ["AWS PrivateLink", "PrivateLink", "interface VPC endpoint", "interface endpoint", "VPC endpoint"], shape: "tile"},
			{icon: "rds", class: "product", names: ["Amazon Relational Database Service", "Amazon RDS", "RDS"], shape: "cylinder"},
			{icon: "redshift", class: "product", names: ["Amazon Redshift", "Redshift"], shape: "cylinder"},
			{icon: "route-53", class: "product", names: ["Amazon Route 53", "Route 53"], shape: "tile"},
			{icon: "s3", class: "product", names: ["Amazon Simple Storage Service", "Amazon S3", "S3"], shape: "stack"},
			{icon: "sagemaker", class: "product", names: ["Amazon SageMaker AI", "SageMaker AI", "SageMaker"]},
			{icon: "secrets-manager", class: "product", names: ["AWS Secrets Manager", "Secrets Manager"]},
			{icon: "security-identity-compliance", class: "category", names: ["AWS Identity and Access Management", "AWS Key Management Service", "AWS Secrets Manager", "AWS WAF", "AWS Shield", "Amazon GuardDuty", "AWS Security Hub", "AWS Certificate Manager", "Amazon Cognito", "AWS IAM Identity Center", "IAM Identity Center", "AWS Network Firewall", "Amazon Inspector", "Amazon Macie"]},
			{icon: "sns", class: "product", names: ["Amazon Simple Notification Service", "Amazon SNS", "SNS"]},
			{icon: "sqs", class: "product", names: ["Amazon Simple Queue Service", "Amazon SQS", "SQS"]},
			{icon: "step-functions", class: "product", names: ["AWS Step Functions", "Step Functions"]},
			{icon: "storage", class: "category", names: ["Amazon Simple Storage Service", "Amazon Elastic Block Store", "Amazon Elastic File System", "Amazon FSx", "Amazon S3 Glacier", "AWS Backup", "AWS Storage Gateway", "AWS DataSync"], shape: "stack"},
			{icon: "transit-gateway", class: "product", names: ["AWS Transit Gateway", "Transit Gateway", "TGW"], shape: "tile"},
			{icon: "vpc", class: "product", names: ["Amazon Virtual Private Cloud", "Amazon VPC", "VPC", "gateway endpoint"], shape: "tile"},
			{icon: "waf", class: "product", names: ["AWS WAF", "WAF"]},
		]
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		// Under iso a product stands as a block unless its icon's row says otherwise;
		// an item asks for the raised card with shape: "card".
		shape: "block"
	}, {
		// A user, operator or team, drawn as a standing figure under iso.
		name:  "person"
		icons: "none"
		products: []
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		shape: "figure"
	}, {
		// A client endpoint, drawn as a laptop under iso; an item may ask for a phone.
		name:  "device"
		icons: "none"
		products: []
		parents: list.Concat([[for c in containers {c.name}], ["page"]])
		shape: "laptop"
	}]
	remembered: [
		{
			literal: "64512"
			reason:  "Amazon-side default ASN of a virtual private gateway, Direct Connect gateway or transit gateway, and settable on each; a default, not a design value. Read the ASN from the gateway's configuration."
		},
		{
			literal: "10.0.0.0/16"
			reason:  "example VPC CIDR of the Amazon VPC documentation, not a design value. The VPC's range comes from the customer's IP plan."
		},
		{
			literal: "10.0.1.0/24"
			reason:  "example subnet CIDR of the Amazon VPC documentation, not a design value. The subnet's range comes from the customer's IP plan."
		},
		{
			literal: "169.254.169.254"
			reason:  "the EC2 instance metadata service address, a fact of the platform, not a design value. Name the instance metadata service (IMDSv2) instead."
		},
	]
}

_nesting: stencil.#GrammarNesting & {#grammar: grammar}

// The literals of grammar.remembered that the core #NoRememberedConstant does
// not hold; 64512 is already there. The Rust check reads every text field;
// this covers the page head, the legend, links and the node text the aws walk
// reaches.
#AwsText: !~"\\b10\\.0\\.0\\.0/16\\b" &
	!~"\\b10\\.0\\.1\\.0/24\\b" &
	!~"\\b169\\.254\\.169\\.254\\b"

// An aws figure: the core page with the aws kinds and nesting, plus the aws
// rules. Each check is a struct keyed by the offending item, so a failure
// names it in its path.
#Page: _nesting.#KindedPage & {
	title:  #AwsText
	kicker: #AwsText
	lede:   #AwsText
	foot?:  #AwsText
	badge?: #AwsText
	legend: [...{text: #AwsText, ...}]
	links?: [...{label?: #AwsText, sub?: #AwsText, ...}]
	body: [...#Node]
	_nameless: {for n in body {n._nameless}}
	// The hop-fact rule. A built fact does not satisfy it: as-built names need
	// no live doc, and they say nothing about what the product is.
	_itemsWithoutSubtitleOrFact: {
		for k, _ in _nameless {(k): "product item" & "needs a subtitle, or a fact with source doc or ask"}
	}
}

// Regional services reached from a VPC over a gateway or interface endpoint,
// never addresses in it: named in a subtitle at word boundaries, or drawn with
// their service icon. A subtitle that names an endpoint draws the endpoint,
// which does sit in the VPC. Lambda, Glue and SageMaker AI are left out: each
// has a VPC-attached form whose interfaces sit in subnets. RDS, Aurora,
// ElastiCache, Redshift, OpenSearch and MSK launch into subnets.
#RegionalServiceName:  "\\b(S3|Simple Storage Service|DynamoDB|SQS|Simple Queue Service|SNS|Simple Notification Service|EventBridge|ECR|Elastic Container Registry|Secrets Manager|KMS|Key Management Service|Step Functions|Kinesis|Athena|Bedrock|CloudWatch|API Gateway)\\b"
#RegionalServiceIcons: ["api-gateway", "athena", "bedrock", "cloudwatch", "dynamodb", "ecr", "eventbridge", "kinesis", "kms", "s3", "secrets-manager", "sns", "sqs", "step-functions"]
#EndpointName:         "\\bendpoints?\\b"

// Global services have no region. Three have regional forms a subtitle names:
// Route 53 Resolver endpoints sit in subnets, IAM Identity Center has a home
// region, and a WAF web ACL for a load balancer or an API is regional. The
// route-53 and waf icons stand for both forms, so only iam and cloudfront are
// read by icon.
#GlobalServiceName:  "\\b(Identity and Access Management|IAM|Route 53|CloudFront|WAF|Organizations|Global Accelerator)\\b"
#GlobalServiceIcons: ["cloudfront", "iam"]
#RegionalFormName:   "\\b(Resolver|Identity Center|regional|Regional)\\b"

// The aws walk over the body. Each node carries:
//   _nameless          the titles of product items with no subtitle and no doc
//                      or ask fact
//   _regionalServices  the titles of product items that are regional services
//                      reached over an endpoint (#RegionalServiceName,
//                      #RegionalServiceIcons)
//   _globalServices    the same for global services (#GlobalServiceName,
//                      #GlobalServiceIcons)
// and holds its text fields to #AwsText. Tint pairing is the core's, in
// #GrammarNesting (cue/grammar.cue).
#Node: {
	tag: string
	if tag == "Row" || tag == "Col" || tag == "Lanes" || tag == "View" {
		children: [...#Node]
		_nameless: {for c in children {c._nameless}}
		_regionalServices: {for c in children {c._regionalServices}}
		_globalServices: {for c in children {c._globalServices}}
	}
	if tag == "Box" {
		kind:  string
		label: #AwsText
		children: [...#Node]
		_nameless: {for c in children {c._nameless}}
		_regionalServices: {for c in children {c._regionalServices}}
		_globalServices: {for c in children {c._globalServices}}

		// A regional service is reached over an endpoint, never placed in a
		// VPC; it belongs in a services Box beside it.
		if kind == "vpc" {
			_regionalServiceInsideVpc: {
				for k, _ in _regionalServices {(k): "product" & "sits inside a vpc; draw it in a services box beside the vpc and the gateway or interface endpoint inside"}
			}
		}
		// A global service has no region; draw it outside every region Box.
		if kind == "region" {
			_globalInsideRegion: {
				for k, _ in _globalServices {(k): "product" & "sits inside a region; a global service sits outside every region"}
			}
		}
	}
	if tag == "Item" {
		kind:      string
		title:     #AwsText
		icon?:     string
		subtitle?: #AwsText
		facts?: [...{text: #AwsText, ...}]
		_hopFacts: [
			if facts != _|_ for f in facts
			let source = [if f.source != _|_ {f.source}, "doc"][0]
			if source != "built" {f},
		]
		_nameless: {if kind == "product" && subtitle == _|_ && len(_hopFacts) == 0 {(title): true}}
		_isRegional: [
			if subtitle != _|_ if subtitle =~ #RegionalServiceName if subtitle !~ #EndpointName {true},
			if icon != _|_ if list.Contains(#RegionalServiceIcons, icon) {true},
		]
		_regionalServices: {if kind == "product" && len(_isRegional) > 0 {(title): true}}
		_isGlobal: [
			if subtitle != _|_ if subtitle =~ #GlobalServiceName if subtitle !~ #RegionalFormName {true},
			if icon != _|_ if list.Contains(#GlobalServiceIcons, icon) {true},
		]
		_globalServices: {if kind == "product" && len(_isGlobal) > 0 {(title): true}}
	}
	if tag == "Fact" || tag == "Note" {
		text: #AwsText
	}
	if tag == "Pipe" {
		label: #AwsText
		sub?:  #AwsText
	}
	if tag == "Tee" {
		hub: #AwsText
		arms: [...{label: #AwsText, sub?: #AwsText, ...}]
	}
	if !list.Contains(["Row", "Col", "Lanes", "View", "Box", "Item"], tag) {
		_nameless: {}
		_regionalServices: {}
		_globalServices: {}
	}
	...
}
