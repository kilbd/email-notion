# Email a Task to Notion

Many of my work requests come through email, but our project management tracking happens in Notion. I needed a way to easily create a task in our Notion database with the context provided in the email. My solution was to use AWS Simple Email Service (SES) to receive emails and forward them to a Lambda function, which would add the task via the Notion API.

There are two email "endpoints":

- **Assign to me** – emails to this address are created and assigned to the sender, or ignored if the sender isn't a member of the Notion workspace.
- **Triage** - for emails sent to my group's mailing list. These are created and given the "Triage" status so they can be easily found and assigned to the most appropriate team member. _**(NOTE: This endpoint is not yet implemented)**_

## Usage

### Requirements

This project uses Amazon's CDK for configuring SES, S3, and the bundled Lambda functions written in Rust. You will need:

- Node and npm for running the AWS CDK
- Rust and Cargo for building and developing the Lambda function
- Zig to help build an AWS-compatible binary
- AWS CLI for deployments from local machine

### Deployments

To deploy this stack, follow these steps:

1. Ensure you have the AWS CLI configured for your account. See the [Getting Started guide](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-getting-started.html). For macOS and Homebrew, this may look like:

```shell
$ brew install awscli
$ aws configure
```

2. Set up bootstrapping for the project (needed for uploading Rust binaries). Get your account ID from the first command below, which you will need to provide along with the region for the bootstrap command:

```shell
$ aws sts get-caller-identity
$ npx aws-cdk bootstrap aws://{AWS_ACCOUNT_ID}/us-east-1
```

2. Install `cargo-zigbuild` per the [project instructions](https://github.com/messense/cargo-zigbuild). Note you will need to add an appropriate Rust target via `rustup target add`.
3. Build the Rust binaries:

```shell
$ cd lambda
$ make
$ cd ..
```

4. Add your secrets to a `.env` file:

```shell
$ cp sample.env .env
$ vi .env
```

5. Add `node_modules` and synthesize the CloudFormation templates to test

```shell
$ npm install
$ npx aws-cdk synth
```

6. If all went well, deploy to your AWS account. Reusing this command will replace your current deployment.

```shell
$ npx aws-cdk deploy
```

7. To tear down, use `npx aws-cdk destroy`.
