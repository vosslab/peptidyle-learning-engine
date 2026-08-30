data "aws_caller_identity" "current" {}

locals {
  ecs_assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Action    = "sts:AssumeRole"
      Principal = { Service = "ecs-tasks.amazonaws.com" }
      Condition = {
        StringEquals = { "aws:SourceAccount" = data.aws_caller_identity.current.account_id }
        ArnLike      = { "aws:SourceArn" = "arn:aws:ecs:${var.aws_region}:${data.aws_caller_identity.current.account_id}:*" }
      }
    }]
  })
}

resource "aws_iam_role" "api_execution" {
  name               = "${local.name}-api-execution"
  assume_role_policy = local.ecs_assume_role_policy
}
resource "aws_iam_role" "worker_execution" {
  name               = "${local.name}-worker-execution"
  assume_role_policy = local.ecs_assume_role_policy
}
resource "aws_iam_role" "publisher_execution" {
  name               = "${local.name}-publisher-execution"
  assume_role_policy = local.ecs_assume_role_policy
}
resource "aws_iam_role_policy_attachment" "api_execution" {
  role       = aws_iam_role.api_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}
resource "aws_iam_role_policy_attachment" "worker_execution" {
  role       = aws_iam_role.worker_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}
resource "aws_iam_role_policy_attachment" "publisher_execution" {
  role       = aws_iam_role.publisher_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}
resource "aws_iam_role_policy" "api_execution_secrets" {
  name = "read-api-runtime-secret"
  role = aws_iam_role.api_execution.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [
    { Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = var.api_application_secrets_arn },
    { Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = var.fast_path_application_secrets_arn },
    { Effect = "Allow", Action = ["kms:Decrypt"], Resource = var.api_application_secrets_kms_key_arn, Condition = { StringEquals = { "kms:ViaService" = "secretsmanager.${var.aws_region}.amazonaws.com", "kms:EncryptionContext:SecretARN" = var.api_application_secrets_arn } } },
    { Effect = "Allow", Action = ["kms:Decrypt"], Resource = var.fast_path_application_secrets_kms_key_arn, Condition = { StringEquals = { "kms:ViaService" = "secretsmanager.${var.aws_region}.amazonaws.com", "kms:EncryptionContext:SecretARN" = var.fast_path_application_secrets_arn } } }
  ] })
}
resource "aws_iam_role_policy" "worker_execution_secrets" {
  name = "read-worker-runtime-secret"
  role = aws_iam_role.worker_execution.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [
    { Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = var.worker_application_secrets_arn },
    { Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = var.recovery_application_secrets_arn },
    { Effect = "Allow", Action = ["kms:Decrypt"], Resource = var.worker_application_secrets_kms_key_arn, Condition = { StringEquals = { "kms:ViaService" = "secretsmanager.${var.aws_region}.amazonaws.com", "kms:EncryptionContext:SecretARN" = var.worker_application_secrets_arn } } },
    { Effect = "Allow", Action = ["kms:Decrypt"], Resource = var.recovery_application_secrets_kms_key_arn, Condition = { StringEquals = { "kms:ViaService" = "secretsmanager.${var.aws_region}.amazonaws.com", "kms:EncryptionContext:SecretARN" = var.recovery_application_secrets_arn } } }
  ] })
}
resource "aws_iam_role_policy" "publisher_execution_secrets" {
  name = "read-publisher-runtime-secret"
  role = aws_iam_role.publisher_execution.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [{
    Effect = "Allow", Action = ["secretsmanager:GetSecretValue"], Resource = var.publisher_application_secrets_arn
    }, {
    Effect    = "Allow", Action = ["kms:Decrypt"], Resource = var.publisher_application_secrets_kms_key_arn,
    Condition = { StringEquals = { "kms:ViaService" = "secretsmanager.${var.aws_region}.amazonaws.com", "kms:EncryptionContext:SecretARN" = var.publisher_application_secrets_arn } }
  }] })
}

resource "aws_iam_role" "api" {
  name               = "${local.name}-api"
  assume_role_policy = local.ecs_assume_role_policy
}
resource "aws_iam_role" "worker" {
  name               = "${local.name}-worker"
  assume_role_policy = local.ecs_assume_role_policy
}
resource "aws_iam_role" "publisher" {
  name               = "${local.name}-publisher"
  assume_role_policy = local.ecs_assume_role_policy
}

resource "aws_iam_role_policy" "api_storage" {
  name = "typed-storage-only"
  role = aws_iam_role.api.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [
    { Effect = "Allow", Action = ["s3:GetObject"], Resource = ["${aws_s3_bucket.object["public_assets"].arn}/problems/*/assets/*", "${aws_s3_bucket.object["private_content"].arn}/workspaces/*", "${aws_s3_bucket.object["private_content"].arn}/problems/*", "${aws_s3_bucket.object["student_records"].arn}/records/*", "${aws_s3_bucket.object["temp_processing"].arn}/processing/*"] },
    { Effect = "Allow", Action = ["s3:GetObjectTagging"], Resource = "${aws_s3_bucket.object["public_assets"].arn}/problems/*/assets/*" },
    { Effect = "Allow", Action = ["s3:PutObject", "s3:PutObjectTagging", "s3:AbortMultipartUpload"], Resource = ["${aws_s3_bucket.object["private_content"].arn}/workspaces/*", "${aws_s3_bucket.object["private_content"].arn}/problems/*", "${aws_s3_bucket.object["student_records"].arn}/records/*", "${aws_s3_bucket.object["temp_processing"].arn}/processing/*"] },
    { Effect = "Allow", Action = ["s3:DeleteObject"], Resource = ["${aws_s3_bucket.object["temp_processing"].arn}/processing/*"] },
    { Effect = "Allow", Action = ["kms:Decrypt", "kms:GenerateDataKey"], Resource = [for key in aws_kms_key.object : key.arn] }
  ] })
}

# The publication outbox owns public promotion. It is intentionally a role
# only until the outbox publisher process is accepted; API never receives a
# public-assets Put grant again.
resource "aws_iam_role_policy" "publisher_storage" {
  name = "immutable-publication-only"
  role = aws_iam_role.publisher.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [
    { Effect = "Allow", Action = ["s3:GetObject"], Resource = ["${aws_s3_bucket.object["private_content"].arn}/workspaces/*/*/imports/*/assets/*", "${aws_s3_bucket.object["private_content"].arn}/workspaces/*/*/questions/assets/*", "${aws_s3_bucket.object["public_assets"].arn}/problems/*/assets/*"] },
    { Effect = "Allow", Action = ["s3:GetObjectTagging"], Resource = "${aws_s3_bucket.object["public_assets"].arn}/problems/*/assets/*" },
    { Effect = "Allow", Action = ["s3:PutObject", "s3:PutObjectTagging", "s3:AbortMultipartUpload"], Resource = "${aws_s3_bucket.object["public_assets"].arn}/problems/*/assets/*", Condition = { StringEquals = { "s3:RequestObjectTag/ple-published-immutable" = "true" } } },
    { Effect = "Allow", Action = ["kms:Decrypt"], Resource = aws_kms_key.object["private_content"].arn },
    { Effect = "Allow", Action = ["kms:GenerateDataKey"], Resource = aws_kms_key.object["public_assets"].arn }
  ] })
}

resource "aws_iam_role_policy" "worker_storage" {
  name = "worker-storage-only"
  role = aws_iam_role.worker.id
  policy = jsonencode({ Version = "2012-10-17", Statement = [
    { Effect = "Allow", Action = ["s3:GetObject"], Resource = ["${aws_s3_bucket.object["private_content"].arn}/problems/*", "${aws_s3_bucket.object["student_records"].arn}/records/*", "${aws_s3_bucket.object["temp_processing"].arn}/processing/*"] },
    { Effect = "Allow", Action = ["s3:PutObject", "s3:AbortMultipartUpload", "s3:DeleteObject"], Resource = ["${aws_s3_bucket.object["private_content"].arn}/problems/*", "${aws_s3_bucket.object["student_records"].arn}/records/*", "${aws_s3_bucket.object["temp_processing"].arn}/processing/*"] },
    { Effect = "Allow", Action = ["kms:Decrypt", "kms:GenerateDataKey"], Resource = [aws_kms_key.object["private_content"].arn, aws_kms_key.object["student_records"].arn, aws_kms_key.object["temp_processing"].arn] }
  ] })
}

resource "aws_cloudwatch_log_group" "application" {
  for_each          = toset(["api", "worker", "publisher"])
  name              = "/ple/${local.name}/${each.value}"
  retention_in_days = 365
  kms_key_id        = aws_kms_key.logs.arn
}

resource "aws_ecs_cluster" "main" {
  name = local.name
  setting {
    name  = "containerInsights"
    value = "enabled"
  }
}

locals {
  runtime_environment = [
    { name = "PLE_BIND_ADDR", value = "0.0.0.0:3000" },
    { name = "PLE_S3_REGION", value = var.aws_region },
    { name = "PLE_PUBLIC_ASSETS_BUCKET", value = aws_s3_bucket.object["public_assets"].bucket },
    { name = "PLE_PRIVATE_CONTENT_BUCKET", value = aws_s3_bucket.object["private_content"].bucket },
    { name = "PLE_STUDENT_RECORDS_BUCKET", value = aws_s3_bucket.object["student_records"].bucket },
    { name = "PLE_TEMP_PROCESSING_BUCKET", value = aws_s3_bucket.object["temp_processing"].bucket },
    { name = "PLE_PUBLIC_ASSETS_KMS_KEY_ARN", value = aws_kms_key.object["public_assets"].arn },
    { name = "PLE_PRIVATE_CONTENT_KMS_KEY_ARN", value = aws_kms_key.object["private_content"].arn },
    { name = "PLE_STUDENT_RECORDS_KMS_KEY_ARN", value = aws_kms_key.object["student_records"].arn },
    { name = "PLE_TEMP_PROCESSING_KMS_KEY_ARN", value = aws_kms_key.object["temp_processing"].arn }
  ]
  api_required_secret_keys = ["DATABASE_URL", "PLE_GRADER_DATABASE_URL", "PLE_WEBAUTHN_ORIGIN", "PLE_WEBAUTHN_RP_ID", "PLE_WEBAUTHN_RP_NAME", "PLE_TRUSTED_PROXY_CIDRS", "PLE_PUBLIC_ASSET_BASE_URL", "PLE_QUESTION_ID_SECRET"]
  imathas_secret_keys      = ["PLE_IMATHAS_BASE_URL", "PLE_IMATHAS_REQUEST_TIMEOUT_SECONDS", "PLE_IMATHAS_MAX_TRANSPORT_BYTES", "PLE_IMATHAS_MAX_SNAPSHOT_BYTES", "PLE_IMATHAS_MAX_RESULT_BYTES", "PLE_IMATHAS_LAUNCH_TTL_MILLIS", "PLE_IMATHAS_LAUNCH_STATE_SECRET", "PLE_IMATHAS_CORRELATION_SECRET", "PLE_IMATHAS_LAUNCH_SIGNING_SECRET", "PLE_IMATHAS_RESULT_VERIFICATION_SECRET", "PLE_IMATHAS_PROVIDER_KEY", "PLE_IMATHAS_PROVIDER_AUTH_HEADER_NAME", "PLE_IMATHAS_PROVIDER_AUTH_VALUE"]
  smtp_secret_keys         = ["PLE_SMTP_RELAY", "PLE_SMTP_PORT", "PLE_SMTP_TLS_MODE", "PLE_SMTP_USERNAME", "PLE_SMTP_FROM", "PLE_PUBLIC_APP_BASE_URL"]
  webwork_secret_keys      = ["PLE_WEBWORK_RENDERER_BASE_URL", "PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS", "PLE_WEBWORK_MAX_RESPONSE_BYTES", "PLE_WEBWORK_RENDERER_ID", "PLE_WEBWORK_RENDERER_VERSION"]
  api_secrets = [
    for key in concat(local.api_required_secret_keys, var.enable_imathas ? local.imathas_secret_keys : [], var.enable_smtp ? local.smtp_secret_keys : [], var.enable_webwork ? local.webwork_secret_keys : []) :
    { name = key, valueFrom = "${var.api_application_secrets_arn}:${key}::" }
  ]
  fast_path_secrets = [
    { name = "PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL", valueFrom = "${var.fast_path_application_secrets_arn}:PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL::" }
  ]
  worker_secrets = [
    for key in ["PLE_WORKER_DATABASE_URL", "PLE_GRADER_DATABASE_URL"] :
    { name = key, valueFrom = "${var.worker_application_secrets_arn}:${key}::" }
  ]
  recovery_secrets = [
    { name = "PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL", valueFrom = "${var.recovery_application_secrets_arn}:PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL::" }
  ]
  publisher_secrets = [
    { name = "PLE_PUBLISHER_DATABASE_URL", valueFrom = "${var.publisher_application_secrets_arn}:PLE_PUBLISHER_DATABASE_URL::" }
  ]
}

resource "aws_ecs_task_definition" "api" {
  family                   = "${local.name}-api"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.api_cpu
  memory                   = var.api_memory
  execution_role_arn       = aws_iam_role.api_execution.arn
  task_role_arn            = aws_iam_role.api.arn
  container_definitions = jsonencode(concat(
    var.enable_smtp ? [{
      name             = "secret-files", image = var.secret_file_writer_image, essential = false, user = "10001", readonlyRootFilesystem = true, stopTimeout = 45,
      environment      = [{ name = "PLE_SECRET_OUTPUT_DIR", value = "/run/ple-secrets" }],
      secrets          = var.enable_smtp ? [for key in ["PLE_SMTP_PASSWORD", "PLE_INVITATION_TOKEN_SECRET"] : { name = key, valueFrom = "${var.api_application_secrets_arn}:${key}::" }] : [],
      mountPoints      = [{ sourceVolume = "runtime-secrets", containerPath = "/run/ple-secrets", readOnly = false }],
      linuxParameters  = { initProcessEnabled = true },
      logConfiguration = { logDriver = "awslogs", options = { awslogs-group = aws_cloudwatch_log_group.application["api"].name, awslogs-region = var.aws_region, awslogs-stream-prefix = "secret-files" } }
    }] : [],
    [{
      name             = "api", image = var.api_image, essential = true, user = "10001", readonlyRootFilesystem = true, stopTimeout = 45,
      dependsOn        = var.enable_smtp ? [{ containerName = "secret-files", condition = "SUCCESS" }] : [],
      portMappings     = [{ containerPort = 3000, protocol = "tcp" }],
      environment      = concat(local.runtime_environment, var.enable_smtp ? [{ name = "PLE_SMTP_PASSWORD_FILE", value = "/run/ple-secrets/smtp-password" }, { name = "PLE_INVITATION_TOKEN_SECRET_FILE", value = "/run/ple-secrets/invitation-token" }] : []), secrets = concat(local.api_secrets, local.fast_path_secrets),
      mountPoints      = [{ sourceVolume = "runtime-secrets", containerPath = "/run/ple-secrets", readOnly = true }],
      linuxParameters  = { initProcessEnabled = true },
      logConfiguration = { logDriver = "awslogs", options = { awslogs-group = aws_cloudwatch_log_group.application["api"].name, awslogs-region = var.aws_region, awslogs-stream-prefix = "ecs" } },
      healthCheck      = { command = ["CMD", "/usr/local/bin/peptidyle-api", "--health-probe"], interval = 30, timeout = 5, retries = 3, startPeriod = 30 }
    }]
  ))
  volume { name = "runtime-secrets" }
}

resource "aws_ecs_task_definition" "worker" {
  family                   = "${local.name}-worker"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.worker_cpu
  memory                   = var.worker_memory
  execution_role_arn       = aws_iam_role.worker_execution.arn
  task_role_arn            = aws_iam_role.worker.arn
  container_definitions = jsonencode([{
    name             = "worker", image = var.worker_image, essential = true, user = "10001", readonlyRootFilesystem = true, stopTimeout = 45,
    command          = ["--worker"], environment = local.runtime_environment, secrets = concat(local.worker_secrets, local.recovery_secrets),
    linuxParameters  = { initProcessEnabled = true },
    logConfiguration = { logDriver = "awslogs", options = { awslogs-group = aws_cloudwatch_log_group.application["worker"].name, awslogs-region = var.aws_region, awslogs-stream-prefix = "ecs" } }
  }])
}

resource "aws_ecs_task_definition" "publisher" {
  family                   = "${local.name}-publisher"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.worker_cpu
  memory                   = var.worker_memory
  execution_role_arn       = aws_iam_role.publisher_execution.arn
  task_role_arn            = aws_iam_role.publisher.arn
  container_definitions = jsonencode([{
    name             = "publisher", image = var.worker_image, essential = true, user = "10001", readonlyRootFilesystem = true, stopTimeout = 45,
    command          = ["--public-asset-publisher"], environment = local.runtime_environment, secrets = local.publisher_secrets,
    linuxParameters  = { initProcessEnabled = true },
    logConfiguration = { logDriver = "awslogs", options = { awslogs-group = aws_cloudwatch_log_group.application["publisher"].name, awslogs-region = var.aws_region, awslogs-stream-prefix = "ecs" } }
  }])
}

resource "aws_ecs_service" "api" {
  name                              = "api"
  cluster                           = aws_ecs_cluster.main.id
  task_definition                   = aws_ecs_task_definition.api.arn
  desired_count                     = var.api_desired_count
  launch_type                       = "FARGATE"
  platform_version                  = var.fargate_platform_version
  health_check_grace_period_seconds = 60
  network_configuration {
    subnets          = values(aws_subnet.private)[*].id
    security_groups  = [aws_security_group.api.id]
    assign_public_ip = false
  }
  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = 3000
  }
  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }
}
resource "aws_ecs_service" "worker" {
  name             = "worker"
  cluster          = aws_ecs_cluster.main.id
  task_definition  = aws_ecs_task_definition.worker.arn
  desired_count    = var.worker_desired_count
  launch_type      = "FARGATE"
  platform_version = var.fargate_platform_version
  network_configuration {
    subnets          = values(aws_subnet.private)[*].id
    security_groups  = [aws_security_group.worker.id]
    assign_public_ip = false
  }
  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }
}
resource "aws_ecs_service" "publisher" {
  name             = "publisher"
  cluster          = aws_ecs_cluster.main.id
  task_definition  = aws_ecs_task_definition.publisher.arn
  desired_count    = var.publisher_desired_count
  launch_type      = "FARGATE"
  platform_version = var.fargate_platform_version
  network_configuration {
    subnets          = values(aws_subnet.private)[*].id
    security_groups  = [aws_security_group.publisher.id]
    assign_public_ip = false
  }
  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }
}

resource "aws_appautoscaling_target" "api" {
  max_capacity       = var.api_max_count
  min_capacity       = 2
  resource_id        = "service/${aws_ecs_cluster.main.name}/${aws_ecs_service.api.name}"
  scalable_dimension = "ecs:service:DesiredCount"
  service_namespace  = "ecs"
}
resource "aws_appautoscaling_policy" "api_cpu" {
  name               = "cpu"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.api.resource_id
  scalable_dimension = aws_appautoscaling_target.api.scalable_dimension
  service_namespace  = aws_appautoscaling_target.api.service_namespace
  target_tracking_scaling_policy_configuration {
    target_value = 60
    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
  }
}

resource "aws_appautoscaling_target" "worker" {
  max_capacity       = var.worker_max_count
  min_capacity       = 1
  resource_id        = "service/${aws_ecs_cluster.main.name}/${aws_ecs_service.worker.name}"
  scalable_dimension = "ecs:service:DesiredCount"
  service_namespace  = "ecs"
}
resource "aws_appautoscaling_policy" "worker_cpu" {
  name               = "worker-cpu"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.worker.resource_id
  scalable_dimension = aws_appautoscaling_target.worker.scalable_dimension
  service_namespace  = aws_appautoscaling_target.worker.service_namespace
  target_tracking_scaling_policy_configuration {
    target_value = 60
    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
  }
}
