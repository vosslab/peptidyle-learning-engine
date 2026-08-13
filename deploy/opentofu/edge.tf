resource "aws_lb" "api" {
  name                       = "${local.name}-edge"
  internal                   = false
  load_balancer_type         = "application"
  security_groups            = [aws_security_group.alb.id]
  subnets                    = values(aws_subnet.public)[*].id
  drop_invalid_header_fields = true
  enable_deletion_protection = true
}

resource "aws_route53_record" "origin" {
  zone_id = var.origin_hosted_zone_id
  name    = var.origin_domain_name
  type    = "A"
  alias {
    name                   = aws_lb.api.dns_name
    zone_id                = aws_lb.api.zone_id
    evaluate_target_health = true
  }
}

# OAC uses this separate policy update after the distribution exists. It deliberately retains the
# S3-only and exact-bucket denial statements from the key's creation policy.
resource "aws_kms_key_policy" "public_assets_cloudfront" {
  key_id = aws_kms_key.object["public_assets"].key_id
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
        Condition = { StringNotLike = { "kms:EncryptionContext:aws:s3:arn" = ["arn:aws:s3:::${local.bucket_names["public_assets"]}", "arn:aws:s3:::${local.bucket_names["public_assets"]}/*"] } }
      },
      {
        Sid       = "CloudFrontDecryptPublicAssets"
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = ["kms:Decrypt", "kms:DescribeKey"]
        Resource  = "*"
        Condition = { StringEquals = { "AWS:SourceArn" = aws_cloudfront_distribution.main.arn } }
      }
    ]
  })
}

resource "aws_lb_target_group" "api" {
  name                 = "${local.name}-api"
  port                 = 3000
  protocol             = "HTTP"
  target_type          = "ip"
  vpc_id               = aws_vpc.main.id
  deregistration_delay = 45
  health_check {
    enabled             = true
    path                = "/health"
    protocol            = "HTTP"
    matcher             = "200"
    healthy_threshold   = 2
    unhealthy_threshold = 2
  }
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.api.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.origin_certificate_arn
  default_action {
    type = "fixed-response"
    fixed_response {
      content_type = "text/plain"
      message_body = "origin access denied"
      status_code  = "403"
    }
  }
}

# The value is secret in state and never an output. CloudFront overwrites this origin custom
# header, and the ALB otherwise returns 403, so a direct origin request cannot reach the API.
resource "random_password" "cloudfront_origin_verify" {
  length  = 48
  special = false
}

resource "aws_lb_listener_rule" "cloudfront_only" {
  listener_arn = aws_lb_listener.https.arn
  priority     = 10
  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }
  condition {
    http_header {
      http_header_name = "X-PLE-Origin-Verify"
      values           = [random_password.cloudfront_origin_verify.result]
    }
  }
}

resource "aws_cloudfront_origin_access_control" "public_assets" {
  name                              = "${local.name}-public-assets"
  description                       = "CloudFront-only read access to public assets"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

resource "aws_cloudfront_distribution" "main" {
  enabled             = true
  is_ipv6_enabled     = true
  comment             = "${local.name} same-origin edge"
  aliases             = [var.domain_name]
  default_root_object = "index.html"
  web_acl_id          = aws_wafv2_web_acl.edge.arn

  origin {
    domain_name              = aws_s3_bucket.object["public_assets"].bucket_regional_domain_name
    origin_id                = "public-assets"
    origin_access_control_id = aws_cloudfront_origin_access_control.public_assets.id
  }
  origin {
    # This controlled alias is covered by the regional origin certificate.
    # CloudFront's AllViewer policy still forwards the canonical viewer Host
    # to the application after TLS validation against this origin name.
    domain_name = var.origin_domain_name
    origin_id   = "api"
    # CloudFront overwrites viewer input of this name with the state-only
    # secret. ALB will forward only a request carrying this exact value.
    custom_header {
      name  = "X-PLE-Origin-Verify"
      value = random_password.cloudfront_origin_verify.result
    }
    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  default_cache_behavior {
    target_origin_id           = "public-assets"
    viewer_protocol_policy     = "redirect-to-https"
    allowed_methods            = ["GET", "HEAD", "OPTIONS"]
    cached_methods             = ["GET", "HEAD"]
    compress                   = true
    cache_policy_id            = data.aws_cloudfront_cache_policy.caching_optimized.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.browser.id
  }
  # CloudFront's `/api/*` pattern does not match `/api` itself. Keep that
  # exact API root on the dynamic origin and API policy as well, rather than
  # letting it fall through to the static public-assets origin.
  ordered_cache_behavior {
    path_pattern               = "/api"
    target_origin_id           = "api"
    viewer_protocol_policy     = "https-only"
    allowed_methods            = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods             = ["GET", "HEAD"]
    compress                   = true
    cache_policy_id            = data.aws_cloudfront_cache_policy.caching_disabled.id
    origin_request_policy_id   = data.aws_cloudfront_origin_request_policy.all_viewer.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.api.id
  }
  ordered_cache_behavior {
    path_pattern               = "/api/*"
    target_origin_id           = "api"
    viewer_protocol_policy     = "https-only"
    allowed_methods            = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods             = ["GET", "HEAD"]
    compress                   = true
    cache_policy_id            = data.aws_cloudfront_cache_policy.caching_disabled.id
    origin_request_policy_id   = data.aws_cloudfront_origin_request_policy.all_viewer.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.api.id
  }
  ordered_cache_behavior {
    path_pattern               = "/health"
    target_origin_id           = "api"
    viewer_protocol_policy     = "https-only"
    allowed_methods            = ["GET", "HEAD"]
    cached_methods             = ["GET", "HEAD"]
    cache_policy_id            = data.aws_cloudfront_cache_policy.caching_disabled.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.browser.id
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }
  viewer_certificate {
    acm_certificate_arn      = var.certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }
}

data "aws_cloudfront_cache_policy" "caching_optimized" {
  name = "Managed-CachingOptimized"
}
data "aws_cloudfront_cache_policy" "caching_disabled" {
  name = "Managed-CachingDisabled"
}
data "aws_cloudfront_origin_request_policy" "all_viewer" {
  name = "Managed-AllViewer"
}

resource "aws_cloudfront_response_headers_policy" "browser" {
  name = "${local.name}-browser-security"
  security_headers_config {
    content_type_options {
      override = true
    }
    frame_options {
      frame_option = "SAMEORIGIN"
      override     = true
    }
    referrer_policy {
      referrer_policy = "no-referrer"
      override        = true
    }
    strict_transport_security {
      access_control_max_age_sec = 31536000
      include_subdomains         = true
      preload                    = true
      override                   = true
    }
    content_security_policy {
      content_security_policy = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:; base-uri 'none'; form-action 'self'; frame-ancestors 'self'; object-src 'none'"
      override                = true
    }
  }
  custom_headers_config {
    items {
      header   = "Permissions-Policy"
      value    = "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
      override = true
    }
    items {
      header   = "Cross-Origin-Resource-Policy"
      value    = "same-origin"
      override = true
    }
  }
}

# The API constructs nonce-bearing or route-specific CSP itself. The edge owns
# transport and passive browser headers only, so it must never overwrite CSP.
resource "aws_cloudfront_response_headers_policy" "api" {
  name = "${local.name}-api-security"
  security_headers_config {
    content_type_options { override = true }
    frame_options {
      frame_option = "SAMEORIGIN"
      override     = true
    }
    referrer_policy {
      referrer_policy = "no-referrer"
      override        = true
    }
    strict_transport_security {
      access_control_max_age_sec = 31536000
      include_subdomains         = true
      preload                    = true
      override                   = true
    }
  }
  custom_headers_config {
    items {
      header   = "Permissions-Policy"
      value    = "camera=(), microphone=(), geolocation=(), payment=(), usb=()"
      override = true
    }
    items {
      header   = "Cross-Origin-Resource-Policy"
      value    = "same-origin"
      override = true
    }
  }
}
