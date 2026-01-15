import psutil
import time
import csv
import subprocess
import datetime
import os
import argparse

# Configuration
LOG_FILE = "server_monitor_log.csv"
DB_USER = "avalencia"
DB_NAME = "tfactu"
DB_HOST = "localhost"

def get_db_connections():
    """Get number of active connections to PostgreSQL."""
    try:
        # Using psql to avoid needing psycopg2 driver which might not be installed
        env = os.environ.copy()
        env["PGPASSWORD"] = "Jacobo23"
        cmd = [
            "psql", 
            "-h", DB_HOST, 
            "-U", DB_USER, 
            "-d", DB_NAME, 
            "-t", 
            "-c", "SELECT count(*) FROM pg_stat_activity;"
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, env=env)
        if result.returncode == 0:
            return int(result.stdout.strip())
        else:
            return -1 # Error
    except Exception as e:
        return -1

def get_server_pid(process_name="lum_rust_backend"):
    """Find the PID of the rust server."""
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            # Check name or command line arguments
            if process_name in proc.info['name']:
                return proc
            if proc.info['cmdline'] and any(process_name in arg for arg in proc.info['cmdline']):
                return proc
        except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
            pass
    return None

def monitor(interval=1, duration=None):
    print(f"Starting Server Monitor...")
    print(f"Logging to {LOG_FILE}")
    
    server_proc = get_server_pid("lum_rust") # Adjust name if needed
    if server_proc:
        print(f"Found Server Process: PID {server_proc.pid} ({server_proc.name()})")
    else:
        print("WARNING: Server process not found! Monitoring system only.")

    with open(LOG_FILE, 'w', newline='') as csvfile:
        fieldnames = ['timestamp', 'cpu_percent', 'memory_percent', 'memory_used_mb', 
                      'server_cpu', 'server_mem_mb', 'db_connections', 'net_sent_mb', 'net_recv_mb']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()

        start_time = time.time()
        net_io_start = psutil.net_io_counters()

        try:
            while True:
                if duration and (time.time() - start_time) > duration:
                    break
                
                now = datetime.datetime.now().isoformat()
                cpu_pct = psutil.cpu_percent(interval=None)
                mem = psutil.virtual_memory()
                
                # Server specific metrics
                server_cpu = 0.0
                server_mem = 0.0
                if server_proc:
                    try:
                        with server_proc.oneshot():
                            server_cpu = server_proc.cpu_percent(interval=None)
                            server_mem = server_proc.memory_info().rss / 1024 / 1024
                    except psutil.NoSuchProcess:
                        server_proc = get_server_pid("lum_rust") # Try to find it again
                
                # Network I/O
                net_io_now = psutil.net_io_counters()
                sent_mb = (net_io_now.bytes_sent - net_io_start.bytes_sent) / 1024 / 1024
                recv_mb = (net_io_now.bytes_recv - net_io_start.bytes_recv) / 1024 / 1024
                
                # DB
                db_conn = get_db_connections()

                row = {
                    'timestamp': now,
                    'cpu_percent': cpu_pct,
                    'memory_percent': mem.percent,
                    'memory_used_mb': mem.used / 1024 / 1024,
                    'server_cpu': server_cpu,
                    'server_mem_mb': server_mem,
                    'db_connections': db_conn,
                    'net_sent_mb': sent_mb,
                    'net_recv_mb': recv_mb
                }
                
                writer.writerow(row)
                csvfile.flush()
                
                print(f"\rCPU: {cpu_pct}% | RAM: {mem.percent}% | AppCPU: {server_cpu}% | DB Conn: {db_conn}", end="")
                
                time.sleep(interval)
                
        except KeyboardInterrupt:
            print("\nMonitoring stopped.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--duration", type=int, help="Duration in seconds", default=None)
    args = parser.parse_args()
    monitor(duration=args.duration)
