resource "aws_vpc" "main" {
  cidr_block = var.cidr
  tags       = var.tags
}

resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id
  tags   = var.tags
}

resource "aws_subnet" "public" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = element(var.public_subnets, count.index)
  availability_zone       = element(var.availability_zones, count.index)
  count                   = length(var.public_subnets)
  map_public_ip_on_launch = true
  tags                    = var.tags
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id
  tags   = var.tags
}

resource "aws_route" "public" {
  route_table_id         = aws_route_table.public.id
  destination_cidr_block = "0.0.0.0/0"
  gateway_id             = aws_internet_gateway.main.id
}

resource "aws_route_table_association" "public" {
  count          = length(var.public_subnets)
  subnet_id      = element(aws_subnet.public.*.id, count.index)
  route_table_id = aws_route_table.public.id
}
