resource "aws_security_group" "redis" {
  name        = "redis-sg"
  description = "allow inbound access only from the ECS tasks"
  vpc_id      = aws_vpc.main.id
  tags        = var.tags

  ingress {
    protocol        = "tcp"
    from_port       = var.redis_port
    to_port         = var.redis_port
    cidr_blocks     = ["0.0.0.0/0"]
    security_groups = [aws_security_group.ecs_tasks.id]
  }

  egress {
    protocol    = "-1"
    from_port   = 0
    to_port     = 0
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_elasticache_subnet_group" "default" {
  name        = "redis-cache-subnet"
  description = "subnet to run the cluster in"
  subnet_ids  = aws_subnet.public.*.id
  tags        = var.tags
}

resource "aws_elasticache_replication_group" "redis" {
  replication_group_id          = "redis"
  replication_group_description = "redis cluster"

  automatic_failover_enabled = var.redis_redundancy
  multi_az_enabled           = var.redis_redundancy
  number_cache_clusters      = var.redis_replica_nodes

  node_type            = "cache.t3.micro"
  parameter_group_name = "default.redis7"
  port                 = var.redis_port
  apply_immediately    = true

  tags               = var.tags
  subnet_group_name  = aws_elasticache_subnet_group.default.name
  security_group_ids = [aws_security_group.redis.id]
  depends_on         = [aws_subnet.public]
}
