import sqlite3
import os
import json
from pathlib import Path

db_path = r'C:\Users\star\AppData\Roaming\com.eshdev.pinlocal\data\app.db'

def check_db():
    if not os.path.exists(db_path):
        print(f"Error: Database not found at {db_path}")
        return

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    print("--- Background Tasks ---")
    cursor.execute("SELECT id, task_type, status, message, progress_done, progress_total FROM background_tasks")
    tasks = cursor.fetchall()
    for task in tasks:
        print(task)

    print("\n--- Boards ---")
    cursor.execute("SELECT id, name, path, image_count, is_missing, needs_ai_sync FROM boards")
    boards = cursor.fetchall()
    for board in boards:
        print(board)

    print("\n--- Workspace Folders ---")
    cursor.execute("SELECT workspace_id, folder_path FROM workspace_folders")
    folders = cursor.fetchall()
    for folder in folders:
        print(folder)

    print("\n--- Image Count Per Board ---")
    cursor.execute("SELECT board_id, COUNT(*) FROM images GROUP BY board_id")
    counts = cursor.fetchall()
    for count in counts:
        print(count)

    conn.close()

if __name__ == "__main__":
    check_db()
