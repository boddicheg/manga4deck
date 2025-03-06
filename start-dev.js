#!/usr/bin/env node

import { spawn, execSync } from 'child_process';
import path from 'path';
import fs from 'fs';
import os from 'os';
import { fileURLToPath } from 'url';

// Get the directory name
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configuration
const backendDir = path.join(__dirname, 'src-tauri', 'backend');
const distDir = path.join(backendDir, 'dist');
const targetExe = path.join(distDir, 'app.exe');

// Kill any existing processes on port 11337 (Python backend)
try {
  console.log('🔄 Checking for existing Python backend processes...');
  if (os.platform() === 'win32') {
    execSync('for /f "tokens=5" %a in (\'netstat -aon ^| findstr :11337\') do taskkill /F /PID %a', { stdio: 'ignore' });
  } else {
    execSync('lsof -ti:11337 | xargs kill -9 || true', { stdio: 'ignore' });
  }
} catch (error) {
  // Ignore errors, as there might not be any processes to kill
}

// Ensure the backend executable exists
if (!fs.existsSync(targetExe)) {
  console.log('⚠️ Backend executable not found, building it first...');
  try {
    execSync('node prepare-backend.js', { stdio: 'inherit' });
  } catch (error) {
    console.error('❌ Failed to prepare backend:', error.message);
    process.exit(1);
  }
}

// Start the Python backend
console.log('🚀 Starting Python backend...');
const pythonProcess = spawn(targetExe, [], {
  stdio: 'inherit',
  detached: true
});

// Don't wait for the Python process to exit
pythonProcess.unref();

// Start the Tauri development process
console.log('🚀 Starting Tauri development environment...');
try {
  execSync('npm run tauri dev', { stdio: 'inherit' });
} catch (error) {
  console.error('❌ Tauri development process exited with an error:', error.message);
}

// When the Tauri process exits, kill the Python backend
console.log('🛑 Stopping Python backend...');
if (os.platform() === 'win32') {
  execSync('for /f "tokens=5" %a in (\'netstat -aon ^| findstr :11337\') do taskkill /F /PID %a', { stdio: 'ignore' });
} else {
  execSync('lsof -ti:11337 | xargs kill -9 || true', { stdio: 'ignore' });
} 