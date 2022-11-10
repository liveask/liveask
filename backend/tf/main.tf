terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 3.22"
    }
  }
}

provider "aws" {
  allowed_account_ids = [var.aws_id]
  region              = var.region
  access_key          = var.access_key
  secret_key          = var.secret_key
}
