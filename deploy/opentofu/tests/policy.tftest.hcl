mock_provider "aws" {
  mock_resource "aws_iam_role" {
    defaults = { arn = "arn:aws:iam::111122223333:role/mock" }
  }
  mock_resource "aws_kms_key" {
    defaults = { arn = "arn:aws:kms:us-west-2:111122223333:key/00000000-0000-0000-0000-000000000000" }
  }
  mock_resource "aws_lb" {
    defaults = { arn = "arn:aws:elasticloadbalancing:us-west-2:111122223333:loadbalancer/app/mock/0000000000000000", arn_suffix = "app/mock/0000000000000000" }
  }
  mock_resource "aws_lb_target_group" {
    defaults = { arn = "arn:aws:elasticloadbalancing:us-west-2:111122223333:targetgroup/mock/0000000000000000" }
  }
  mock_resource "aws_lb_listener" {
    defaults = { arn = "arn:aws:elasticloadbalancing:us-west-2:111122223333:listener/app/mock/0000000000000000/0000000000000000" }
  }
  mock_resource "aws_sns_topic" {
    defaults = { arn = "arn:aws:sns:us-west-2:111122223333:mock" }
  }
}
mock_provider "random" {
  mock_resource "random_password" {
    defaults = { result = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
  }
}
mock_provider "aws" {
  alias = "us_east_1"
}

run "security_baseline_plan" {
  command = plan
  variables {
    aws_region                                = "us-west-2"
    environment                               = "staging"
    deployment_id                             = "audit-20260812"
    domain_name                               = "learn.example.edu"
    certificate_arn                           = "arn:aws:acm:us-east-1:111122223333:certificate/00000000-0000-0000-0000-000000000000"
    origin_certificate_arn                    = "arn:aws:acm:us-west-2:111122223333:certificate/00000000-0000-0000-0000-000000000000"
    origin_domain_name                        = "origin.learn.example.edu"
    origin_hosted_zone_id                     = "Z000000000000000000000"
    vpc_cidr                                  = "10.42.0.0/20"
    availability_zones                        = ["us-west-2a", "us-west-2b"]
    api_image                                 = "111122223333.dkr.ecr.us-west-2.amazonaws.com/ple-api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    worker_image                              = "111122223333.dkr.ecr.us-west-2.amazonaws.com/ple-worker@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    secret_file_writer_image                  = "111122223333.dkr.ecr.us-west-2.amazonaws.com/ple-secret-file-writer@sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    api_application_secrets_arn               = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-api-runtime-test"
    worker_application_secrets_arn            = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-worker-runtime-test"
    recovery_application_secrets_arn          = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-accepted-submission-recovery-test"
    fast_path_application_secrets_arn         = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-accepted-submission-fast-path-test"
    api_application_secrets_kms_key_arn       = "arn:aws:kms:us-west-2:111122223333:key/00000000-0000-0000-0000-000000000001"
    worker_application_secrets_kms_key_arn    = "arn:aws:kms:us-west-2:111122223333:key/00000000-0000-0000-0000-000000000002"
    recovery_application_secrets_kms_key_arn  = "arn:aws:kms:us-west-2:111122223333:key/00000000-0000-0000-0000-000000000004"
    fast_path_application_secrets_kms_key_arn = "arn:aws:kms:us-west-2:111122223333:key/00000000-0000-0000-0000-000000000005"
    publisher_application_secrets_arn         = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-publisher-runtime-test"
    publisher_application_secrets_kms_key_arn = "arn:aws:kms:us-west-2:111122223333:key/00000000-0000-0000-0000-000000000003"
    smtp_security_group_id                    = "sg-11111111111111111"
    imathas_security_group_id                 = "sg-22222222222222222"
    renderer_security_group_id                = "sg-33333333333333333"
    alert_email                               = "operations@example.edu"
  }

  assert {
    condition     = aws_db_instance.postgres.publicly_accessible == false && aws_db_instance.postgres.storage_encrypted && aws_db_instance.postgres.deletion_protection
    error_message = "RDS must remain private, encrypted, and deletion protected."
  }
  assert {
    condition     = strcontains(aws_s3_bucket_policy.object["public_assets"].policy, "/problems/*/assets/*") && !strcontains(aws_s3_bucket_policy.object["private_content"].policy, "\"Resource\":\"arn:aws:s3:::private_content/problems/*/assets/*\"") && strcontains(aws_s3_bucket_policy.object["private_content"].policy, "__no-public-immutability-policy__")
    error_message = "Public immutable tag fencing must not affect private restricted ProblemAsset objects."
  }
  assert {
    condition     = alltrue([for group in [aws_security_group.api, aws_security_group.worker, aws_security_group.publisher, aws_security_group.alb, aws_security_group.database, aws_security_group.vpce] : length(group.egress) == 0])
    error_message = "Security groups must revoke implicit AWS egress; every egress path is an explicit standalone rule."
  }
  assert {
    condition     = alltrue([for bucket in aws_s3_bucket_public_access_block.object : bucket.block_public_acls && bucket.block_public_policy && bucket.ignore_public_acls && bucket.restrict_public_buckets])
    error_message = "Every object bucket must block all public access mechanisms."
  }
  assert {
    condition = (
      aws_cloudfront_distribution.main.default_cache_behavior[0].response_headers_policy_id == aws_cloudfront_response_headers_policy.browser.id
      && length([for behavior in aws_cloudfront_distribution.main.ordered_cache_behavior : behavior if behavior.path_pattern == "/api" && behavior.target_origin_id == "api" && behavior.response_headers_policy_id == aws_cloudfront_response_headers_policy.api.id]) == 1
      && length([for behavior in aws_cloudfront_distribution.main.ordered_cache_behavior : behavior if behavior.path_pattern == "/api/*" && behavior.target_origin_id == "api" && behavior.response_headers_policy_id == aws_cloudfront_response_headers_policy.api.id]) == 1
    )
    error_message = "Static content needs the browser CSP; both exact and descendant API paths need the no-CSP API policy."
  }
  assert {
    condition     = length(aws_vpc_security_group_egress_rule.api_to_imathas) == (var.enable_imathas ? 1 : 0) && length(aws_vpc_security_group_egress_rule.api_to_renderer) == (var.enable_webwork ? 1 : 0) && length(aws_vpc_security_group_egress_rule.api_to_smtp) == (var.enable_smtp ? 1 : 0)
    error_message = "Each external integration must gain API egress only when that capability is enabled."
  }
  assert {
    condition     = strcontains(aws_ecs_task_definition.publisher.container_definitions, "--public-asset-publisher") && aws_ecs_service.publisher.network_configuration[0].assign_public_ip == false && strcontains(aws_iam_role_policy.publisher_storage.policy, "ple-published-immutable")
    error_message = "Public-asset promotion must run as its own private, narrow publisher task."
  }
  assert {
    condition     = aws_ecs_service.api.network_configuration[0].assign_public_ip == false && aws_ecs_service.worker.network_configuration[0].assign_public_ip == false
    error_message = "Fargate tasks must not receive public IP addresses."
  }
  assert {
    condition     = alltrue([for container in concat(jsondecode(aws_ecs_task_definition.api.container_definitions), jsondecode(aws_ecs_task_definition.worker.container_definitions)) : container.readonlyRootFilesystem && !contains(["", "0", "root"], container.user)])
    error_message = "Every application container must have a non-root user and read-only root filesystem."
  }
  assert {
    condition     = aws_iam_role.api.name != aws_iam_role.worker.name && aws_s3_bucket.object["public_assets"].object_lock_enabled
    error_message = "API and worker must retain separate identities, and published assets must use Object Lock."
  }
  assert {
    condition     = strcontains(aws_s3_bucket_policy.object["public_assets"].policy, "DenyPublishedAssetOverwrite") && strcontains(aws_s3_bucket_policy.object["public_assets"].policy, "ple-published-immutable")
    error_message = "Published-asset writes must be tag-fenced and overwrite denied in the bucket policy."
  }
  assert {
    condition     = strcontains(aws_s3_bucket_policy.object["public_assets"].policy, "DenyImmutablePublishedAssetRetag") && strcontains(aws_s3_bucket_policy.object["public_assets"].policy, "s3:PutObjectTagging") && strcontains(aws_s3_bucket_policy.object["public_assets"].policy, "s3:DeleteObjectTagging")
    error_message = "A published immutable tag must not be mutable or removable before an overwrite attempt."
  }
  assert {
    condition     = anytrue([for origin in aws_cloudfront_distribution.main.origin : origin.origin_id == "api" && anytrue([for header in origin.custom_header : header.name == "X-PLE-Origin-Verify"])])
    error_message = "CloudFront must authenticate requests to the API origin with its state-only header."
  }
  assert {
    condition     = anytrue([for origin in aws_cloudfront_distribution.main.origin : origin.domain_name == var.origin_domain_name && origin.origin_id == "api" && anytrue([for config in origin.custom_origin_config : config.origin_protocol_policy == "https-only"])])
    error_message = "The API origin must use its controlled TLS DNS name."
  }
  assert {
    condition     = length(aws_cloudfront_response_headers_policy.api.security_headers_config[0].content_security_policy) == 0 && length(aws_cloudfront_response_headers_policy.browser.security_headers_config[0].content_security_policy) == 1
    error_message = "API edge policy must not overwrite the application CSP."
  }
  assert {
    condition     = aws_iam_role.api_execution.name != aws_iam_role.worker_execution.name && aws_iam_role.worker_execution.name != aws_iam_role.publisher_execution.name
    error_message = "API, worker, and publisher must keep separate execution identities."
  }
  assert {
    condition     = alltrue([for behavior in aws_cloudfront_distribution.main.ordered_cache_behavior : behavior.origin_request_policy_id == data.aws_cloudfront_origin_request_policy.all_viewer.id if behavior.path_pattern == "/api" || behavior.path_pattern == "/api/*"])
    error_message = "CloudFront must forward the canonical viewer Host to the exact-host application boundary."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.api_storage.policy, "/records/*") && strcontains(aws_iam_role_policy.api_storage.policy, "/processing/*") && !strcontains(aws_iam_role_policy.api_storage.policy, "/imports/*")
    error_message = "Storage IAM must track typed ObjectKey prefixes, not obsolete broad paths."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.publisher_storage.policy, "ple-published-immutable") && !strcontains(aws_iam_role_policy.publisher_storage.policy, "DeleteObject") && !strcontains(aws_iam_role_policy.api_storage.policy, "s3:RequestObjectTag/ple-published-immutable")
    error_message = "Only the narrow publisher principal may create immutable public assets; API may not write or delete them."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.publisher_storage.policy, "/workspaces/*/*/imports/*/assets/*") && strcontains(aws_iam_role_policy.publisher_storage.policy, "/workspaces/*/*/questions/assets/*") && !strcontains(aws_iam_role_policy.publisher_storage.policy, "private_content/problems/*")
    error_message = "Publisher private reads must be limited to the two typed workspace asset paths."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.api_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, aws_kms_key.secrets.arn) && strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.publisher_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.worker_application_secrets_arn)
    error_message = "Every execution role must decrypt only its own externally managed application-secret CMK and JSON value."
  }
  assert {
    condition     = alltrue([for policy in [aws_iam_role_policy.api_execution_secrets.policy, aws_iam_role_policy.worker_execution_secrets.policy, aws_iam_role_policy.publisher_execution_secrets.policy] : strcontains(policy, "kms:ViaService") && strcontains(policy, "kms:EncryptionContext:SecretARN") && strcontains(policy, "secretsmanager.${var.aws_region}.amazonaws.com")])
    error_message = "Execution-role KMS decrypt must be usable only through regional Secrets Manager and its exact secret encryption context."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.api_application_secrets_arn) && strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.api_application_secrets_kms_key_arn) && strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.fast_path_application_secrets_arn) && strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.fast_path_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.worker_application_secrets_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.recovery_application_secrets_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.publisher_application_secrets_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.worker_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.recovery_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.api_execution_secrets.policy, var.publisher_application_secrets_kms_key_arn)
    error_message = "API execution identity must read only the API and dedicated fast-path secret values and their CMKs."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.worker_application_secrets_arn) && strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.worker_application_secrets_kms_key_arn) && strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.recovery_application_secrets_arn) && strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.recovery_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.api_application_secrets_arn) && !strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.fast_path_application_secrets_arn) && !strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.publisher_application_secrets_arn) && !strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.api_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.fast_path_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.worker_execution_secrets.policy, var.publisher_application_secrets_kms_key_arn)
    error_message = "Worker execution identity must read only the worker and dedicated recovery secret values and their CMKs."
  }
  assert {
    condition     = length(toset([var.api_application_secrets_arn, var.worker_application_secrets_arn, var.recovery_application_secrets_arn, var.fast_path_application_secrets_arn, var.publisher_application_secrets_arn])) == 5 && length(toset([var.api_application_secrets_kms_key_arn, var.worker_application_secrets_kms_key_arn, var.recovery_application_secrets_kms_key_arn, var.fast_path_application_secrets_kms_key_arn, var.publisher_application_secrets_kms_key_arn])) == 5 && strcontains(aws_ecs_task_definition.worker.container_definitions, "PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL") && strcontains(aws_ecs_task_definition.worker.container_definitions, var.recovery_application_secrets_arn) && !strcontains(aws_ecs_task_definition.worker.container_definitions, "PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL")
    error_message = "Each process capability must use a distinct secret and CMK, with recovery credentials reaching only the worker task."
  }
  assert {
    condition     = strcontains(aws_ecs_task_definition.api.container_definitions, "PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL") && strcontains(aws_ecs_task_definition.api.container_definitions, var.fast_path_application_secrets_arn) && !strcontains(aws_ecs_task_definition.api.container_definitions, "PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL")
    error_message = "Fast-path credentials must reach only the API task, while recovery credentials stay worker-only."
  }
  assert {
    condition     = strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.publisher_application_secrets_arn) && strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.publisher_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.api_application_secrets_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.worker_application_secrets_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.recovery_application_secrets_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.fast_path_application_secrets_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.api_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.worker_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.recovery_application_secrets_kms_key_arn) && !strcontains(aws_iam_role_policy.publisher_execution_secrets.policy, var.fast_path_application_secrets_kms_key_arn)
    error_message = "Publisher execution identity must be bound only to the publisher secret ARN and CMK."
  }
}
