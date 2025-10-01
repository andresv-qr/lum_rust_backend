#!/bin/bash

# Test script for User Products API (/api/v4/invoices/products)
# This script tests all functionality of the products endpoint with JWT authentication

BASE_URL="http://localhost:8000"
JWT_TOKEN=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to make API request and show results
test_endpoint() {
    local test_name="$1"
    local url="$2"
    local expected_status="$3"
    
    print_status "Testing: $test_name"
    echo "URL: $url"
    
    if [ -n "$JWT_TOKEN" ]; then
        response=$(curl -s -w "\n%{http_code}" -H "Authorization: Bearer $JWT_TOKEN" "$url")
    else
        response=$(curl -s -w "\n%{http_code}" "$url")
    fi
    
    # Split response and status code
    status_code=$(echo "$response" | tail -n1)
    json_response=$(echo "$response" | head -n -1)
    
    echo "Status Code: $status_code"
    echo "Response: $json_response" | jq . 2>/dev/null || echo "$json_response"
    
    if [ "$status_code" = "$expected_status" ]; then
        print_success "Test passed (expected $expected_status, got $status_code)"
    else
        print_error "Test failed (expected $expected_status, got $status_code)"
    fi
    
    echo "----------------------------------------"
}

# Check if server is running
print_status "Checking if server is running at $BASE_URL"
if ! curl -s "$BASE_URL/health" > /dev/null; then
    print_error "Server is not running at $BASE_URL"
    print_warning "Please start the server with: cargo run"
    exit 1
fi
print_success "Server is running"

# Get JWT token for testing
print_status "Getting JWT token for testing"
echo "Please provide a valid JWT token for testing:"
read -r JWT_TOKEN

if [ -z "$JWT_TOKEN" ]; then
    print_warning "No JWT token provided. Testing unauthorized access only."
fi

echo "========================================"
echo "Testing User Products API"
echo "Endpoint: GET /api/v4/invoices/products"
echo "========================================"

# Test 1: Unauthorized access (without JWT)
if [ -n "$JWT_TOKEN" ]; then
    print_status "Test 1: Unauthorized access (no JWT token)"
    JWT_TOKEN_BACKUP="$JWT_TOKEN"
    JWT_TOKEN=""
    test_endpoint "Unauthorized access" "$BASE_URL/api/v4/invoices/products" "401"
    JWT_TOKEN="$JWT_TOKEN_BACKUP"
else
    test_endpoint "Unauthorized access" "$BASE_URL/api/v4/invoices/products" "401"
fi

# Skip authorized tests if no JWT token
if [ -z "$JWT_TOKEN" ]; then
    print_warning "Skipping authorized tests - no JWT token provided"
    exit 0
fi

# Test 2: Get all products (basic test)
test_endpoint "Get all user products" "$BASE_URL/api/v4/invoices/products" "200"

# Test 3: Test with update_date filter (recent date)
test_endpoint "Get products with recent update_date" "$BASE_URL/api/v4/invoices/products?update_date=2024-01-01" "200"

# Test 4: Test with update_date filter (old date)
test_endpoint "Get products with old update_date" "$BASE_URL/api/v4/invoices/products?update_date=2020-01-01" "200"

# Test 5: Test with ISO 8601 full datetime
test_endpoint "Get products with ISO datetime" "$BASE_URL/api/v4/invoices/products?update_date=2024-01-01T00:00:00Z" "200"

# Test 6: Test with invalid date format
test_endpoint "Invalid date format" "$BASE_URL/api/v4/invoices/products?update_date=invalid-date" "400"

# Test 7: Test with empty date parameter
test_endpoint "Empty date parameter" "$BASE_URL/api/v4/invoices/products?update_date=" "200"

# Test 8: Test with multiple parameters (update_date only for products)
test_endpoint "Multiple valid parameters" "$BASE_URL/api/v4/invoices/products?update_date=2024-06-01T00:00:00Z" "200"

# Test 9: Test with future date (should return empty or limited results)
test_endpoint "Future date filter" "$BASE_URL/api/v4/invoices/products?update_date=2030-01-01" "200"

# Test 10: Test endpoint health
print_status "Testing products endpoint response structure"
if [ -n "$JWT_TOKEN" ]; then
    response=$(curl -s -H "Authorization: Bearer $JWT_TOKEN" "$BASE_URL/api/v4/invoices/products")
    
    # Check if response has required fields
    if echo "$response" | jq -e '.success' > /dev/null 2>&1; then
        print_success "Response has 'success' field"
    else
        print_error "Response missing 'success' field"
    fi
    
    if echo "$response" | jq -e '.message' > /dev/null 2>&1; then
        print_success "Response has 'message' field"
    else
        print_error "Response missing 'message' field"
    fi
    
    if echo "$response" | jq -e '.data' > /dev/null 2>&1; then
        print_success "Response has 'data' field"
    else
        print_error "Response missing 'data' field"
    fi
    
    # Check if data is array
    if echo "$response" | jq -e '.data | type' | grep -q "array"; then
        print_success "Data field is an array"
        
        # Check array elements structure if not empty
        if echo "$response" | jq -e '.data | length > 0' > /dev/null 2>&1; then
            if echo "$response" | jq -e '.data[0].code' > /dev/null 2>&1; then
                print_success "Product items have 'code' field"
            else
                print_error "Product items missing 'code' field"
            fi
            
            if echo "$response" | jq -e '.data[0].issuer_ruc' > /dev/null 2>&1; then
                print_success "Product items have 'issuer_ruc' field"
            else
                print_warning "Product items missing 'issuer_ruc' field (may be null)"
            fi
            
            if echo "$response" | jq -e '.data[0].issuer_name' > /dev/null 2>&1; then
                print_success "Product items have 'issuer_name' field"
            else
                print_error "Product items missing 'issuer_name' field"
            fi
            
            if echo "$response" | jq -e '.data[0].description' > /dev/null 2>&1; then
                print_success "Product items have 'description' field"
            else
                print_error "Product items missing 'description' field"
            fi
            
            if echo "$response" | jq -e '.data[0].l1' > /dev/null 2>&1; then
                print_success "Product items have 'l1' classification field"
            else
                print_warning "Product items missing 'l1' field (may be null)"
            fi
        else
            print_warning "Data array is empty (no products found for user)"
        fi
    else
        print_error "Data field is not an array"
    fi
fi

echo "========================================"
print_status "User Products API testing completed"
echo "========================================"

# Performance test
print_status "Running performance test (10 consecutive requests)"
start_time=$(date +%s%N)
for i in {1..10}; do
    curl -s -H "Authorization: Bearer $JWT_TOKEN" "$BASE_URL/api/v4/invoices/products" > /dev/null
done
end_time=$(date +%s%N)
duration=$(((end_time - start_time) / 1000000))
avg_duration=$((duration / 10))
print_success "Average response time: ${avg_duration}ms"

echo ""
print_status "Suggested next steps:"
echo "1. Verify that the products returned belong to the authenticated user"
echo "2. Test with different JWT tokens to ensure user isolation"
echo "3. Verify update_date filtering works correctly with your data"
echo "4. Check database performance with larger datasets"
echo "5. Test rate limiting and security measures"
