import asyncio
import aiohttp
import time
import random
import statistics
import argparse
from collections import defaultdict
import json

# Configuration
BASE_URL = "http://localhost:8000"
USER_EMAIL = "andresfelipevalenciag@gmail.com"
USER_PASS = "Andres123+"

# Endpoints weights (Probability of action)
ACTIONS = [
    # (Name, Weight, Method, UrlPath, Data, IsJson)
    ("get_balance", 40, "GET", "/api/v4/rewards/balance", None, False),
    ("get_offers", 30, "GET", "/api/v4/ofertasws", None, False),
    ("get_headers", 15, "GET", "/api/v4/invoices/headers?limit=20", None, False),
    ("get_userdata", 10, "GET", "/api/v4/userdata", None, False),
    ("track_login", 5, "POST", "/api/v4/gamification/track", {
        "action": "daily_login", 
        "channel": "api",
        "metadata": {"source": "load_testing"}
    }, True)
]

# Global stats
stats = {
    "requests": 0,
    "errors": 0,
    "latencies": [],
    "codes": defaultdict(int),
    "endpoints": defaultdict(lambda: {"count": 0, "errors": 0, "latencies": []})
}

async def get_auth_token(session):
    """Authenticate and return JWT token."""
    url = f"{BASE_URL}/api/v4/auth/login"
    payload = {
        "email": USER_EMAIL,
        "password": USER_PASS,
        "provider": "email"
    }
    
    try:
        async with session.post(url, json=payload) as response:
            if response.status == 200:
                data = await response.json()
                # Handle different response structures based on analysis
                if "token" in data:
                    return data["token"]
                elif "access_token" in data:
                    return data["access_token"]
                elif "data" in data and "token" in data["data"]:
                    return data["data"]["token"]
                else:
                    print(f"Login failed: Token not found in response: {data}")
                    return None
            else:
                body = await response.text()
                print(f"Login failed: {response.status} - {body}")
                return None
    except Exception as e:
        print(f"Login exception: {str(e)}")
        return None

async def user_session(client_id, session, token, duration):
    """Simulates a single user's behavior over time."""
    start_time = time.time()
    
    # Spoof IP to bypass rate limit per user
    fake_ip = f"10.0.{random.randint(0,255)}.{random.randint(1,254)}"
    
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
        "X-Forwarded-For": fake_ip,
        "X-Real-IP": fake_ip
    }

    while time.time() - start_time < duration:
        # Think time (human behavior simulation)
        await asyncio.sleep(random.uniform(0.5, 2.0))
        
        # Pick an action
        action_name, _, method, path, data, is_json = random.choices(
            ACTIONS, 
            weights=[a[1] for a in ACTIONS], 
            k=1
        )[0]

        url = f"{BASE_URL}{path}"
        req_start = time.time()
        
        try:
            async with session.request(method, url, headers=headers, json=data if is_json else None) as response:
                latency = (time.time() - req_start) * 1000 # ms
                
                # Record stats
                stats["requests"] += 1
                stats["codes"][response.status] += 1
                stats["latencies"].append(latency)
                stats["endpoints"][action_name]["count"] += 1
                stats["endpoints"][action_name]["latencies"].append(latency)
                
                if response.status >= 400:
                    stats["errors"] += 1
                    stats["endpoints"][action_name]["errors"] += 1
                    # Optional: Print first few errors
                    # if stats["errors"] <= 5:
                    #     print(f"Error {response.status} on {action_name}: {await response.text()}")

        except Exception as e:
            stats["errors"] += 1
            # print(f"Exception on {action_name}: {e}")

async def run_stress_test(num_users, duration):
    print(f"Starting stress test with {num_users} users for {duration} seconds...")
    
    async with aiohttp.ClientSession() as session:
        # 1. Authenticate (Single main user)
        print("Authenticating...")
        token = await get_auth_token(session)
        if not token:
            print("CRITICAL: Authentication failed. Aborting test.")
            return

        print(f"Authenticated successfully. Token starts with: {token[:10]}...")
        
        # 2. Launch users
        tasks = []
        for i in range(num_users):
            task = asyncio.create_task(user_session(i, session, token, duration))
            tasks.append(task)
            
        # 3. Wait for completion
        await asyncio.gather(*tasks)

    # 4. Report
    print_report(duration)

def print_report(duration):
    total_reqs = stats["requests"]
    if total_reqs == 0:
        print("No requests completed.")
        return

    rps = total_reqs / duration
    avg_latency = statistics.mean(stats["latencies"]) if stats["latencies"] else 0
    p95_latency = statistics.quantiles(stats["latencies"], n=20)[18] if len(stats["latencies"]) >= 20 else 0
    
    print("\n" + "="*50)
    print("STRESS TEST REPORT")
    print("="*50)
    print(f"Total Requests: {total_reqs}")
    print(f"Duration:       {duration}s")
    print(f"RPS (Avg):      {rps:.2f}")
    print(f"Error Rate:     {(stats['errors']/total_reqs)*100:.2f}%")
    print(f"Avg Latency:    {avg_latency:.2f} ms")
    print(f"P95 Latency:    {p95_latency:.2f} ms")
    print("-" * 50)
    print("Status Codes:")
    for code, count in stats["codes"].items():
        print(f"  {code}: {count}")
    print("-" * 50)
    print("Endpoint Performance:")
    print(f"{'Endpoint':<20} | {'Reqs':<8} | {'Errs':<6} | {'Avg (ms)':<10}")
    for name, data in stats["endpoints"].items():
        avg = statistics.mean(data["latencies"]) if data["latencies"] else 0
        print(f"{name:<20} | {data['count']:<8} | {data['errors']:<6} | {avg:<10.2f}")
    print("="*50)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--users", type=int, default=10, help="Number of concurrent users")
    parser.add_argument("--duration", type=int, default=10, help="Test duration in seconds")
    args = parser.parse_args()
    
    asyncio.run(run_stress_test(args.users, args.duration))
