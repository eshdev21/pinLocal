import torch
import numpy as np
import sqlite3
from PIL import Image
from pathlib import Path
from typing import Optional, List
from embeddings import normalize_embedding

def get_query_embedding(
    text: Optional[str], 
    img_path: Optional[str], 
    device, 
    processor, 
    model, 
    max_patches=256, 
    workspace_root: Optional[str]=None
):
    if processor is None or model is None:
        return None

    with torch.no_grad():
        t_emb, i_emb = None, None
        if text:
            inputs = processor(text=[text.lower()], return_tensors='pt', padding='max_length', max_length=64).to(device)
            t_emb = normalize_embedding(model.get_text_features(**inputs)).cpu().numpy()
        if img_path:
            load_path = Path(img_path)
            if not load_path.is_absolute() and workspace_root:
                load_path = Path(workspace_root) / load_path
                
            img = Image.open(load_path).convert('RGB')
            inputs = processor(images=img, return_tensors='pt', max_num_patches=max_patches).to(device)
            i_emb = normalize_embedding(model.get_image_features(**inputs)).cpu().numpy()
            
        if t_emb is not None and i_emb is not None:
            return (t_emb + i_emb) / 2
        return t_emb if t_emb is not None else i_emb

def perform_search(db_path: str, q_emb: np.ndarray, top_k: int = 100):
    with sqlite3.connect(db_path) as conn:
        cursor = conn.cursor()
        cursor.execute('SELECT filepath, embedding FROM embeddings WHERE embedding IS NOT NULL')
        results = []
        batch_size = 1000
        while True:
            rows = cursor.fetchmany(batch_size)
            if not rows: break
            
            filepaths = [r[0] for r in rows]
            db_embs = np.array([np.frombuffer(r[1], dtype=np.float32) for r in rows])
            if db_embs.ndim == 1: db_embs = db_embs.reshape(1, -1)
            
            similarities = np.dot(db_embs, q_emb.T).squeeze()
            if similarities.ndim == 0: similarities = np.array([similarities])
            
            for i, sim in enumerate(similarities):
                results.append({"path": filepaths[i], "score": float(sim)})
                
        results.sort(key=lambda x: x["score"], reverse=True)
        return results[:top_k]
