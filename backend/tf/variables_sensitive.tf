variable "access_key" {
  type      = string
  sensitive = true
}

variable "secret_key" {
  type      = string
  sensitive = true
}

variable "aws_id" {
  type      = string
  sensitive = true
}

variable "tinyurl_token" {
  type      = string
  sensitive = true
}

variable "alb_certificate_arn" {
  type      = string
  sensitive = true
}
variable "ddb_table_arn" {
  type      = string
  sensitive = true
}

variable "mj_template_id" {
  type      = string
  sensitive = true
}
variable "mj_key" {
  type      = string
  sensitive = true
}
variable "mj_secret" {
  type      = string
  sensitive = true
}

variable "arn_r53_zone" {
  type      = string
  sensitive = true
}

variable "sentry_dsn" {
  type      = string
  sensitive = true
}
