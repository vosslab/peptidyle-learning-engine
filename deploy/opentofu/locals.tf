locals {
  name = "ple-${var.environment}-${var.deployment_id}"
  tags = {
    Application  = "peptidyle-learning-engine"
    Environment  = var.environment
    DeploymentId = var.deployment_id
    ManagedBy    = "opentofu"
    DataClass    = "educational-record"
  }

  private_subnet_cidrs = [for index, _ in var.availability_zones : cidrsubnet(var.vpc_cidr, 4, index)]
  public_subnet_cidrs  = [for index, _ in var.availability_zones : cidrsubnet(var.vpc_cidr, 4, index + 8)]
  bucket_names = {
    public_assets   = "${local.name}-public-assets"
    private_content = "${local.name}-private-content"
    student_records = "${local.name}-student-records"
    temp_processing = "${local.name}-temp-processing"
  }
}
