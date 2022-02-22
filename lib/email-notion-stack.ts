import "dotenv/config";
import { Stack, StackProps } from "aws-cdk-lib";
import * as cdk from "aws-cdk-lib";
import {
  aws_lambda as lambda,
  aws_s3 as s3,
  aws_ses as ses,
  aws_ses_actions as actions,
} from "aws-cdk-lib";
import { Construct } from "constructs";

export class EmailNotionStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    // The code that defines your stack goes here
    const keyPrefix = `${process.env.ASSIGN_FOLDER}/`;
    const bucket = new s3.Bucket(this, "email", {
      versioned: true,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });
    const assign = new lambda.Function(this, "AssignHandler", {
      code: lambda.Code.fromAsset("lambda/zips/assign-task"),
      environment: {
        S3BUCKET: bucket.bucketName,
        KEY_PREFIX: keyPrefix,
      },
      handler: "assign-task",
      runtime: lambda.Runtime.PROVIDED_AL2,
    });
    // Only one RuleSet can be active at a time. If you create one from scratch,
    // you have to activate it in the AWS console each time. It makes more
    // sense to get the active RuleSet and change its rules.
    const activeRuleSet = ses.ReceiptRuleSet.fromReceiptRuleSetName(
      this,
      "sesRuleSet",
      "default"
    );
    activeRuleSet.addRule("assignRule", {
      receiptRuleName: "AssignRule",
      recipients: process.env.ASSIGN_EMAILS?.split(","),
      actions: [
        new actions.S3({
          bucket,
          objectKeyPrefix: keyPrefix,
        }),
        new actions.Lambda({
          function: assign,
        }),
      ],
    });
  }
}
