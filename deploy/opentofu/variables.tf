variable "aws_region" {
  type = string
}
variable "environment" {
  type = string
  validation {
    condition     = contains(["staging", "production"], var.environment)
    error_message = "environment must be staging or production."
  }
}
variable "deployment_id" {
  type        = string
  description = "Unique, disposable deployment identifier; never reuse it across environments."
  validation {
    condition     = can(regex("^[a-z0-9-]{3,32}$", var.deployment_id))
    error_message = "deployment_id must be 3-32 lowercase letters, digits, or hyphens."
  }
}
variable "domain_name" {
  type        = string
  description = "Canonical browser host, for example learn.example.edu."
}
variable "certificate_arn" {
  type        = string
  description = "ACM certificate in us-east-1 for the CloudFront viewer domain."
}
variable "origin_certificate_arn" {
  type        = string
  description = "Regional ACM certificate whose SAN covers origin_domain_name; CloudFront verifies this name when connecting to the ALB origin."
}
variable "origin_domain_name" {
  type        = string
  description = "Deployment-controlled DNS name which aliases the ALB and is covered by origin_certificate_arn. CloudFront uses it for TLS SNI while forwarding domain_name as Host."
}
variable "origin_hosted_zone_id" {
  type        = string
  description = "Route 53 hosted zone ID authoritative for origin_domain_name."
}
variable "vpc_cidr" {
  type = string
}
variable "availability_zones" {
  type = list(string)
  validation {
    condition     = length(var.availability_zones) >= 2
    error_message = "At least two availability zones are required."
  }
}
variable "api_image" {
  type        = string
  description = "Immutable API OCI image reference including @sha256 digest."
  validation {
    condition     = can(regex("@sha256:[0-9a-f]{64}$", var.api_image))
    error_message = "api_image must be digest pinned."
  }
}
variable "worker_image" {
  type        = string
  description = "Immutable worker OCI image reference including @sha256 digest."
  validation {
    condition     = can(regex("@sha256:[0-9a-f]{64}$", var.worker_image))
    error_message = "worker_image must be digest pinned."
  }
}
variable "secret_file_writer_image" {
  type        = string
  description = "Reviewed, digest-pinned secret-file writer that writes exactly smtp-password and invitation-token under PLE_SECRET_OUTPUT_DIR, mode 0600, then exits successfully."
  validation {
    condition     = can(regex("@sha256:[0-9a-f]{64}$", var.secret_file_writer_image))
    error_message = "secret_file_writer_image must be digest pinned."
  }
}
variable "api_cpu" {
  type    = number
  default = 1024
}
variable "api_memory" {
  type    = number
  default = 2048
}
variable "worker_cpu" {
  type    = number
  default = 1024
}
variable "worker_memory" {
  type    = number
  default = 2048
}
variable "api_desired_count" {
  type    = number
  default = 2
}
variable "api_max_count" {
  type    = number
  default = 8
}
variable "worker_desired_count" {
  type    = number
  default = 1
}
variable "worker_max_count" {
  type    = number
  default = 4
}
variable "fargate_platform_version" {
  type        = string
  description = "Reviewed Fargate platform version. Advance only after disposable workload and drain evidence."
  default     = "1.4.0"
}
variable "database_instance_class" {
  type    = string
  default = "db.t4g.medium"
}
variable "database_name" {
  type    = string
  default = "peptidyle"
}
variable "database_master_username" {
  type    = string
  default = "ple_admin"
}
variable "rds_ca_cert_identifier" {
  type        = string
  description = "Reviewed regional RDS CA identifier; its root must be bundled in the immutable application image before rollout."
  default     = "rds-ca-rsa2048-g1"
}
variable "api_application_secrets_arn" {
  type        = string
  description = "Existing API-only Secrets Manager JSON ARN. It contains no worker credentials."
}
variable "api_application_secrets_kms_key_arn" {
  type        = string
  description = "CMK ARN encrypting api_application_secrets_arn; its policy permits only the API execution role via Secrets Manager."
}
variable "worker_application_secrets_arn" {
  type        = string
  description = "Existing worker-only Secrets Manager JSON ARN. It contains no API/browser or SMTP credentials."
}
variable "worker_application_secrets_kms_key_arn" {
  type        = string
  description = "CMK ARN encrypting worker_application_secrets_arn; its policy permits only the worker execution role through Secrets Manager."
}
variable "recovery_application_secrets_arn" {
  type        = string
  description = "Existing accepted-submission recovery-only Secrets Manager JSON ARN. It contains only PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL for the dedicated recovery login."
}
variable "recovery_application_secrets_kms_key_arn" {
  type        = string
  description = "CMK ARN encrypting recovery_application_secrets_arn; its policy permits only the worker execution role through Secrets Manager."
}
variable "fast_path_application_secrets_arn" {
  type        = string
  description = "Existing accepted-submission fast-path-only Secrets Manager JSON ARN. It contains only PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL for the dedicated fast-path login."
}
variable "fast_path_application_secrets_kms_key_arn" {
  type        = string
  description = "CMK ARN encrypting fast_path_application_secrets_arn; its policy permits only the API execution role through Secrets Manager."
}
variable "publisher_application_secrets_arn" {
  type        = string
  description = "Existing publisher-only Secrets Manager JSON ARN containing only PLE_PUBLISHER_DATABASE_URL."
}
variable "publisher_application_secrets_kms_key_arn" {
  type        = string
  description = "CMK ARN encrypting publisher_application_secrets_arn; its policy permits only the publisher execution role via Secrets Manager."
}
variable "smtp_security_group_id" {
  type        = string
  default     = null
  description = "Private SMTP relay security group; API egress is restricted to this group on 587."
}
variable "imathas_security_group_id" {
  type        = string
  default     = null
  description = "Private iMathAS service security group; API egress is restricted to HTTPS."
}
variable "renderer_security_group_id" {
  type        = string
  default     = null
  description = "Private WebWork renderer service security group; API egress is restricted to HTTPS."
}
variable "alert_email" {
  type = string
}
variable "enable_imathas" {
  type    = bool
  default = false
}
variable "enable_smtp" {
  type    = bool
  default = true
}
variable "enable_webwork" {
  type    = bool
  default = false
}
variable "publisher_desired_count" {
  type    = number
  default = 1
}
variable "waf_rate_limit" {
  type    = number
  default = 2000
}
