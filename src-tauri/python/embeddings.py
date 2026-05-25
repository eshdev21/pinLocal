import torch
import numpy as np
from PIL import Image
from pathlib import Path
import sqlite3
import sys
from typing import List, Tuple, Optional, Callable

def normalize_embedding(features) -> torch.Tensor:
    if not isinstance(features, torch.Tensor):
        features = features.pooler_output if hasattr(features, 'pooler_output') else features[1]
    return features / torch.linalg.norm(features, ord=2, dim=-1, keepdim=True)

def index_files(
    device: torch.device, 
    processor, 
    model, 
    db_path: str, 
    batch_size: int = 10, 
    progress_callback: Optional[Callable] = None, 
    max_num_patches: int = 256, 
    workspace_root: Optional[str] = None
):
    with sqlite3.connect(db_path) as conn:
        cursor = conn.cursor()
        
        # Target rows where embedding is NULL
        cursor.execute('SELECT filepath, modified_at FROM embeddings WHERE embedding IS NULL')
        paths_to_process = cursor.fetchall()
        
        if not paths_to_process:
            if progress_callback:
                progress_callback({"status": "ok", "done": 0, "message": "Database is up to date."})
            return

        total = len(paths_to_process)
        processed = 0
        root_path = Path(workspace_root) if workspace_root else None
        
        for i in range(0, total, batch_size):
            batch = paths_to_process[i:i+batch_size]
            batch_images = []
            valid_data = []
            
            for path_str, mtime in batch:
                try:
                    path_obj = Path(path_str)
                    load_path = path_obj if path_obj.is_absolute() or not root_path else root_path / path_obj
                    
                    if not load_path.exists():
                        continue
                        
                    img = Image.open(load_path).convert('RGB')
                    batch_images.append(img)
                    valid_data.append((path_str, mtime))
                except Exception as e:
                    print(f"DEBUG: Load error for {path_str}: {e}", file=sys.stderr)
                    continue
                
            if not batch_images:
                continue
            
            try:
                inputs = processor(images=batch_images, return_tensors='pt', max_num_patches=max_num_patches).to(device)
                with torch.no_grad():
                    features = model.get_image_features(**inputs)
                    features = normalize_embedding(features)
                
                for idx, (path, mtime) in enumerate(valid_data):
                    emb = features[idx].cpu().numpy().astype(np.float32).tobytes()
                    cursor.execute('UPDATE embeddings SET embedding=? WHERE filepath=?', (emb, path))
                
                processed += len(valid_data)
                conn.commit()
                if progress_callback:
                    progress_callback({"status": "progress", "done": processed, "total": total})
            except Exception as e:
                print(f"Batch error: {e}", file=sys.stderr)
                
        if progress_callback:
            progress_callback({"status": "ok", "done": processed})
