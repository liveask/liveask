output "alb_dns_name" {
  description = "The DNS name of the load balancer."
  value       = concat(aws_lb.alb.*.dns_name, [""])[0]
}
