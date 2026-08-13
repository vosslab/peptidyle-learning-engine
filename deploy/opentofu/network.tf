data "aws_ec2_managed_prefix_list" "cloudfront_origin_facing" {
  name = "com.amazonaws.global.cloudfront.origin-facing"
}

resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_hostnames = true
  enable_dns_support   = true
}

resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id
}

resource "aws_subnet" "public" {
  for_each                = toset(var.availability_zones)
  vpc_id                  = aws_vpc.main.id
  availability_zone       = each.value
  cidr_block              = local.public_subnet_cidrs[index(var.availability_zones, each.value)]
  map_public_ip_on_launch = false
}

resource "aws_subnet" "private" {
  for_each          = toset(var.availability_zones)
  vpc_id            = aws_vpc.main.id
  availability_zone = each.value
  cidr_block        = local.private_subnet_cidrs[index(var.availability_zones, each.value)]
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id
}
resource "aws_route" "public_internet" {
  route_table_id         = aws_route_table.public.id
  destination_cidr_block = "0.0.0.0/0"
  gateway_id             = aws_internet_gateway.main.id
}
resource "aws_route_table_association" "public" {
  for_each       = aws_subnet.public
  subnet_id      = each.value.id
  route_table_id = aws_route_table.public.id
}

# Private tasks intentionally have no NAT default route. AWS service access is only through VPC endpoints.
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id
}
resource "aws_route_table_association" "private" {
  for_each       = aws_subnet.private
  subnet_id      = each.value.id
  route_table_id = aws_route_table.private.id
}

resource "aws_security_group" "vpce" {
  name        = "${local.name}-vpce"
  description = "TLS from private tasks to interface endpoints"
  vpc_id      = aws_vpc.main.id
  egress      = []
}

resource "aws_vpc_endpoint" "s3" {
  vpc_id            = aws_vpc.main.id
  service_name      = "com.amazonaws.${var.aws_region}.s3"
  vpc_endpoint_type = "Gateway"
  route_table_ids   = [aws_route_table.private.id]
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { AWS = [aws_iam_role.api.arn, aws_iam_role.worker.arn, aws_iam_role.publisher.arn] }
      Action    = ["s3:GetBucketLocation", "s3:GetObject", "s3:GetObjectTagging", "s3:PutObject", "s3:PutObjectTagging", "s3:DeleteObject", "s3:AbortMultipartUpload"]
      Resource = concat(
        [for bucket in aws_s3_bucket.object : bucket.arn],
        [for bucket in aws_s3_bucket.object : "${bucket.arn}/*"]
      )
    }]
  })
}

resource "aws_vpc_endpoint" "interface" {
  for_each            = toset(["ecr.api", "ecr.dkr", "logs", "secretsmanager", "kms", "sts"])
  vpc_id              = aws_vpc.main.id
  service_name        = "com.amazonaws.${var.aws_region}.${each.value}"
  vpc_endpoint_type   = "Interface"
  private_dns_enabled = true
  subnet_ids          = values(aws_subnet.private)[*].id
  security_group_ids  = [aws_security_group.vpce.id]
}

resource "aws_security_group" "alb" {
  name        = "${local.name}-alb"
  description = "Only CloudFront origin-facing addresses may reach the public ALB."
  vpc_id      = aws_vpc.main.id
  egress      = []
}

resource "aws_security_group" "api" {
  name        = "${local.name}-api"
  description = "API receives traffic only through the ALB."
  vpc_id      = aws_vpc.main.id
  egress      = []
}

resource "aws_security_group" "worker" {
  name        = "${local.name}-worker"
  description = "Worker has no ingress and can reach only RDS and AWS VPC endpoints."
  vpc_id      = aws_vpc.main.id
  egress      = []
}
resource "aws_security_group" "publisher" {
  name        = "${local.name}-publisher"
  description = "Dedicated public-asset publisher: no ingress, only RDS and AWS endpoints."
  vpc_id      = aws_vpc.main.id
  egress      = []
}

resource "aws_security_group" "database" {
  name        = "${local.name}-database"
  description = "PostgreSQL accepts TLS clients only from API and worker tasks."
  vpc_id      = aws_vpc.main.id
  egress      = []
}

resource "aws_vpc_security_group_ingress_rule" "vpce_from_api" {
  security_group_id            = aws_security_group.vpce.id
  referenced_security_group_id = aws_security_group.api.id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_ingress_rule" "vpce_from_worker" {
  security_group_id            = aws_security_group.vpce.id
  referenced_security_group_id = aws_security_group.worker.id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_ingress_rule" "vpce_from_publisher" {
  security_group_id            = aws_security_group.vpce.id
  referenced_security_group_id = aws_security_group.publisher.id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_egress_rule" "vpce_aws_service" {
  security_group_id = aws_security_group.vpce.id
  ip_protocol       = "-1"
  cidr_ipv4         = "0.0.0.0/0"
}
resource "aws_vpc_security_group_ingress_rule" "alb_from_cloudfront" {
  security_group_id = aws_security_group.alb.id
  ip_protocol       = "tcp"
  from_port         = 443
  to_port           = 443
  prefix_list_id    = data.aws_ec2_managed_prefix_list.cloudfront_origin_facing.id
}
resource "aws_vpc_security_group_egress_rule" "alb_to_api" {
  security_group_id            = aws_security_group.alb.id
  referenced_security_group_id = aws_security_group.api.id
  ip_protocol                  = "tcp"
  from_port                    = 3000
  to_port                      = 3000
}
resource "aws_vpc_security_group_ingress_rule" "api_from_alb" {
  security_group_id            = aws_security_group.api.id
  referenced_security_group_id = aws_security_group.alb.id
  ip_protocol                  = "tcp"
  from_port                    = 3000
  to_port                      = 3000
}
resource "aws_vpc_security_group_egress_rule" "api_to_database" {
  security_group_id            = aws_security_group.api.id
  referenced_security_group_id = aws_security_group.database.id
  ip_protocol                  = "tcp"
  from_port                    = 5432
  to_port                      = 5432
}
resource "aws_vpc_security_group_egress_rule" "api_to_aws" {
  security_group_id            = aws_security_group.api.id
  referenced_security_group_id = aws_security_group.vpce.id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_egress_rule" "api_to_imathas" {
  count                        = var.enable_imathas ? 1 : 0
  security_group_id            = aws_security_group.api.id
  referenced_security_group_id = var.imathas_security_group_id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_egress_rule" "api_to_renderer" {
  count                        = var.enable_webwork ? 1 : 0
  security_group_id            = aws_security_group.api.id
  referenced_security_group_id = var.renderer_security_group_id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_egress_rule" "api_to_smtp" {
  count                        = var.enable_smtp ? 1 : 0
  security_group_id            = aws_security_group.api.id
  referenced_security_group_id = var.smtp_security_group_id
  ip_protocol                  = "tcp"
  from_port                    = 587
  to_port                      = 587
}
resource "aws_vpc_security_group_egress_rule" "worker_to_database" {
  security_group_id            = aws_security_group.worker.id
  referenced_security_group_id = aws_security_group.database.id
  ip_protocol                  = "tcp"
  from_port                    = 5432
  to_port                      = 5432
}
resource "aws_vpc_security_group_egress_rule" "worker_to_aws" {
  security_group_id            = aws_security_group.worker.id
  referenced_security_group_id = aws_security_group.vpce.id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_egress_rule" "publisher_to_database" {
  security_group_id            = aws_security_group.publisher.id
  referenced_security_group_id = aws_security_group.database.id
  ip_protocol                  = "tcp"
  from_port                    = 5432
  to_port                      = 5432
}
resource "aws_vpc_security_group_egress_rule" "publisher_to_aws" {
  security_group_id            = aws_security_group.publisher.id
  referenced_security_group_id = aws_security_group.vpce.id
  ip_protocol                  = "tcp"
  from_port                    = 443
  to_port                      = 443
}
resource "aws_vpc_security_group_ingress_rule" "database_from_api" {
  security_group_id            = aws_security_group.database.id
  referenced_security_group_id = aws_security_group.api.id
  ip_protocol                  = "tcp"
  from_port                    = 5432
  to_port                      = 5432
}
resource "aws_vpc_security_group_ingress_rule" "database_from_worker" {
  security_group_id            = aws_security_group.database.id
  referenced_security_group_id = aws_security_group.worker.id
  ip_protocol                  = "tcp"
  from_port                    = 5432
  to_port                      = 5432
}
resource "aws_vpc_security_group_ingress_rule" "database_from_publisher" {
  security_group_id            = aws_security_group.database.id
  referenced_security_group_id = aws_security_group.publisher.id
  ip_protocol                  = "tcp"
  from_port                    = 5432
  to_port                      = 5432
}
