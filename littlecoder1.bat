@echo off

set LLAMACPP_BASE_URL=http://127.0.0.1:8080/v1
set LLAMACPP_API_KEY=noop

cd /d "%~dp0"

little-coder --model llamacpp/qwen3.6-35b-a3b

pause