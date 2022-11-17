resource "aws_route53_record" "alb" {
  name = "beta.www"
  type = "CNAME"

  records = [
    aws_lb.alb.dns_name,
  ]

  zone_id = var.arn_r53_zone
  ttl     = "60"
}
