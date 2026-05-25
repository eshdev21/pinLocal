import os
import sqlite3

db_path = r'C:\Users\star\AppData\Roaming\com.eshdev.pinlocal\data\app.db'

def verify_folders():
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("SELECT folder_path FROM workspace_folders WHERE workspace_id = '54f5cf8e-834b-4ed4-8f8d-3f21cc996dc6'")
    folders = cursor.fetchall()
    
    print(f"Checking {len(folders)} folders...")
    missing = 0
    for (path,) in folders:
        if not os.path.exists(path):
            print(f"MISSING: {path}")
            missing += 1
        else:
            print(f"OK: {path}")
            
    print(f"\nTotal missing: {missing}")
    conn.close()

if __name__ == "__main__":
    verify_folders()
