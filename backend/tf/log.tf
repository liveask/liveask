resource "aws_cloudwatch_log_group" "api" {
  name = "api"
  tags = var.tags
}
