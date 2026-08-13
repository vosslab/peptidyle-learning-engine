output "cloudfront_distribution_id" {
  value       = aws_cloudfront_distribution.main.id
  description = "Use only for post-deploy invalidation of reviewed, immutable static manifests."
}
output "cloudfront_domain_name" { value = aws_cloudfront_distribution.main.domain_name }
output "public_assets_bucket" { value = aws_s3_bucket.object["public_assets"].bucket }
output "private_content_bucket" { value = aws_s3_bucket.object["private_content"].bucket }
output "student_records_bucket" { value = aws_s3_bucket.object["student_records"].bucket }
output "temp_processing_bucket" { value = aws_s3_bucket.object["temp_processing"].bucket }
output "application_task_role_arns" {
  value = { api = aws_iam_role.api.arn, worker = aws_iam_role.worker.arn, publisher = aws_iam_role.publisher.arn }
}
output "rds_master_secret_arn" {
  value       = try(aws_db_instance.postgres.master_user_secret[0].secret_arn, null)
  sensitive   = true
  description = "Administrative recovery-only secret; it is not an application credential."
}
