import sqlite3
import os

main_db_path = r"C:\Users\star\AppData\Roaming\com.eshdev.pinlocal\data\app.db"
ai_db_path_z = r"C:\Users\star\AppData\Roaming\com.eshdev.pinlocal\cache\ai\z_6d089e9a454ddc51.db"

def check_ai_sync():
    # 1. Get image paths from Main DB for board 'z'
    main_conn = sqlite3.connect(main_db_path)
    # Finding board 'z' ID first
    board_id = main_conn.execute("SELECT id FROM boards WHERE name = 'z'").fetchone()[0]
    db_images = {row[0].lower() for row in main_conn.execute("SELECT path FROM images WHERE board_id = ? AND is_missing = 0", (board_id,)).fetchall()}
    main_conn.close()

    # 2. Get image paths from AI DB
    ai_conn = sqlite3.connect(ai_db_path_z)
    ai_images = {row[0].lower() for row in ai_conn.execute("SELECT filepath FROM embeddings").fetchall()}
    ai_conn.close()

    print(f"Main DB (z): {len(db_images)} images")
    print(f"AI DB (z): {len(ai_images)} embeddings")

    only_in_ai = ai_images - db_images
    only_in_main = db_images - ai_images

    if only_in_ai:
        print(f"\n[!] {len(only_in_ai)} orphaned embeddings in AI DB (not in Main DB):")
        for p in list(only_in_ai)[:10]: print(f"    - {p}")
        if len(only_in_ai) > 10: print("    ...")

    if only_in_main:
        print(f"\n[!] {len(only_in_main)} images missing embeddings (in Main DB but not AI DB):")
        for p in list(only_in_main)[:10]: print(f"    - {p}")
        if len(only_in_main) > 10: print("    ...")
        
    if not only_in_ai and not only_in_main:
        print("\n[+] AI DB is perfectly in sync with Main DB.")

if __name__ == "__main__":
    check_ai_sync()
