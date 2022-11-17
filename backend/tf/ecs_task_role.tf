
resource "aws_iam_role" "ecs_task_role" {
  name               = "la-ecs-task-role"
  tags               = var.tags
  description        = "role the task uses to access other AWS services"
  assume_role_policy = data.aws_iam_policy_document.policy_doc_assume_role.json
}

data "aws_iam_policy_document" "policy_doc_dynamo" {
  version = "2012-10-17"
  statement {
    sid    = ""
    effect = "Allow"
    actions = [
      "dynamodb:GetItem",
      "dynamodb:PutItem",
      "dynamodb:ListTables",
    ]
    resources = [var.ddb_table_arn]
  }
}

resource "aws_iam_policy" "dynamo_access" {
  name        = "ecs-access-dynamo"
  description = "policy allowing ecs to access dynamodb"
  tags        = var.tags

  policy = data.aws_iam_policy_document.policy_doc_dynamo.json
}

resource "aws_iam_role_policy_attachment" "ecs_task_role_dynamo_access" {
  role       = aws_iam_role.ecs_task_role.name
  policy_arn = aws_iam_policy.dynamo_access.arn
}
