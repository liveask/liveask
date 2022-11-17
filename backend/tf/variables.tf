variable "region" {
  default = "eu-west-1"
}

variable "tags" {
  default = {
    Environment = "prod"
    Application = "liveask"
  }
}

variable "cidr" {
  description = "The CIDR block for the VPC."
  default     = "10.0.0.0/16"
}

variable "private_subnets" {
  description = "a list of CIDRs for private subnets in your VPC, must be set if the cidr variable is defined, needs to have as many elements as there are availability zones"
  default     = ["10.0.0.0/20", "10.0.32.0/20", "10.0.64.0/20"]
}

variable "public_subnets" {
  description = "a list of CIDRs for public subnets in your VPC, must be set if the cidr variable is defined, needs to have as many elements as there are availability zones"
  default     = ["10.0.16.0/20", "10.0.48.0/20", "10.0.80.0/20"]
}

variable "availability_zones" {
  description = "a comma-separated list of availability zones, defaults to all AZ of the region, if set to something other than the defaults, both private_subnets and public_subnets have to be defined as well"
  default     = ["eu-west-1a", "eu-west-1b", "eu-west-1c"]
}

variable "redis_port" {
  description = "what port redis is supposed to use"
  default     = 6379
}

variable "redis_redundancy" {
  description = "do we want redis to be redundant (multi az and auto failover)"
  default     = true
}

variable "redis_replica_nodes" {
  description = "replica nodes"
  default     = 2
}

variable "docker-image" {
  default = "liveask/server"
}

variable "app_port" {
  description = "Port exposed by the docker image to redirect traffic to"
  default     = 8090
}

variable "ecs_min_percent" {
  description = "ecs minimum health percentage"
  default     = 50
}

variable "ecs_max_percent" {
  description = "ecs max percentage"
  default     = 200
}

variable "fargate_cpu" {
  description = "Fargate instance CPU units to provision (1 vCPU = 1024 CPU units)"
  default     = "256"
}

variable "fargate_memory" {
  description = "Fargate instance memory to provision (in MiB)"
  default     = "512"
}

variable "app_count" {
  description = "Number of docker containers to run on ecs"
  default     = 2
}
