import argparse
import json
import os
import sqlite3
import sys
import warnings
from pathlib import Path
from typing import Optional

import torch
from transformers import AutoProcessor, Siglip2Model
import transformers

# Mute noisy libraries
transformers.logging.set_verbosity_error()
os.environ["TOKENIZERS_PARALLELISM"] = "false"

# Import local modules
import embeddings
import search
import numpy as np
from tqdm import tqdm

def log(msg: str, level: str = "INFO"):
    """Protocol-based logging for Rust consumption"""
    print(f"[LOG] {level}: {msg}", file=sys.stderr, flush=True)

# Force tqdm to be talkative
if hasattr(tqdm, "monitor_interval"):
    tqdm.monitor_interval = 0

# Suppress warnings
from PIL import Image
Image.MAX_IMAGE_PIXELS = None
warnings.filterwarnings('ignore', category=Image.DecompressionBombWarning)

def setup_device(forced_device: Optional[str]=None):
    if forced_device == 'cuda':
        if torch.cuda.is_available():
            return torch.device('cuda'), "NVIDIA GPU (CUDA)"
        return torch.device('cpu'), "CUDA forced but not available, using CPU"
    
    if forced_device == 'dml':
        try:
            import torch_directml
            if torch_directml.is_available():
                return torch_directml.device(), "AMD/Intel GPU (DirectML)"
        except ImportError:
            pass
        return torch.device('cpu'), "DirectML forced but not available, using CPU"
        
    if torch.cuda.is_available():
        return torch.device('cuda'), "NVIDIA GPU (CUDA)"
    
    return torch.device('cpu'), "CPU"

class Sidecar:
    def __init__(self):
        self.model = None
        self.processor = None
        self.device = None
        self.max_patches = 256
        self.current_model_id = None

    def load_model(self, model_id: str, device_type: str = "auto", max_patches: int = 256):
        try:
            self.max_patches = max_patches
            self.device, dev_msg = setup_device(None if device_type == "auto" else device_type)
            log(f"Computation Device: {dev_msg}")
            
            dtype = torch.float16 if self.device.type == 'cuda' else torch.float32
            
            log(f"Loading weights for {model_id}...")
            self.processor = AutoProcessor.from_pretrained(model_id)
            
            # Use SDPA if available for better performance
            attn_implementation = 'sdpa' if hasattr(torch.nn.functional, 'scaled_dot_product_attention') else 'eager'
            
            self.model = Siglip2Model.from_pretrained(
                model_id, 
                torch_dtype=dtype, 
                attn_implementation=attn_implementation
            ).to(self.device)
            self.model.eval()
            
            self.current_model_id = model_id
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
            log("Model weights successfully loaded into memory.")
            return True
        except Exception as e:
            log(f"Model Load Error: {e}", level="ERROR")
            return False

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["auto", "manual"], default="manual")
    args = parser.parse_args()

    # In our new architecture, 'auto' mode at the script level is handled by setup.py
    # and interface.py is always called in 'manual' mode by the sidecar manager.
    
    sidecar = Sidecar()
    print("READY", flush=True)

    for line in sys.stdin:
        try:
            req = json.loads(line)
            action = req.get("action")
            
            if action == "load_model":
                success = sidecar.load_model(
                    req.get("model_id"), 
                    req.get("device", "auto"), 
                    req.get("max_patches", 256)
                )
                print(json.dumps({"status": "ok" if success else "error"}), flush=True)
                
            elif action == "encode_text":
                q_emb = search.get_query_embedding(
                    req.get("query_text"), None, 
                    sidecar.device, sidecar.processor, sidecar.model, 
                    workspace_root=req.get("workspace_root")
                )
                if q_emb is None:
                    print(json.dumps({"status": "error", "message": "Encoding failed"}), flush=True)
                else:
                    print(json.dumps({"status": "ok", "embedding": q_emb.flatten().tolist()}), flush=True)

            elif action == "index":
                embeddings.index_files(
                    sidecar.device, sidecar.processor, sidecar.model, 
                    req["db_path"], 
                    progress_callback=lambda d: print(json.dumps(d), flush=True),
                    workspace_root=req.get("workspace_root")
                )
                
            elif action == "search":
                q_emb = search.get_query_embedding(
                    req.get("query_text"), req.get("query_image_path"), 
                    sidecar.device, sidecar.processor, sidecar.model, 
                    workspace_root=req.get("workspace_root")
                )
                if q_emb is None:
                    print(json.dumps({"status": "error", "message": "Query failed"}), flush=True)
                else:
                    results = search.perform_search(req["db_path"], q_emb)
                    print(json.dumps({"status": "ok", "results": results}), flush=True)
                
                    
            elif action == "rescore":
                q_emb = search.get_query_embedding(
                    req.get("query_text"), None, 
                    sidecar.device, sidecar.processor, sidecar.model, 
                    workspace_root=req.get("workspace_root")
                )
                if q_emb is None:
                    print(json.dumps({"status": "error", "message": "Query failed"}), flush=True)
                else:
                    results = []
                    with sqlite3.connect(req["db_path"]) as conn:
                        cursor = conn.cursor()
                        for path in req["paths"]:
                            cursor.execute('SELECT embedding FROM embeddings WHERE filepath=?', (path,))
                            row = cursor.fetchone()
                            if row and row[0]:
                                emb_val = np.frombuffer(row[0], dtype=np.float32)
                                score = float(np.dot(emb_val, q_emb.T).squeeze())
                                results.append({"path": path, "score": score})
                    results.sort(key=lambda x: x["score"], reverse=True)
                    print(json.dumps({"status": "ok", "results": results}), flush=True)

            elif action == "exit":
                break
                
        except Exception as e:
            print(json.dumps({"status": "error", "message": str(e)}), flush=True)

if __name__ == "__main__":
    main()
