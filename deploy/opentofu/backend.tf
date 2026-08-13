# State storage is a separately bootstrapped security boundary. `tofu init` in a real account must
# supply bucket, key, region, and a customer-managed `kms_key_id` through a reviewed backend config.
# These secure defaults prevent an accidental local mutable state workflow when real backend
# coordinates are supplied; the deploy identity must separately deny unencrypted state writes.
terraform {
  backend "s3" {
    encrypt      = true
    use_lockfile = true
  }
}
