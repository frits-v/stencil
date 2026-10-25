//! The AWS pack, `assets/icons/aws/`.

use stencil_model::AwsIcon;

pub(super) fn svg_bytes(icon: AwsIcon) -> &'static [u8] {
    match icon {
        AwsIcon::Analytics => include_bytes!("../../../../assets/icons/aws/analytics.svg"),
        AwsIcon::ApiGateway => include_bytes!("../../../../assets/icons/aws/api-gateway.svg"),
        AwsIcon::ApplicationIntegration => {
            include_bytes!("../../../../assets/icons/aws/application-integration.svg")
        }
        AwsIcon::Athena => include_bytes!("../../../../assets/icons/aws/athena.svg"),
        AwsIcon::Aurora => include_bytes!("../../../../assets/icons/aws/aurora.svg"),
        AwsIcon::Bedrock => include_bytes!("../../../../assets/icons/aws/bedrock.svg"),
        AwsIcon::Cloudfront => include_bytes!("../../../../assets/icons/aws/cloudfront.svg"),
        AwsIcon::Cloudwatch => include_bytes!("../../../../assets/icons/aws/cloudwatch.svg"),
        AwsIcon::Compute => include_bytes!("../../../../assets/icons/aws/compute.svg"),
        AwsIcon::Containers => include_bytes!("../../../../assets/icons/aws/containers.svg"),
        AwsIcon::Database => include_bytes!("../../../../assets/icons/aws/database.svg"),
        AwsIcon::DirectConnect => include_bytes!("../../../../assets/icons/aws/direct-connect.svg"),
        AwsIcon::Dynamodb => include_bytes!("../../../../assets/icons/aws/dynamodb.svg"),
        AwsIcon::Ebs => include_bytes!("../../../../assets/icons/aws/ebs.svg"),
        AwsIcon::Ec2 => include_bytes!("../../../../assets/icons/aws/ec2.svg"),
        AwsIcon::Ecr => include_bytes!("../../../../assets/icons/aws/ecr.svg"),
        AwsIcon::Ecs => include_bytes!("../../../../assets/icons/aws/ecs.svg"),
        AwsIcon::Efs => include_bytes!("../../../../assets/icons/aws/efs.svg"),
        AwsIcon::Eks => include_bytes!("../../../../assets/icons/aws/eks.svg"),
        AwsIcon::Elasticache => include_bytes!("../../../../assets/icons/aws/elasticache.svg"),
        AwsIcon::Elb => include_bytes!("../../../../assets/icons/aws/elb.svg"),
        AwsIcon::Eventbridge => include_bytes!("../../../../assets/icons/aws/eventbridge.svg"),
        AwsIcon::Fargate => include_bytes!("../../../../assets/icons/aws/fargate.svg"),
        AwsIcon::Glue => include_bytes!("../../../../assets/icons/aws/glue.svg"),
        AwsIcon::Iam => include_bytes!("../../../../assets/icons/aws/iam.svg"),
        AwsIcon::Kinesis => include_bytes!("../../../../assets/icons/aws/kinesis.svg"),
        AwsIcon::Kms => include_bytes!("../../../../assets/icons/aws/kms.svg"),
        AwsIcon::Lambda => include_bytes!("../../../../assets/icons/aws/lambda.svg"),
        AwsIcon::MachineLearning => {
            include_bytes!("../../../../assets/icons/aws/machine-learning.svg")
        }
        AwsIcon::ManagementGovernance => {
            include_bytes!("../../../../assets/icons/aws/management-governance.svg")
        }
        AwsIcon::NetworkingContentDelivery => {
            include_bytes!("../../../../assets/icons/aws/networking-content-delivery.svg")
        }
        AwsIcon::Privatelink => include_bytes!("../../../../assets/icons/aws/privatelink.svg"),
        AwsIcon::Rds => include_bytes!("../../../../assets/icons/aws/rds.svg"),
        AwsIcon::Redshift => include_bytes!("../../../../assets/icons/aws/redshift.svg"),
        AwsIcon::Route53 => include_bytes!("../../../../assets/icons/aws/route-53.svg"),
        AwsIcon::S3 => include_bytes!("../../../../assets/icons/aws/s3.svg"),
        AwsIcon::Sagemaker => include_bytes!("../../../../assets/icons/aws/sagemaker.svg"),
        AwsIcon::SecretsManager => {
            include_bytes!("../../../../assets/icons/aws/secrets-manager.svg")
        }
        AwsIcon::SecurityIdentityCompliance => {
            include_bytes!("../../../../assets/icons/aws/security-identity-compliance.svg")
        }
        AwsIcon::Sns => include_bytes!("../../../../assets/icons/aws/sns.svg"),
        AwsIcon::Sqs => include_bytes!("../../../../assets/icons/aws/sqs.svg"),
        AwsIcon::StepFunctions => include_bytes!("../../../../assets/icons/aws/step-functions.svg"),
        AwsIcon::Storage => include_bytes!("../../../../assets/icons/aws/storage.svg"),
        AwsIcon::TransitGateway => {
            include_bytes!("../../../../assets/icons/aws/transit-gateway.svg")
        }
        AwsIcon::Vpc => include_bytes!("../../../../assets/icons/aws/vpc.svg"),
        AwsIcon::Waf => include_bytes!("../../../../assets/icons/aws/waf.svg"),
    }
}
