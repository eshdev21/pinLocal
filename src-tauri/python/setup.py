import argparse
import os
import subprocess
import sys
from pathlib import Path

def check_nvidia_gpu():
    try:
        subprocess.run(["nvidia-smi"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        return True
    except:
        return False

def get_cuda_index(cuda_tag):
    # Mapping tags to torch whl index
    # Tags: cu118, cu121, cu124, cu126, cu130
    # Note: cu130 doesn't exist yet officially as a whl index, 
    # but we'll follow the pattern or fallback to default.
    # For now cu124/cu121/cu118 are standard.
    return f"https://download.pytorch.org/whl/{cuda_tag}"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--venv", required=True, help="Path to venv directory")
    parser.add_argument("--hardware", choices=["auto", "nvidia", "amd", "cpu"], default="auto")
    parser.add_argument("--cuda", default="cu124", help="CUDA version tag (e.g. cu124)")
    parser.add_argument("--link-mode", choices=["copy", "symlink", "hardlink"], default="copy")
    args = parser.parse_args()

    venv_dir = Path(args.venv)
    hardware = args.hardware

    print(f"--- AI Setup: Starting ---")
    print(f"Target Venv: {venv_dir}")
    print(f"Hardware Preference: {hardware}")

    # 1. Hardware Detection
    if hardware == "auto":
        if check_nvidia_gpu():
            hardware = "nvidia"
            print("Auto-detected Hardware: NVIDIA GPU")
        else:
            # Check for DML later in interface, but for installation CPU is baseline
            # unless we specifically want torch-directml
            hardware = "cpu"
            print("Auto-detected Hardware: CPU / Other")

    # 2. Venv Creation/Validation
    venv_python = venv_dir / "Scripts" / "python.exe" if os.name == "nt" else venv_dir / "bin" / "python"
    
    if not venv_python.exists():
        print(f"Virtual environment missing or broken. Creating/Recreating at {venv_dir}...")
        # If directory exists but no python, clear it first to avoid uv conflicts
        if venv_dir.exists():
            import shutil
            try:
                shutil.rmtree(venv_dir)
            except:
                pass
        
        subprocess.run(["uv", "venv", str(venv_dir)], check=True)

    # 3. Dependency Installation
    python_dir = Path(__file__).parent.absolute()
    req_file = python_dir / "requirements.txt"
    
    if not req_file.exists():
        # Create default requirements.txt if missing
        with open(req_file, "w") as f:
            f.write("torch\ntorchvision\ntransformers\npillow\nnumpy\nopencv-python\ntqdm\n")

    print(f"Installing dependencies (Hardware: {hardware}, Mode: {args.link_mode})...")
    
    env = os.environ.copy()
    env["VIRTUAL_ENV"] = str(venv_dir)

    base_cmd = ["uv", "pip", "install", "-r", str(req_file), f"--link-mode={args.link_mode}"]
    
    if hardware == "nvidia":
        index_url = get_cuda_index(args.cuda)
        print(f"Using Extra Index URL: {index_url}")
        install_cmd = base_cmd + ["--extra-index-url", index_url]
    elif hardware == "amd":
        # Install CPU torch + torch-directml (directml wraps CPU torch)
        print("Using AMD/DirectML extra index...")
        install_cmd = base_cmd + ["--extra-index-url", "https://download.pytorch.org/whl/cpu", "torch-directml"]
    else:
        # CPU
        install_cmd = base_cmd + ["--extra-index-url", "https://download.pytorch.org/whl/cpu"]

    try:
        subprocess.run(install_cmd, env=env, check=True)
    except subprocess.CalledProcessError as e:
        print(f"Installation failed with primary command. Retrying with 'copy' mode and default index...")
        # Fallback to copy mode and default index if specific one fails
        fallback_cmd = ["uv", "pip", "install", "-r", str(req_file), "--link-mode=copy"]
        if hardware == "amd":
            fallback_cmd.append("torch-directml")
        subprocess.run(fallback_cmd, env=env, check=True)

    # 4. Final Validation
    print("Verifying installation...")
    venv_python = venv_dir / "Scripts" / "python.exe" if os.name == "nt" else venv_dir / "bin" / "python"
    
    verify_script = "import torch; print(f'Torch version: {torch.__version__}'); print(f'CUDA Available: {torch.cuda.is_available()}')"
    try:
        subprocess.run([str(venv_python), "-c", verify_script], check=True)
    except:
        print("Validation failed! Environment might be broken.")
        sys.exit(1)

    print("--- AI Setup: Success ---")

if __name__ == "__main__":
    main()
