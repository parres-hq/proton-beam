#!/bin/bash
set -euo pipefail

# Proton Beam CloudFormation Deployment Helper
# This script makes it easy to deploy Proton Beam using CloudFormation

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Proton Beam - CloudFormation Deployment Helper      ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if AWS CLI is installed
if ! command -v aws &> /dev/null; then
    echo -e "${YELLOW}Error: AWS CLI not found. Please install it first.${NC}"
    echo "https://aws.amazon.com/cli/"
    exit 1
fi

# Check if jq is installed (optional but helpful)
if ! command -v jq &> /dev/null; then
    echo -e "${YELLOW}Warning: jq not found. Output will be less formatted.${NC}"
fi

# Default values
STACK_NAME="${STACK_NAME:-proton-beam-${USER}-$(date +%s)}"
REGION="${AWS_REGION:-us-east-1}"

echo "Configuration:"
echo "  Stack Name: $STACK_NAME"
echo "  Region: $REGION"
echo ""

# Function to get parameter
get_param() {
    local param_name=$1
    local prompt=$2
    local default=${3:-}

    if [[ -n "${!param_name:-}" ]]; then
        echo "${!param_name}"
        return
    fi

    if [[ -n "$default" ]]; then
        read -p "$prompt [$default]: " value
        echo "${value:-$default}"
    else
        read -p "$prompt: " value
        echo "$value"
    fi
}

# Collect parameters
echo "Please provide the following information:"
echo ""

INPUT_URL=$(get_param INPUT_URL "Input URL (HTTPS or S3)")
S3_OUTPUT_BUCKET=$(get_param S3_OUTPUT_BUCKET "S3 output bucket name")
S3_OUTPUT_PREFIX=$(get_param S3_OUTPUT_PREFIX "S3 output prefix" "proton-beam-output/")
INSTANCE_TYPE=$(get_param INSTANCE_TYPE "Instance type" "c6i.32xlarge")
VOLUME_SIZE=$(get_param VOLUME_SIZE "Volume size (GB)" "5000")
KEY_NAME=$(get_param KEY_NAME "EC2 Key Pair name")

# Get VPC and Subnet
echo ""
echo "Fetching default VPC and subnet..."
DEFAULT_VPC=$(aws ec2 describe-vpcs --region "$REGION" --filters "Name=is-default,Values=true" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")
if [[ -n "$DEFAULT_VPC" && "$DEFAULT_VPC" != "None" ]]; then
    VPC_ID=$(get_param VPC_ID "VPC ID" "$DEFAULT_VPC")
else
    VPC_ID=$(get_param VPC_ID "VPC ID")
fi

DEFAULT_SUBNET=$(aws ec2 describe-subnets --region "$REGION" --filters "Name=vpc-id,Values=$VPC_ID" --query 'Subnets[0].SubnetId' --output text 2>/dev/null || echo "")
if [[ -n "$DEFAULT_SUBNET" && "$DEFAULT_SUBNET" != "None" ]]; then
    SUBNET_ID=$(get_param SUBNET_ID "Subnet ID" "$DEFAULT_SUBNET")
else
    SUBNET_ID=$(get_param SUBNET_ID "Subnet ID")
fi

# Optional parameters
VALIDATE_SIGNATURES=$(get_param VALIDATE_SIGNATURES "Validate signatures (true/false)" "true")
VALIDATE_EVENT_IDS=$(get_param VALIDATE_EVENT_IDS "Validate event IDs (true/false)" "true")
COMPRESSION_LEVEL=$(get_param COMPRESSION_LEVEL "Compression level (0-9)" "6")
SHUTDOWN_WHEN_DONE=$(get_param SHUTDOWN_WHEN_DONE "Auto-shutdown when done (true/false)" "true")
USE_SPOT_INSTANCE=$(get_param USE_SPOT_INSTANCE "Use spot instance (true/false)" "false")

echo ""
echo -e "${GREEN}Creating CloudFormation stack...${NC}"
echo ""

# Create the stack
aws cloudformation create-stack \
    --region "$REGION" \
    --stack-name "$STACK_NAME" \
    --template-body file://$(dirname "$0")/aws-cloudformation.yaml \
    --capabilities CAPABILITY_IAM \
    --parameters \
        ParameterKey=InputURL,ParameterValue="$INPUT_URL" \
        ParameterKey=S3OutputBucket,ParameterValue="$S3_OUTPUT_BUCKET" \
        ParameterKey=S3OutputPrefix,ParameterValue="$S3_OUTPUT_PREFIX" \
        ParameterKey=InstanceType,ParameterValue="$INSTANCE_TYPE" \
        ParameterKey=VolumeSize,ParameterValue="$VOLUME_SIZE" \
        ParameterKey=KeyName,ParameterValue="$KEY_NAME" \
        ParameterKey=VpcId,ParameterValue="$VPC_ID" \
        ParameterKey=SubnetId,ParameterValue="$SUBNET_ID" \
        ParameterKey=ValidateSignatures,ParameterValue="$VALIDATE_SIGNATURES" \
        ParameterKey=ValidateEventIds,ParameterValue="$VALIDATE_EVENT_IDS" \
        ParameterKey=CompressionLevel,ParameterValue="$COMPRESSION_LEVEL" \
        ParameterKey=ShutdownWhenDone,ParameterValue="$SHUTDOWN_WHEN_DONE" \
        ParameterKey=UseSpotInstance,ParameterValue="$USE_SPOT_INSTANCE" \
    --tags \
        Key=Project,Value=ProtonBeam \
        Key=ManagedBy,Value=CloudFormation

echo -e "${GREEN}✓ Stack creation initiated${NC}"
echo ""
echo "Waiting for stack creation to complete..."
echo "(This may take 5-10 minutes)"
echo ""

# Wait for stack creation
aws cloudformation wait stack-create-complete \
    --region "$REGION" \
    --stack-name "$STACK_NAME"

echo -e "${GREEN}✓ Stack created successfully${NC}"
echo ""

# Get outputs
echo "Stack outputs:"
aws cloudformation describe-stacks \
    --region "$REGION" \
    --stack-name "$STACK_NAME" \
    --query 'Stacks[0].Outputs' \
    --output table

echo ""
echo -e "${GREEN}Deployment complete!${NC}"
echo ""
echo "To monitor progress:"
echo "  1. SSH into instance (see SSH command above)"
echo "  2. Tail the log: tail -f /var/log/proton-beam-deployment.log"
echo ""
echo "To check S3 output:"
echo "  aws s3 ls s3://$S3_OUTPUT_BUCKET/$S3_OUTPUT_PREFIX"
echo ""
echo "To delete the stack when done:"
echo "  aws cloudformation delete-stack --region $REGION --stack-name $STACK_NAME"
echo ""

