resource "aws_security_group" "ecs_tasks" {
  name        = "ecs-tasks-sg"
  description = "allow inbound access only from the ALB to the ECS tasks"
  vpc_id      = aws_vpc.main.id
  tags        = var.tags

  ingress {
    protocol        = "tcp"
    from_port       = var.app_port
    to_port         = var.app_port
    cidr_blocks     = ["0.0.0.0/0"]
    security_groups = [aws_security_group.lb.id]
  }

  egress {
    protocol    = "-1"
    from_port   = 0
    to_port     = 0
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_ecs_task_definition" "service" {
  family             = "liveask"
  network_mode       = "awsvpc"
  task_role_arn      = aws_iam_role.ecs_task_role.arn
  execution_role_arn = aws_iam_role.ecs_task_execution_role.arn

  cpu    = var.fargate_cpu
  memory = var.fargate_memory

  requires_compatibilities = ["FARGATE"]
  container_definitions = templatefile("task.json", {
    port           = var.app_port
    region         = var.region
    tinyurl_token  = var.tinyurl_token
    mj_template_id = var.mj_template_id
    mj_key         = var.mj_key
    mj_secret      = var.mj_secret
    loggroup       = aws_cloudwatch_log_group.api.name
    api-image      = var.docker-image
    redis          = "redis://${aws_elasticache_replication_group.redis.primary_endpoint_address}:${var.redis_port}"
  })

  tags = var.tags
}

resource "aws_ecs_cluster" "api" {
  name = "ecs-cluster"
  tags = var.tags
}

resource "aws_ecs_service" "api" {
  name                 = "liveask-service"
  tags                 = var.tags
  cluster              = aws_ecs_cluster.api.id
  task_definition      = aws_ecs_task_definition.service.arn
  launch_type          = "FARGATE"
  force_new_deployment = true

  desired_count                      = var.app_count
  deployment_minimum_healthy_percent = var.ecs_min_percent
  deployment_maximum_percent         = var.ecs_max_percent

  network_configuration {
    security_groups  = [aws_security_group.ecs_tasks.id]
    subnets          = aws_subnet.public.*.id
    assign_public_ip = true
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.alb.arn
    container_name   = "liveask-server"
    container_port   = var.app_port
  }

  depends_on = [
    aws_iam_role_policy_attachment.ecs_task_execution_role,
    aws_elasticache_replication_group.redis
  ]
}
