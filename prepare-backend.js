#!/usr/bin/env node

import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { fileURLToPath } from 'url';

// Get the directory name
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configuration
const backendDir = path.join(__dirname, 'src-tauri', 'backend');
const distDir = path.join(backendDir, 'dist');
const targetExe = path.join(distDir, 'app.exe');
const pythonCmd = os.platform() === 'win32' ? 'python' : 'python3';

console.log('🚀 Preparing Python backend...');

// Ensure the dist directory exists
if (!fs.existsSync(distDir)) {
  console.log('📁 Creating dist directory...');
  fs.mkdirSync(distDir, { recursive: true });
}

// Install Python dependencies
try {
  console.log('📦 Installing Python dependencies...');
  execSync(`cd "${backendDir}" && pip install -r req.txt`, { stdio: 'inherit' });
} catch (error) {
  console.error('❌ Failed to install Python dependencies:', error.message);
  process.exit(1);
}

// Build the Python backend
try {
  console.log('🔨 Building Python backend...');
  execSync(`cd "${backendDir}" && ${pythonCmd} build.py`, { stdio: 'inherit' });
} catch (error) {
  console.error('❌ Failed to build Python backend:', error.message);
  process.exit(1);
}

// Verify the executable was created
if (!fs.existsSync(targetExe)) {
  console.error(`❌ Failed to find built executable at ${targetExe}`);
  process.exit(1);
}

console.log('✅ Python backend prepared successfully!'); 