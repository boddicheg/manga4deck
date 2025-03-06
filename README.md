# Manga4Deck

A manga reader for Kavita on Steam Deck.

## Development Setup

### Prerequisites

- Node.js (v16 or higher)
- Python 3.8 or higher
- pip (Python package manager)
- Rust and Cargo

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/manga4deck.git
   cd manga4deck
   ```

2. Install Node.js dependencies:
   ```bash
   npm install
   ```

3. Install Python dependencies:
   ```bash
   cd src-tauri/backend
   pip install -r req.txt
   cd ../..
   ```

## Development

### Starting the Development Environment

To start the development environment with the Python backend running:

```bash
npm run dev-with-backend
```

This will:
1. Build the Python backend if needed
2. Start the Python backend server
3. Start the Tauri development environment

### Building the Python Backend Separately

If you need to rebuild the Python backend:

```bash
npm run prepare-backend
```

### Building for Production

To build the entire application for production:

```bash
npm run build-all
```

This will:
1. Build the Python backend
2. Build the frontend
3. Build the Tauri application

## Troubleshooting

### Python Backend Issues

If you encounter issues with the Python backend:

1. Check if there are any processes using port 11337:
   ```bash
   # On Linux/macOS
   lsof -i :11337
   
   # On Windows
   netstat -ano | findstr :11337
   ```

2. Kill any processes using port 11337:
   ```bash
   # On Linux/macOS
   lsof -ti:11337 | xargs kill -9
   
   # On Windows
   for /f "tokens=5" %a in ('netstat -aon ^| findstr :11337') do taskkill /F /PID %a
   ```

3. Rebuild the Python backend:
   ```bash
   npm run prepare-backend
   ```

### Tauri Development Issues

If you encounter issues with the Tauri development environment:

1. Kill any processes using port 1420:
   ```bash
   npm run kill
   ```

2. Restart the development environment:
   ```bash
   npm run dev-with-backend
   ```

manga4deck
==========

Reader for SteamDeck, developing only for reading manga using Kavita selfhosted server

<a name="download" href="https://raw.githubusercontent.com/boddicheg/manga4deck/main/installer.desktop">Download .desktop</a>

Screenshots:
----
![pic1](assets/manga4deck.jpg)
![pic1](assets/manga4deck_2.jpg)

Steam Deck mappings:
----
Buttons:
- (A) - Keyboard -> Enter 
- (B) - Keyboard -> Backspace 
- (X) - Keyboard -> 'F2' - bind for caching manga serie, works only on manga volumes page 
- (Y) - Keyboard -> 'F1' - set focused volume as completed

Cross:
- (<) - Keboard -> Left Arrow 
- (>) - Keboard -> Rigth Arrow 
- (^) - Keboard -> Up Arrow 
- (v) - Keboard -> Down Arrow 


Deps
----
```
sudo apt-get install python3-pil python3-pil.imagetk
sudo apt-get install python3-tk
```
mac
```
brew install python-tk
```
