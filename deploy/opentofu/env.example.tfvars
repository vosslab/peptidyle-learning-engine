# Copy outside the repository. Use a unique deployment_id for every disposable deployment exercise.
aws_region               = "us-west-2"
environment              = "staging"
deployment_id            = "replace-me"
domain_name              = "learn.example.edu"
certificate_arn          = "arn:aws:acm:us-east-1:111122223333:certificate/REPLACE"
origin_certificate_arn   = "arn:aws:acm:us-west-2:111122223333:certificate/REPLACE"
origin_domain_name       = "origin.learn.example.edu"
origin_hosted_zone_id    = "ZREPLACE"
vpc_cidr                 = "10.42.0.0/20"
availability_zones       = ["us-west-2a", "us-west-2b"]
api_image                = "111122223333.dkr.ecr.us-west-2.amazonaws.com/ple-api@sha256:REPLACE_WITH_64_LOWERCASE_HEX"
worker_image             = "111122223333.dkr.ecr.us-west-2.amazonaws.com/ple-worker@sha256:REPLACE_WITH_64_LOWERCASE_HEX"
secret_file_writer_image = "111122223333.dkr.ecr.us-west-2.amazonaws.com/ple-secret-file-writer@sha256:REPLACE_WITH_64_LOWERCASE_HEX"

# These are ARNs only. Secret values must be created and rotated outside OpenTofu.
api_application_secrets_arn               = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-api-runtime-REPLACE"
worker_application_secrets_arn            = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-worker-runtime-REPLACE"
recovery_application_secrets_arn          = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-accepted-submission-recovery-REPLACE"
fast_path_application_secrets_arn         = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-accepted-submission-fast-path-REPLACE"
api_application_secrets_kms_key_arn       = "arn:aws:kms:us-west-2:111122223333:key/REPLACE"
worker_application_secrets_kms_key_arn    = "arn:aws:kms:us-west-2:111122223333:key/REPLACE"
recovery_application_secrets_kms_key_arn  = "arn:aws:kms:us-west-2:111122223333:key/REPLACE"
fast_path_application_secrets_kms_key_arn = "arn:aws:kms:us-west-2:111122223333:key/REPLACE"
publisher_application_secrets_arn         = "arn:aws:secretsmanager:us-west-2:111122223333:secret:ple-publisher-runtime-REPLACE"
publisher_application_secrets_kms_key_arn = "arn:aws:kms:us-west-2:111122223333:key/REPLACE"
smtp_security_group_id                    = "sg-REPLACE"
imathas_security_group_id                 = "sg-REPLACE"
renderer_security_group_id                = "sg-REPLACE"
alert_email                               = "operations@example.edu"

# Enable optional integrations only after every named JSON key in compute.tf
# has been provisioned in application_secrets_arn.
enable_imathas = false
enable_smtp    = true
enable_webwork = false
