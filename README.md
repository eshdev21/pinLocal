# PinLocal

PinLocal is a desktop application for local image search, boards management, and AI-powered visual search. Built with Rust (Tauri), React, and Python, it runs entirely on your local machine, keeping your data private.

## Features
- **Local Search**: Fast database search of your images and folders.
- **AI Search**: Local vector-based visual/semantic search powered by SigLIP embeddings.
- **Boards Management**: Organize images into custom boards.
- **Workspace Tracker**: Keep track of local folders and sync changes in real time.

## Demo
Below is a screen recording demonstrating the application in action:

<video src="assets/demo.mp4" width="100%" controls autoplay loop muted></video>

## Tech Stack
- **Frontend**: React, TypeScript, Tailwind CSS, Lucide icons, Framer Motion, Zustand
- **Backend/Desktop wrapper**: Rust (Tauri v2)
- **AI Engine**: Python (SigLIP embeddings)
- **Database**: SQLite

## Getting Started

### Prerequisites
- [Bun](https://bun.sh/) (or Node.js)
- [Rust](https://www.rust-lang.org/)
- [Python 3.10+](https://www.python.org/)

### Development

1. Install frontend dependencies:
   ```bash
   bun install
   ```

2. Run the application in development mode:
   ```bash
   bun run tauri dev
   ```
