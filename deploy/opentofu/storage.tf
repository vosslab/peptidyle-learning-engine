resource "aws_kms_key" "object" {
  for_each                = local.bucket_names
  description             = "${local.name} ${replace(each.key, "_", " ")} encryption boundary"
  enable_key_rotation     = true
  deletion_window_in_days = 30
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "AccountRootAdministration"
        Effect    = "Allow"
        Principal = { AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root" }
        Action    = "kms:*"
        Resource  = "*"
      },
      {
        Sid       = "DenyDirectKmsUse"
        Effect    = "Deny"
        Principal = "*"
        Action    = ["kms:Decrypt", "kms:GenerateDataKey", "kms:GenerateDataKeyWithoutPlaintext"]
        Resource  = "*"
        Condition = { StringNotEquals = { "kms:ViaService" = "s3.${var.aws_region}.amazonaws.com" } }
      },
      {
        Sid       = "DenyWrongS3EncryptionContext"
        Effect    = "Deny"
        Principal = "*"
        Action    = ["kms:Decrypt", "kms:GenerateDataKey", "kms:GenerateDataKeyWithoutPlaintext"]
        Resource  = "*"
        Condition = { StringNotLike = { "kms:EncryptionContext:aws:s3:arn" = ["arn:aws:s3:::${local.bucket_names[each.key]}", "arn:aws:s3:::${local.bucket_names[each.key]}/*"] } }
      },
    ]
  })
}

resource "aws_kms_alias" "object" {
  for_each      = aws_kms_key.object
  name          = "alias/${local.name}-${replace(each.key, "_", "-")}"
  target_key_id = each.value.key_id
}

resource "aws_s3_bucket" "object" {
  for_each            = local.bucket_names
  bucket              = each.value
  force_destroy       = false
  object_lock_enabled = each.key == "public_assets"
}

resource "aws_s3_bucket_public_access_block" "object" {
  for_each                = aws_s3_bucket.object
  bucket                  = each.value.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_ownership_controls" "object" {
  for_each = aws_s3_bucket.object
  bucket   = each.value.id
  rule { object_ownership = "BucketOwnerEnforced" }
}

resource "aws_s3_bucket_versioning" "object" {
  for_each = aws_s3_bucket.object
  bucket   = each.value.id
  versioning_configuration { status = "Enabled" }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "object" {
  for_each = aws_s3_bucket.object
  bucket   = each.value.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = aws_kms_key.object[each.key].arn
    }
    bucket_key_enabled = true
  }
}

# The public bucket has Object Lock enabled at creation so a future retention requirement can be
# applied per object. There is intentionally no default legal retention: a disposable deployment
# must be destroyable. The active append-only guarantee is the immutable-tag bucket policy below;
# production operators may add a reviewed retention rule only after accepting its recovery impact.

resource "aws_s3_bucket_lifecycle_configuration" "object" {
  for_each = aws_s3_bucket.object
  bucket   = each.value.id
  rule {
    id     = "abort-incomplete-multipart-uploads"
    status = "Enabled"
    filter {}
    abort_incomplete_multipart_upload { days_after_initiation = 1 }
  }
  dynamic "rule" {
    for_each = each.key == "temp_processing" ? [1] : []
    content {
      id     = "expire-temporary-processing"
      status = "Enabled"
      filter {}
      expiration { days = 7 }
      noncurrent_version_expiration { noncurrent_days = 14 }
    }
  }
  dynamic "rule" {
    for_each = each.key == "student_records" ? [1] : []
    content {
      id     = "retain-student-record-history"
      status = "Enabled"
      filter {}
      noncurrent_version_expiration { noncurrent_days = 365 }
    }
  }
}

resource "aws_s3_bucket_policy" "object" {
  for_each = aws_s3_bucket.object
  bucket   = each.value.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "DenyInsecureTransport"
        Effect    = "Deny"
        Principal = "*"
        Action    = "s3:*"
        Resource  = [each.value.arn, "${each.value.arn}/*"]
        Condition = { Bool = { "aws:SecureTransport" = "false" } }
      },
      {
        Sid       = "DenyMissingOrWrongKmsKey"
        Effect    = "Deny"
        Principal = "*"
        Action    = ["s3:PutObject", "s3:InitiateMultipartUpload"]
        Resource  = "${each.value.arn}/*"
        Condition = { StringNotEquals = { "s3:x-amz-server-side-encryption-aws-kms-key-id" = aws_kms_key.object[each.key].arn } }
      },
      {
        Sid       = "DenyNonKmsWrites"
        Effect    = "Deny"
        Principal = "*"
        Action    = ["s3:PutObject", "s3:InitiateMultipartUpload"]
        Resource  = "${each.value.arn}/*"
        Condition = { StringNotEquals = { "s3:x-amz-server-side-encryption" = "aws:kms" } }
      },
      {
        Sid       = "CloudFrontReadOnlyPublicAssetsOnly"
        Effect    = each.key == "public_assets" ? "Allow" : "Deny"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = "s3:GetObject"
        Resource  = "${each.value.arn}/*"
        Condition = { StringEquals = { "AWS:SourceArn" = aws_cloudfront_distribution.main.arn, "s3:ExistingObjectTag/ple-published-immutable" = "true" } }
      },
      {
        Sid       = "DenyUntaggablePublishedAssetWrites"
        Effect    = "Deny"
        Principal = "*"
        Action    = "s3:PutObject"
        Resource  = each.key == "public_assets" ? "${each.value.arn}/questions/*/versions/*/assets/*" : "${each.value.arn}/__no-public-immutability-policy__/*"
        Condition = { StringNotEquals = { "s3:RequestObjectTag/ple-published-immutable" = "true" } }
      },
      {
        Sid       = "DenyPublishedAssetOverwrite"
        Effect    = "Deny"
        Principal = "*"
        Action    = "s3:PutObject"
        Resource  = each.key == "public_assets" ? "${each.value.arn}/questions/*/versions/*/assets/*" : "${each.value.arn}/__no-public-immutability-policy__/*"
        Condition = { StringEquals = { "s3:ExistingObjectTag/ple-published-immutable" = "true" } }
      },
      {
        Sid       = "DenyRuntimeDeleteOfPublishedAssets"
        Effect    = "Deny"
        Principal = "*"
        Action    = ["s3:DeleteObject", "s3:DeleteObjectVersion", "s3:DeleteObjectTagging"]
        Resource  = each.key == "public_assets" ? "${each.value.arn}/questions/*/versions/*/assets/*" : "${each.value.arn}/__no-public-immutability-policy__/*"
        Condition = { StringLike = { "aws:PrincipalArn" = ["arn:aws:iam::*:role/${local.name}-api", "arn:aws:iam::*:role/${local.name}-worker"] } }
      },
      {
        Sid       = "DenyImmutablePublishedAssetRetag"
        Effect    = "Deny"
        Principal = "*"
        Action    = ["s3:PutObjectTagging", "s3:DeleteObjectTagging"]
        Resource  = each.key == "public_assets" ? "${each.value.arn}/questions/*/versions/*/assets/*" : "${each.value.arn}/__no-public-immutability-policy__/*"
        Condition = { StringEquals = { "s3:ExistingObjectTag/ple-published-immutable" = "true" } }
      }
    ]
  })
}
