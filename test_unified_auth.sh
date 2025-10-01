#!/bin/bash

# ============================================================================
# UNIFIED AUTH ENDPOINT TEST
# ============================================================================
# Date: September 19, 2025
# Purpose: Test unified authentication endpoint
# ============================================================================

echo "🚀 Testing Unified Authentication Endpoint"
echo "=========================================="

# Test 1: Email Registration
echo ""
echo "Test 1: Email Registration"
echo "--------------------------"
curl -X POST http://localhost:3000/api/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider_data": {
      "provider": "email",
      "email": "test@example.com",
      "password": "password123",
      "name": "Test User"
    },
    "create_if_not_exists": true
  }' | jq '.'

echo ""
echo ""

# Test 2: Email Login
echo "Test 2: Email Login"
echo "-------------------"
curl -X POST http://localhost:3000/api/auth/unified \
  -H "Content-Type: application/json" \
  -d '{
    "provider_data": {
      "provider": "email",
      "email": "test@example.com",
      "password": "password123"
    },
    "create_if_not_exists": false
  }' | jq '.'

echo ""
echo ""

# Test 3: Health Check
echo "Test 3: Health Check"
echo "--------------------"
curl -X GET http://localhost:3000/api/auth/health | jq '.'

echo ""
echo ""

# Test 4: Config Check
echo "Test 4: Config Check"
echo "--------------------"
curl -X GET http://localhost:3000/api/auth/config | jq '.'

echo ""
echo "✅ All tests completed!"