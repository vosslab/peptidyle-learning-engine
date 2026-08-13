resource "aws_db_subnet_group" "main" {
  name       = "${local.name}-database"
  subnet_ids = values(aws_subnet.private)[*].id
}

resource "aws_cloudwatch_log_group" "postgres" {
  name              = "/ple/${local.name}/rds/postgresql"
  retention_in_days = 365
  kms_key_id        = aws_kms_key.logs.arn
}

resource "aws_db_parameter_group" "postgres" {
  name   = "${local.name}-postgres17"
  family = "postgres17"
  parameter {
    name  = "rds.force_ssl"
    value = "1"
  }
  parameter {
    name  = "log_connections"
    value = "1"
  }
  parameter {
    name  = "log_disconnections"
    value = "1"
  }
}

resource "aws_db_instance" "postgres" {
  identifier                      = "${local.name}-postgres"
  engine                          = "postgres"
  engine_version                  = "17"
  instance_class                  = var.database_instance_class
  allocated_storage               = 100
  max_allocated_storage           = 1000
  storage_type                    = "gp3"
  storage_encrypted               = true
  kms_key_id                      = aws_kms_key.database.arn
  db_name                         = var.database_name
  username                        = var.database_master_username
  manage_master_user_password     = true
  master_user_secret_kms_key_id   = aws_kms_key.secrets.arn
  port                            = 5432
  ca_cert_identifier              = var.rds_ca_cert_identifier
  db_subnet_group_name            = aws_db_subnet_group.main.name
  vpc_security_group_ids          = [aws_security_group.database.id]
  parameter_group_name            = aws_db_parameter_group.postgres.name
  publicly_accessible             = false
  multi_az                        = true
  backup_retention_period         = 35
  backup_window                   = "06:00-06:30"
  maintenance_window              = "sun:07:00-sun:07:30"
  deletion_protection             = true
  skip_final_snapshot             = false
  final_snapshot_identifier       = "${local.name}-final"
  copy_tags_to_snapshot           = true
  performance_insights_enabled    = true
  performance_insights_kms_key_id = aws_kms_key.database.arn
  enabled_cloudwatch_logs_exports = ["postgresql"]
  auto_minor_version_upgrade      = true
  apply_immediately               = false
}
