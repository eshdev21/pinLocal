import sqlite3
import os
from pathlib import Path

db_path = r"C:\Users\star\AppData\Roaming\com.eshdev.pinlocal\data\app.db"
folders = [
    r"c:\users\star\pictures\aidevert",
    r"c:\users\star\pictures\z"
]

def check_db_fs_sync_recursive():
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    print(f"Checking database: {db_path}")
    print("-" * 50)

    for folder_path in folders:
        folder_path = folder_path.replace("\\", "/").lower()
        print(f"\nProcessing Folder (Recursive): {folder_path}")
        
        # Get boards under this folder
        cursor.execute("SELECT id, name, path FROM boards WHERE path LIKE ?", (folder_path + '%',))
        boards = cursor.fetchall()
        
        if not boards:
            print(f"  [!] No boards found in DB for this root.")
            continue
            
        for board_id, board_name, board_full_path in boards:
            print(f"  Board: {board_name} ({board_full_path})")
            
            # DB Images
            cursor.execute("SELECT path FROM images WHERE board_id = ? AND is_missing = 0", (board_id,))
            db_images = {row[0].lower() for row in cursor.fetchall()}
            
            # FS Images (Recursive)
            fs_images = set()
            if os.path.exists(board_full_path):
                for root, dirs, files in os.walk(board_full_path):
                    for f in files:
                        if f.lower().endswith(('.png', '.jpg', '.jpeg', '.webp', '.gif')):
                            full_p = os.path.join(root, f).replace("\\", "/").lower()
                            fs_images.add(full_p)
            
            print(f"    - DB Images: {len(db_images)}")
            print(f"    - FS Images: {len(fs_images)}")
            
            only_in_db = db_images - fs_images
            only_in_fs = fs_images - db_images
            
            if only_in_db:
                print(f"    [!] {len(only_in_db)} images in DB but missing on FS:")
                for p in list(only_in_db)[:5]: print(f"        - {p}")
                if len(only_in_db) > 5: print("        ...")
            
            if only_in_fs:
                print(f"    [!] {len(only_in_fs)} images on FS but missing in DB:")
                for p in list(only_in_fs)[:5]: print(f"        - {p}")
                if len(only_in_fs) > 5: print("        ...")
                
            if not only_in_db and not only_in_fs:
                print("    [+] Perfectly in sync (Recursive).")

    conn.close()

if __name__ == "__main__":
    check_db_fs_sync_recursive()
