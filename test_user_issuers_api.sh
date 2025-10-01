#!/bin/bash

# Test script for User Issuers API v4
# Requirement: JWT token in environment varia# Test 7: Second page
echo -e "${BLUE}📋 Test 7: Get user issuers (second page - limit=10, offset=10)${NC}"
curl -X GET "$BASE_URL$ENDPOINT?limit=10&offset=10" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-7" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 7 completed${NC}\n"

# Test 8: Without JWT (should fail with 401)
echo -e "${BLUE}📋 Test 8: Without JWT (should fail with 401)${NC}"e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 Testing User Issuers API v4${NC}"
echo "============================================"

# Check if JWT token is provided
if [ -z "$JWT_TOKEN" ]; then
    echo -e "${RED}❌ Error: JWT_TOKEN environment variable not set${NC}"
    echo "Usage: JWT_TOKEN='your_jwt_token' ./test_user_issuers_api.sh"
    exit 1
fi

BASE_URL="http://localhost:8000"
ENDPOINT="/api/v4/invoices/issuers"

echo -e "${YELLOW}📡 Base URL:${NC} $BASE_URL"
echo -e "${YELLOW}🎯 Endpoint:${NC} $ENDPOINT"
echo ""

# Test 1: Basic request - Get user issuers (default pagination)
echo -e "${BLUE}📋 Test 1: Get user issuers (default pagination)${NC}"
curl -X GET "$BASE_URL$ENDPOINT" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-1" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 1 completed${NC}\n"

# Test 2: With custom pagination
echo -e "${BLUE}📋 Test 2: Get user issuers (limit=5, offset=0)${NC}"
curl -X GET "$BASE_URL$ENDPOINT?limit=5&offset=0" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-2" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 2 completed${NC}\n"

# Test 3: Large limit (should be capped at 100)
echo -e "${BLUE}📋 Test 3: Get user issuers (limit=500 - should be capped at 100)${NC}"
curl -X GET "$BASE_URL$ENDPOINT?limit=500&offset=0" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-3" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 3 completed${NC}\n"

# Test 4: With date filter (last 30 days)
echo -e "${BLUE}📋 Test 4: Get user issuers (with date filter - last 30 days)${NC}"
THIRTY_DAYS_AGO=$(date -d '30 days ago' --iso-8601=seconds)
curl -X GET "$BASE_URL$ENDPOINT?limit=10&offset=0&update_date_from=$THIRTY_DAYS_AGO" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-4" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 4 completed${NC}\n"

# Test 5: With specific date filter
echo -e "${BLUE}📋 Test 5: Get user issuers (with specific date filter)${NC}"
curl -X GET "$BASE_URL$ENDPOINT?limit=5&offset=0&update_date_from=2024-01-01T00:00:00Z" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-5" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 5 completed${NC}\n"

# Test 6: Invalid date format (should fail with 400)
echo -e "${BLUE}📋 Test 6: Invalid date format (should fail with 400)${NC}"
curl -X GET "$BASE_URL$ENDPOINT?limit=5&offset=0&update_date_from=invalid-date" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-6" \
  -w "\n⏱️  Response time: %{time_total}s\n📊 HTTP Code: %{http_code}\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 6 completed${NC}\n"

# Test 7: Second page
echo -e "${BLUE}📋 Test 7: Get user issuers (second page - limit=10, offset=10)${NC}"
curl -X GET "$BASE_URL$ENDPOINT?limit=10&offset=10" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-4" \
  -w "\n⏱️  Response time: %{time_total}s\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 4 completed${NC}\n"

# Test 8: Without JWT (should fail with 401)
echo -e "${BLUE}📋 Test 8: Without JWT (should fail with 401)${NC}"
curl -X GET "$BASE_URL$ENDPOINT" \
  -H "Content-Type: application/json" \
  -H "x-request-id: test-user-issuers-8" \
  -w "\n⏱️  Response time: %{time_total}s\n📊 HTTP Code: %{http_code}\n" \
  -s | jq '.'

echo -e "\n${GREEN}✅ Test 8 completed${NC}\n"

echo -e "${GREEN}🎉 All tests completed!${NC}"
echo -e "${YELLOW}💡 Tip: Check the server logs for detailed request processing information${NC}"
