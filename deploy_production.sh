#!/bin/bash
# ====================================================================
# LUM RUST WS - PRODUCTION DEPLOYMENT SCRIPT
# ====================================================================
# Este script automatiza el deployment del servidor a producción
# Versión: 1.0
# Fecha: Octubre 2025
# ====================================================================

set -e  # Exit on any error

# ====================================================================
# CONFIGURATION - EDITAR SEGÚN TU INFRAESTRUCTURA
# ====================================================================
PRODUCTION_SERVER="api.lumapp.org"
PRODUCTION_USER="root"  # o tu usuario con permisos sudo
PRODUCTION_PATH="/opt/lum_rust_ws"
SERVICE_NAME="lum_rust_ws"

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ====================================================================
# FUNCTIONS
# ====================================================================
print_step() {
    echo -e "${BLUE}==>${NC} ${GREEN}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# ====================================================================
# PRE-DEPLOYMENT CHECKS
# ====================================================================
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   LUM RUST WS - PRODUCTION DEPLOYMENT                    ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""

print_step "1/10 - Pre-deployment checks..."

# Check if binary exists
if [ ! -f "target/release/lum_rust_ws" ]; then
    print_error "Binary not found! Run 'cargo build --release' first"
    exit 1
fi
print_success "Binary found ($(ls -lh target/release/lum_rust_ws | awk '{print $5}'))"

# Check if .env.production exists
if [ ! -f ".env.production" ]; then
    print_error ".env.production not found!"
    exit 1
fi
print_success ".env.production found"

# Verify .env.production has been configured
if grep -q "REEMPLAZAR_CON" .env.production; then
    print_error ".env.production contains placeholder values!"
    print_warning "Please edit .env.production with real credentials"
    exit 1
fi
print_success ".env.production appears configured"

# ====================================================================
# BUILD FINAL BINARY (Optional - already built)
# ====================================================================
print_step "2/10 - Checking if rebuild is needed..."
read -p "Rebuild binary? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_step "Building release binary..."
    cargo build --release
    print_success "Binary rebuilt"
else
    print_success "Using existing binary"
fi

# ====================================================================
# CREATE DEPLOYMENT PACKAGE
# ====================================================================
print_step "3/10 - Creating deployment package..."

DEPLOY_DIR="deploy_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$DEPLOY_DIR"

# Copy binary
cp target/release/lum_rust_ws "$DEPLOY_DIR/"
print_success "Binary copied"

# Copy .env file as .env (rename from .env.production)
cp .env.production "$DEPLOY_DIR/.env"
print_success "Environment file copied"

# Copy systemd service file
if [ -f "lum_rust_ws.service" ]; then
    cp lum_rust_ws.service "$DEPLOY_DIR/"
    print_success "Systemd service file copied"
fi

# Create tar.gz
tar -czf "${DEPLOY_DIR}.tar.gz" -C "$DEPLOY_DIR" .
PACKAGE_SIZE=$(ls -lh "${DEPLOY_DIR}.tar.gz" | awk '{print $5}')
print_success "Package created: ${DEPLOY_DIR}.tar.gz (${PACKAGE_SIZE})"

# Cleanup temporary directory
rm -rf "$DEPLOY_DIR"

# ====================================================================
# UPLOAD TO PRODUCTION SERVER
# ====================================================================
print_step "4/10 - Uploading to production server..."

print_warning "Uploading to ${PRODUCTION_USER}@${PRODUCTION_SERVER}:${PRODUCTION_PATH}"
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_error "Deployment cancelled"
    exit 1
fi

# Create production directory if it doesn't exist
ssh "${PRODUCTION_USER}@${PRODUCTION_SERVER}" "mkdir -p ${PRODUCTION_PATH}/backups"
print_success "Production directory ready"

# Upload package
scp "${DEPLOY_DIR}.tar.gz" "${PRODUCTION_USER}@${PRODUCTION_SERVER}:${PRODUCTION_PATH}/"
print_success "Package uploaded"

# ====================================================================
# BACKUP EXISTING DEPLOYMENT
# ====================================================================
print_step "5/10 - Backing up existing deployment..."

ssh "${PRODUCTION_USER}@${PRODUCTION_SERVER}" << 'ENDSSH'
set -e
cd /opt/lum_rust_ws

if [ -f "lum_rust_ws" ]; then
    BACKUP_NAME="backup_$(date +%Y%m%d_%H%M%S).tar.gz"
    tar -czf "backups/${BACKUP_NAME}" lum_rust_ws .env 2>/dev/null || true
    echo "✅ Backup created: ${BACKUP_NAME}"
    
    # Keep only last 5 backups
    cd backups
    ls -t backup_*.tar.gz | tail -n +6 | xargs rm -f 2>/dev/null || true
    echo "✅ Old backups cleaned (keeping last 5)"
else
    echo "ℹ️  No existing deployment to backup"
fi
ENDSSH

print_success "Backup completed"

# ====================================================================
# STOP EXISTING SERVICE
# ====================================================================
print_step "6/10 - Stopping existing service..."

ssh "${PRODUCTION_USER}@${PRODUCTION_SERVER}" << ENDSSH
set -e

# Check if service exists
if systemctl list-units --full --all | grep -q "${SERVICE_NAME}.service"; then
    echo "Stopping ${SERVICE_NAME}..."
    sudo systemctl stop ${SERVICE_NAME} || true
    sleep 2
    echo "✅ Service stopped"
else
    echo "ℹ️  Service ${SERVICE_NAME} not found (first deployment?)"
fi

# Also check for old qreader_api service
if systemctl list-units --full --all | grep -q "qreader_api.service"; then
    echo "⚠️  Found old qreader_api service"
    read -p "Stop qreader_api? (y/N): " -n 1 -r
    echo
    if [[ \$REPLY =~ ^[Yy]$ ]]; then
        sudo systemctl stop qreader_api || true
        sudo systemctl disable qreader_api || true
        echo "✅ qreader_api stopped and disabled"
    fi
fi
ENDSSH

print_success "Service management completed"

# ====================================================================
# EXTRACT NEW DEPLOYMENT
# ====================================================================
print_step "7/10 - Extracting new deployment..."

ssh "${PRODUCTION_USER}@${PRODUCTION_SERVER}" << ENDSSH
set -e
cd ${PRODUCTION_PATH}

# Extract
tar -xzf ${DEPLOY_DIR}.tar.gz

# Set permissions
chmod +x lum_rust_ws
chown ${PRODUCTION_USER}:${PRODUCTION_USER} lum_rust_ws .env

echo "✅ Files extracted and permissions set"
ENDSSH

print_success "Deployment extracted"

# ====================================================================
# INSTALL SYSTEMD SERVICE
# ====================================================================
print_step "8/10 - Installing systemd service..."

ssh "${PRODUCTION_USER}@${PRODUCTION_SERVER}" << ENDSSH
set -e
cd ${PRODUCTION_PATH}

if [ -f "lum_rust_ws.service" ]; then
    sudo cp lum_rust_ws.service /etc/systemd/system/${SERVICE_NAME}.service
    sudo systemctl daemon-reload
    sudo systemctl enable ${SERVICE_NAME}
    echo "✅ Systemd service installed and enabled"
else
    echo "⚠️  Service file not found, skipping systemd setup"
fi
ENDSSH

print_success "Systemd service configured"

# ====================================================================
# START SERVICE
# ====================================================================
print_step "9/10 - Starting service..."

ssh "${PRODUCTION_USER}@${PRODUCTION_SERVER}" << ENDSSH
set -e

echo "Starting ${SERVICE_NAME}..."
sudo systemctl start ${SERVICE_NAME}

echo "Waiting 5 seconds for service to start..."
sleep 5

# Check status
if sudo systemctl is-active --quiet ${SERVICE_NAME}; then
    echo "✅ Service started successfully"
    sudo systemctl status ${SERVICE_NAME} --no-pager -l | head -20
else
    echo "❌ Service failed to start!"
    echo "Checking logs:"
    sudo journalctl -u ${SERVICE_NAME} -n 50 --no-pager
    exit 1
fi
ENDSSH

print_success "Service started"

# ====================================================================
# HEALTH CHECK
# ====================================================================
print_step "10/10 - Running health checks..."

echo "Waiting 3 seconds for server to be ready..."
sleep 3

# Check health endpoint
HEALTH_RESPONSE=$(curl -s https://${PRODUCTION_SERVER}/health || echo "FAILED")

if echo "$HEALTH_RESPONSE" | grep -q "lum_rust_ws"; then
    print_success "Health check passed!"
    echo "$HEALTH_RESPONSE" | jq . 2>/dev/null || echo "$HEALTH_RESPONSE"
else
    print_error "Health check failed!"
    echo "Response: $HEALTH_RESPONSE"
    print_warning "Check server logs for details"
fi

# Check rewards endpoint
echo ""
print_step "Testing /api/v1/rewards/offers endpoint..."
OFFERS_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" https://${PRODUCTION_SERVER}/api/v1/rewards/offers -H "Authorization: Bearer test" || echo "FAILED")

if [ "$OFFERS_RESPONSE" == "401" ]; then
    print_success "Rewards endpoint responding (401 = auth working correctly)"
elif [ "$OFFERS_RESPONSE" == "404" ]; then
    print_error "Rewards endpoint not found (404)"
else
    print_warning "Unexpected response: $OFFERS_RESPONSE"
fi

# ====================================================================
# CLEANUP
# ====================================================================
echo ""
print_step "Cleaning up local deployment package..."
rm -f "${DEPLOY_DIR}.tar.gz"
print_success "Cleanup completed"

# ====================================================================
# DEPLOYMENT SUMMARY
# ====================================================================
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   DEPLOYMENT COMPLETED                                    ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""
print_success "Server: https://${PRODUCTION_SERVER}"
print_success "Service: ${SERVICE_NAME}"
print_success "Path: ${PRODUCTION_PATH}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Test endpoints with real JWT token"
echo "2. Monitor logs: ssh ${PRODUCTION_USER}@${PRODUCTION_SERVER} 'sudo journalctl -u ${SERVICE_NAME} -f'"
echo "3. Check metrics: https://${PRODUCTION_SERVER}/monitoring/metrics"
echo "4. Test push notifications with real FCM_SERVER_KEY"
echo ""
echo -e "${YELLOW}Rollback if needed:${NC}"
echo "ssh ${PRODUCTION_USER}@${PRODUCTION_SERVER}"
echo "cd ${PRODUCTION_PATH}/backups"
echo "tar -xzf backup_YYYYMMDD_HHMMSS.tar.gz -C ${PRODUCTION_PATH}"
echo "sudo systemctl restart ${SERVICE_NAME}"
echo ""
