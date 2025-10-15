# CloudFormation Deployment Guide

Complete step-by-step guide for deploying Proton Beam using AWS CloudFormation.

## Prerequisites

### 1. AWS CLI Setup

```bash
# Install AWS CLI (if not already installed)
# macOS
brew install awscli

# Linux
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# Verify installation
aws --version
```

### 2. Configure AWS Credentials

```bash
aws configure

# You'll be prompted for:
# AWS Access Key ID: YOUR_ACCESS_KEY
# AWS Secret Access Key: YOUR_SECRET_KEY
# Default region name: us-east-1
# Default output format: json
```

### 3. Create EC2 Key Pair (if you don't have one)

```bash
# Create a new key pair
aws ec2 create-key-pair \
  --key-name proton-beam-key \
  --query 'KeyMaterial' \
  --output text > ~/.ssh/proton-beam-key.pem

# Set proper permissions
chmod 400 ~/.ssh/proton-beam-key.pem

# Verify it exists
aws ec2 describe-key-pairs --key-names proton-beam-key
```

### 4. Create S3 Bucket (if you don't have one)

```bash
# Create bucket (must be globally unique name)
aws s3 mb s3://your-proton-beam-output

# Verify it was created
aws s3 ls | grep proton-beam
```

## Deployment Methods

### Method 1: Interactive Helper Script (Recommended)

This is the easiest way - the script asks you questions and handles everything.

```bash
cd /Users/jeff/Code/proton-beam

# Run the helper script
./scripts/deploy-cloudformation.sh
```

You'll be prompted for:

```
Configuration:
  Stack Name: proton-beam-jeff-1234567890
  Region: us-east-1

Please provide the following information:

Input URL (HTTPS or S3): https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst
S3 output bucket name: your-output-bucket
S3 output prefix [proton-beam-output/]: processed/
Instance type [c6i.32xlarge]: ← Press Enter for default
Volume size (GB) [5000]: ← Press Enter for default
EC2 Key Pair name: proton-beam-key

Fetching default VPC and subnet...
VPC ID [vpc-xxxxx]: ← Usually press Enter for default
Subnet ID [subnet-xxxxx]: ← Usually press Enter for default

Validate signatures (true/false) [true]: ← true for quality, false for speed
Validate event IDs (true/false) [true]: ← true for quality, false for speed
Compression level (0-9) [6]: ← 6 is balanced
Auto-shutdown when done (true/false) [true]: ← Saves money!
Use spot instance (true/false) [false]: true ← Save 70%!
```

Then it will:
1. Create the CloudFormation stack
2. Wait for completion (~5 minutes for stack creation)
3. Display the outputs (IP address, SSH command, etc.)

### Method 2: Environment Variables (Scripted)

Set environment variables and run non-interactively:

```bash
cd /Users/jeff/Code/proton-beam

# Set all parameters
export INPUT_URL="https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst"
export S3_OUTPUT_BUCKET="your-output-bucket"
export S3_OUTPUT_PREFIX="processed/"
export INSTANCE_TYPE="c6i.32xlarge"
export VOLUME_SIZE="5000"
export KEY_NAME="proton-beam-key"
export VALIDATE_SIGNATURES="true"
export VALIDATE_EVENT_IDS="true"
export COMPRESSION_LEVEL="6"
export SHUTDOWN_WHEN_DONE="true"
export USE_SPOT_INSTANCE="true"

# Get default VPC and subnet
export VPC_ID=$(aws ec2 describe-vpcs \
  --filters "Name=is-default,Values=true" \
  --query 'Vpcs[0].VpcId' \
  --output text)

export SUBNET_ID=$(aws ec2 describe-subnets \
  --filters "Name=vpc-id,Values=$VPC_ID" \
  --query 'Subnets[0].SubnetId' \
  --output text)

# Run the script
./scripts/deploy-cloudformation.sh
```

### Method 3: Direct AWS CLI (Manual Control)

Use AWS CLI directly for complete control:

```bash
cd /Users/jeff/Code/proton-beam

# Create the stack
aws cloudformation create-stack \
  --stack-name proton-beam-processing \
  --template-body file://scripts/aws-cloudformation.yaml \
  --capabilities CAPABILITY_IAM \
  --parameters \
    ParameterKey=InputURL,ParameterValue="https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst" \
    ParameterKey=S3OutputBucket,ParameterValue="your-output-bucket" \
    ParameterKey=S3OutputPrefix,ParameterValue="processed/" \
    ParameterKey=InstanceType,ParameterValue="c6i.32xlarge" \
    ParameterKey=VolumeSize,ParameterValue="5000" \
    ParameterKey=KeyName,ParameterValue="proton-beam-key" \
    ParameterKey=VpcId,ParameterValue="vpc-xxxxx" \
    ParameterKey=SubnetId,ParameterValue="subnet-xxxxx" \
    ParameterKey=ValidateSignatures,ParameterValue="true" \
    ParameterKey=ValidateEventIds,ParameterValue="true" \
    ParameterKey=CompressionLevel,ParameterValue="6" \
    ParameterKey=ShutdownWhenDone,ParameterValue="true" \
    ParameterKey=UseSpotInstance,ParameterValue="true"

# Output will show:
{
    "StackId": "arn:aws:cloudformation:us-east-1:123456789:stack/proton-beam-processing/..."
}
```

### Method 4: AWS Console (Visual)

1. **Open AWS CloudFormation Console**
   - Go to: https://console.aws.amazon.com/cloudformation
   - Select your region (top-right)

2. **Create Stack**
   - Click "Create stack" → "With new resources"
   - Select "Upload a template file"
   - Click "Choose file" and select `scripts/aws-cloudformation.yaml`
   - Click "Next"

3. **Specify Stack Details**
   - Stack name: `proton-beam-processing`
   - Fill in parameters:
     ```
     InputURL: https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst
     S3OutputBucket: your-output-bucket
     S3OutputPrefix: processed/
     InstanceType: c6i.32xlarge
     VolumeSize: 5000
     KeyName: proton-beam-key
     VpcId: (select from dropdown)
     SubnetId: (select from dropdown)
     ValidateSignatures: true
     ValidateEventIds: true
     CompressionLevel: 6
     ShutdownWhenDone: true
     UseSpotInstance: true
     ```
   - Click "Next"

4. **Configure Stack Options** (optional)
   - Add tags if desired
   - Click "Next"

5. **Review**
   - Check "I acknowledge that AWS CloudFormation might create IAM resources"
   - Click "Submit"

6. **Monitor Progress**
   - Watch the "Events" tab
   - Wait for status: `CREATE_COMPLETE` (~5 minutes)

7. **View Outputs**
   - Click "Outputs" tab
   - Note the instance IP and SSH command

## Monitoring the Stack

### Check Stack Status

```bash
# Get stack status
aws cloudformation describe-stacks \
  --stack-name proton-beam-processing \
  --query 'Stacks[0].StackStatus' \
  --output text

# Watch events in real-time
watch -n 5 'aws cloudformation describe-stack-events \
  --stack-name proton-beam-processing \
  --max-items 5 \
  --query "StackEvents[*].[Timestamp,ResourceStatus,ResourceType,ResourceStatusReason]" \
  --output table'
```

### Get Stack Outputs

```bash
# Get all outputs
aws cloudformation describe-stacks \
  --stack-name proton-beam-processing \
  --query 'Stacks[0].Outputs' \
  --output table

# Get specific output (Instance IP)
aws cloudformation describe-stacks \
  --stack-name proton-beam-processing \
  --query 'Stacks[0].Outputs[?OutputKey==`PublicIp`].OutputValue' \
  --output text
```

Example outputs:
```
┌──────────────────────┬────────────────────────────────────────────────┐
│     OutputKey        │                 OutputValue                    │
├──────────────────────┼────────────────────────────────────────────────┤
│  InstanceId          │  i-0123456789abcdef0                           │
│  PublicIp            │  54.123.45.67                                  │
│  PublicDnsName       │  ec2-54-123-45-67.compute-1.amazonaws.com      │
│  SSHCommand          │  ssh -i proton-beam-key.pem ubuntu@54.123.45.67│
│  S3OutputLocation    │  s3://your-output-bucket/processed/            │
│  S3ListCommand       │  aws s3 ls s3://your-output-bucket/processed/  │
└──────────────────────┴────────────────────────────────────────────────┘
```

## SSH Into Instance

Once the stack is created:

```bash
# Get IP address
INSTANCE_IP=$(aws cloudformation describe-stacks \
  --stack-name proton-beam-processing \
  --query 'Stacks[0].Outputs[?OutputKey==`PublicIp`].OutputValue' \
  --output text)

# SSH in
ssh -i ~/.ssh/proton-beam-key.pem ubuntu@$INSTANCE_IP
```

### Watch the Processing

Once connected via SSH:

```bash
# Watch deployment log
tail -f /var/log/proton-beam-deployment.log

# Watch conversion log
tail -f /data/pb_data/proton-beam.log

# Check progress
watch -n 5 'tail -20 /var/log/proton-beam-deployment.log'

# Monitor system resources
htop

# Check disk usage
df -h

# Watch S3 uploads (when they start)
watch -n 10 'aws s3 ls s3://your-output-bucket/processed/ --human-readable'
```

## Timeline

Here's what happens after you create the stack:

```
Time    Event
─────────────────────────────────────────────────────────────
0:00    CloudFormation stack creation begins
0:01    ├─ IAM Role created
0:01    ├─ Security Group created
0:02    ├─ Instance Profile created
0:02    └─ EC2 Instance launching...
0:05    Stack creation complete: CREATE_COMPLETE

        [Instance now running, User Data script executing...]

0:06    ├─ Installing dependencies (build tools, zstd, etc.)
0:08    ├─ Installing Rust
0:10    ├─ Building Proton Beam with S3 support
0:16    ├─ Downloading .zst file (400GB)
0:23    ├─ Decompressing to JSONL (1.2TB)
0:34    ├─ Converting to protobuf (128 cores)
0:44    ├─ Building index
0:46    ├─ Uploading to S3
0:51    └─ Cleanup and shutdown initiated

0:52    Instance shutting down (if SHUTDOWN_WHEN_DONE=true)
```

**Total time: ~50 minutes** (35 min processing + 15 min setup)

## Verify Completion

### Check S3 for Results

```bash
# List all files
aws s3 ls s3://your-output-bucket/processed/ --recursive --human-readable

# Expected output:
2025-09-27 15:23:45   45.2 GiB  2025_09_27.pb.gz
2025-09-27 15:24:12   48.3 GiB  2025_09_28.pb.gz
...
2025-09-27 15:25:01  125.5 MiB  index.db
2025-09-27 15:25:02    2.1 MiB  proton-beam.log

# Total size
aws s3 ls s3://your-output-bucket/processed/ --recursive --summarize

# Download log to check for errors
aws s3 cp s3://your-output-bucket/processed/proton-beam.log ./
grep ERROR proton-beam.log
```

### Check Instance Status

```bash
# Check if instance is still running
aws ec2 describe-instances \
  --filters "Name=tag:aws:cloudformation:stack-name,Values=proton-beam-processing" \
  --query 'Reservations[0].Instances[0].State.Name' \
  --output text

# Should show: terminated (if auto-shutdown enabled)
# Or: running (if still processing)
```

## Cleanup

### Delete Everything (When Done)

```bash
# Delete the CloudFormation stack (removes everything)
aws cloudformation delete-stack --stack-name proton-beam-processing

# Wait for deletion to complete
aws cloudformation wait stack-delete-complete --stack-name proton-beam-processing

# Verify deletion
aws cloudformation describe-stacks --stack-name proton-beam-processing
# Should show: "Stack with id ... does not exist"
```

This will delete:
- ✅ EC2 Instance
- ✅ EBS Volume
- ✅ Security Group
- ✅ IAM Role
- ✅ Instance Profile

**Note**: S3 files are NOT deleted (you keep your results!)

### Manual Cleanup (If Needed)

If auto-shutdown is disabled and you want to stop the instance:

```bash
# Get instance ID
INSTANCE_ID=$(aws cloudformation describe-stacks \
  --stack-name proton-beam-processing \
  --query 'Stacks[0].Outputs[?OutputKey==`InstanceId`].OutputValue' \
  --output text)

# Stop instance (keeps EBS, stops billing compute)
aws ec2 stop-instances --instance-ids $INSTANCE_ID

# Or terminate instance (deletes everything)
aws ec2 terminate-instances --instance-ids $INSTANCE_ID
```

## Troubleshooting

### Stack Creation Failed

```bash
# Check what failed
aws cloudformation describe-stack-events \
  --stack-name proton-beam-processing \
  --query "StackEvents[?ResourceStatus=='CREATE_FAILED']" \
  --output table

# Common issues:
# - No EC2 key pair: Create one first
# - Insufficient permissions: Check IAM user permissions
# - VPC/Subnet not found: Verify VPC ID and Subnet ID
# - Instance limit reached: Request limit increase
```

### Instance Not Processing

SSH in and check:

```bash
# Check if user data script ran
sudo cat /var/log/cloud-init-output.log

# Check deployment log
tail -100 /var/log/proton-beam-deployment.log

# Check for errors
grep -i error /var/log/proton-beam-deployment.log
```

### Out of Disk Space

```bash
# Check disk usage
df -h

# Should show 5TB available on /dev/nvme0n1p1
# If full, increase VolumeSize parameter and recreate stack
```

### Spot Instance Interrupted

If using Spot instances, they can be interrupted:

```bash
# Check interruption notice (2 minutes before termination)
curl http://169.254.169.254/latest/meta-data/spot/instance-action

# If interrupted, the work is lost
# Solution: Use on-demand or retry with Spot
```

## Cost Breakdown

For your 1.2TB dataset with CloudFormation:

```
Instance (c6i.32xlarge Spot):
  ~50 minutes × $1.92/hr = $1.60

EBS Volume (5TB gp3):
  50 minutes × 5TB × $0.08/GB/mo ÷ 730 hrs = $0.27

S3 Upload:
  Free (within region)

S3 Storage (ongoing):
  ~450GB × $0.023/GB/mo = $10.35/month

Data Transfer:
  Free (download from internet, upload to S3 same region)

Total One-Time: $1.87 (Spot) or $5.60 (On-Demand)
Ongoing Storage: $10.35/month
```

## Best Practices

1. **Always Use Spot for Processing**
   - 70% cheaper
   - For 50-minute job, interruption risk is low
   - If interrupted, just retry

2. **Enable Auto-Shutdown**
   - Saves money if you forget about it
   - Set `ShutdownWhenDone: true`

3. **Use Tags**
   - CloudFormation auto-tags all resources
   - Makes cost tracking easy

4. **Keep Logs in S3**
   - The script uploads logs automatically
   - Essential for debugging

5. **Delete Stack When Done**
   - Don't leave instance running
   - S3 data persists after stack deletion

6. **Test with Small Data First**
   - Use a subset to verify everything works
   - Then run full dataset

## Advanced Usage

### Update Stack Parameters

```bash
# Change parameters without recreating
aws cloudformation update-stack \
  --stack-name proton-beam-processing \
  --use-previous-template \
  --parameters \
    ParameterKey=InstanceType,ParameterValue="c6i.16xlarge" \
    ParameterKey=VolumeSize,UsePreviousValue=true \
    # ... other parameters
```

### Multiple Stacks in Parallel

Process multiple datasets simultaneously:

```bash
# Create multiple stacks
for i in {1..4}; do
  aws cloudformation create-stack \
    --stack-name proton-beam-chunk-$i \
    --template-body file://scripts/aws-cloudformation.yaml \
    --parameters ...
done
```

### Export Template

```bash
# Save template for version control
aws cloudformation get-template \
  --stack-name proton-beam-processing \
  --query 'TemplateBody' \
  > my-template.yaml
```

## Quick Reference

```bash
# Create stack
./scripts/deploy-cloudformation.sh

# Check status
aws cloudformation describe-stacks --stack-name proton-beam-processing

# Get IP
aws cloudformation describe-stacks --stack-name proton-beam-processing \
  --query 'Stacks[0].Outputs[?OutputKey==`PublicIp`].OutputValue' --output text

# SSH in
ssh -i ~/.ssh/proton-beam-key.pem ubuntu@<ip>

# Check results
aws s3 ls s3://your-output-bucket/processed/

# Delete stack
aws cloudformation delete-stack --stack-name proton-beam-processing
```

## Summary

CloudFormation deployment is:
- ✅ Fully automated
- ✅ Repeatable
- ✅ Version controlled
- ✅ Easy to clean up
- ✅ Best for production workflows

For your 1.2TB dataset, it's the recommended approach! 🚀

Total time: ~50 minutes
Total cost: ~$1.87 (Spot) or ~$5.60 (On-Demand)

