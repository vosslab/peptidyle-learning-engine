resource "aws_kms_key" "database" {
  description             = "${local.name} encrypted PostgreSQL storage and performance insights"
  enable_key_rotation     = true
  deletion_window_in_days = 30
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      { Sid = "AccountRootAdministration", Effect = "Allow", Principal = { AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root" }, Action = "kms:*", Resource = "*" },
      { Sid = "AllowRdsEncryptedStorage", Effect = "Allow", Principal = { Service = "rds.amazonaws.com" }, Action = ["kms:Encrypt", "kms:Decrypt", "kms:ReEncrypt*", "kms:GenerateDataKey*", "kms:CreateGrant", "kms:DescribeKey"], Resource = "*", Condition = { StringEquals = { "aws:SourceAccount" = data.aws_caller_identity.current.account_id } } }
    ]
  })
}
resource "aws_kms_key" "logs" {
  description             = "${local.name} CloudWatch log encryption"
  enable_key_rotation     = true
  deletion_window_in_days = 30
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      { Sid = "AccountRootAdministration", Effect = "Allow", Principal = { AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root" }, Action = "kms:*", Resource = "*" },
      { Sid = "AllowCloudWatchLogs", Effect = "Allow", Principal = { Service = "logs.${var.aws_region}.amazonaws.com" }, Action = ["kms:Encrypt", "kms:Decrypt", "kms:ReEncrypt*", "kms:GenerateDataKey*", "kms:DescribeKey"], Resource = "*", Condition = { ArnLike = { "kms:EncryptionContext:aws:logs:arn" = "arn:aws:logs:${var.aws_region}:${data.aws_caller_identity.current.account_id}:*" } } },
      { Sid = "AllowSnsNotifications", Effect = "Allow", Principal = { Service = "sns.amazonaws.com" }, Action = ["kms:Decrypt", "kms:GenerateDataKey"], Resource = "*", Condition = { StringEquals = { "aws:SourceAccount" = data.aws_caller_identity.current.account_id } } }
    ]
  })
}
resource "aws_kms_key" "secrets" {
  description             = "${local.name} RDS managed master-secret encryption"
  enable_key_rotation     = true
  deletion_window_in_days = 30
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      { Sid = "AccountRootAdministration", Effect = "Allow", Principal = { AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root" }, Action = "kms:*", Resource = "*" },
      { Sid = "AllowSecretsManagerForRdsMasterSecret", Effect = "Allow", Principal = { Service = "secretsmanager.amazonaws.com" }, Action = ["kms:Encrypt", "kms:Decrypt", "kms:ReEncrypt*", "kms:GenerateDataKey*", "kms:CreateGrant", "kms:DescribeKey"], Resource = "*", Condition = { StringEquals = { "aws:SourceAccount" = data.aws_caller_identity.current.account_id } } }
    ]
  })
}

resource "aws_sns_topic" "security" {
  name              = "${local.name}-security"
  kms_master_key_id = aws_kms_key.logs.arn
}
resource "aws_sns_topic_subscription" "security_email" {
  topic_arn = aws_sns_topic.security.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

resource "aws_cloudwatch_metric_alarm" "rds_cpu" {
  alarm_name          = "${local.name}-rds-cpu"
  alarm_description   = "RDS CPU is saturated; investigate before educational traffic is affected."
  namespace           = "AWS/RDS"
  metric_name         = "CPUUtilization"
  statistic           = "Average"
  period              = 300
  evaluation_periods  = 3
  threshold           = 80
  comparison_operator = "GreaterThanThreshold"
  dimensions          = { DBInstanceIdentifier = aws_db_instance.postgres.id }
  alarm_actions       = [aws_sns_topic.security.arn]
}
resource "aws_cloudwatch_metric_alarm" "rds_storage" {
  alarm_name          = "${local.name}-rds-storage"
  namespace           = "AWS/RDS"
  metric_name         = "FreeStorageSpace"
  statistic           = "Minimum"
  period              = 300
  evaluation_periods  = 1
  threshold           = 21474836480
  comparison_operator = "LessThanThreshold"
  dimensions          = { DBInstanceIdentifier = aws_db_instance.postgres.id }
  alarm_actions       = [aws_sns_topic.security.arn]
}
resource "aws_cloudwatch_metric_alarm" "api_5xx" {
  alarm_name          = "${local.name}-api-5xx"
  namespace           = "AWS/ApplicationELB"
  metric_name         = "HTTPCode_Target_5XX_Count"
  statistic           = "Sum"
  period              = 60
  evaluation_periods  = 5
  threshold           = 5
  comparison_operator = "GreaterThanOrEqualToThreshold"
  dimensions          = { LoadBalancer = aws_lb.api.arn_suffix }
  alarm_actions       = [aws_sns_topic.security.arn]
}
resource "aws_cloudwatch_metric_alarm" "waf_rate" {
  alarm_name          = "${local.name}-waf-rate-observed"
  namespace           = "AWS/WAFV2"
  metric_name         = "CountedRequests"
  statistic           = "Sum"
  period              = 300
  evaluation_periods  = 1
  threshold           = var.waf_rate_limit
  comparison_operator = "GreaterThanOrEqualToThreshold"
  dimensions = {
    WebACL = aws_wafv2_web_acl.edge.name
    Region = "CloudFront"
    Rule   = "RateLimitObservation"
  }
  alarm_actions = [aws_sns_topic.security.arn]
}
