# CloudFormation Quick Start

**One-page guide to deploy Proton Beam on AWS in 5 minutes**

## Prerequisites (Do Once)

```bash
# 1. Create EC2 key pair
aws ec2 create-key-pair \
  --key-name proton-beam-key \
  --query 'KeyMaterial' \
  --output text > ~/.ssh/proton-beam-key.pem
chmod 400 ~/.ssh/proton-beam-key.pem

# 2. Create S3 bucket
aws s3 mb s3://your-proton-beam-output
```

## Deploy (3 Steps)

### Step 1: Set Configuration

```bash
cd /Users/jeff/Code/proton-beam

export INPUT_URL="https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst"
export S3_OUTPUT_BUCKET="your-proton-beam-output"
export KEY_NAME="proton-beam-key"
export USE_SPOT_INSTANCE="true"
```

### Step 2: Deploy

```bash
./scripts/deploy-cloudformation.sh
```

Press Enter to accept defaults for most questions.

### Step 3: Wait & Monitor

```bash
# Get instance IP (wait ~5 min for stack creation)
INSTANCE_IP=$(aws cloudformation describe-stacks \
  --stack-name proton-beam-* \
  --query 'Stacks[0].Outputs[?OutputKey==`PublicIp`].OutputValue' \
  --output text)

# SSH and watch
ssh -i ~/.ssh/proton-beam-key.pem ubuntu@$INSTANCE_IP
tail -f /var/log/proton-beam-deployment.log
```

## Timeline

```
0 min  → Create stack
5 min  → Stack complete, instance running
15 min → Dependencies installed, Rust built
22 min → File downloaded (400GB)
33 min → File decompressed (1.2TB)
43 min → Conversion complete
45 min → Index built
50 min → Upload to S3 complete
51 min → Instance shutdown ✅
```

## Check Results

```bash
# List files in S3
aws s3 ls s3://your-proton-beam-output/proton-beam-output/ --human-readable

# Download log
aws s3 cp s3://your-proton-beam-output/proton-beam-output/proton-beam.log ./
```

## Cleanup

```bash
# Delete everything (keeps S3 data)
aws cloudformation delete-stack --stack-name proton-beam-*
```

## Cost

- **Spot**: ~$1.87 for 50 minutes
- **On-Demand**: ~$5.60 for 50 minutes
- **Storage**: $10.35/month for 450GB in S3

## Troubleshooting

```bash
# Check stack status
aws cloudformation describe-stacks --stack-name proton-beam-*

# View events
aws cloudformation describe-stack-events --stack-name proton-beam-* --max-items 10

# Check instance
ssh -i ~/.ssh/proton-beam-key.pem ubuntu@<ip>
tail -100 /var/log/proton-beam-deployment.log
```

## Full Guide

See [CLOUDFORMATION_GUIDE.md](CLOUDFORMATION_GUIDE.md) for complete documentation.

---

**That's it!** Set 3 variables, run 1 command, wait 50 minutes, get your data in S3. 🚀

