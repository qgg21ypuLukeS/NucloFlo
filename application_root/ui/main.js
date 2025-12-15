// ui/main.js

// --- Imports ---
const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const { spawn } = require('child_process');

// --- Window Creation Function ---
function createWindow() {
    const mainWindow = new BrowserWindow({
        width: 800,
        height: 600,
        webPreferences: {
            // Must point to the preload script
            preload: path.join(__dirname, 'preload.js'), 
            // Required for security: disables node features in the renderer
            nodeIntegration: false,
            contextIsolation: true, 
        },
    });

    // Load the HTML file
    mainWindow.loadFile('index.html');
    
    // Optional: Open DevTools for debugging
    // mainWindow.webContents.openDevTools();
}

// --- Application Lifecycle ---
app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
    // Standard cleanup for non-macOS platforms
    if (process.platform !== 'darwin') {
        app.quit();
    }
});

app.on('activate', () => {
    // Recreate window if app is activated (common on macOS)
    if (BrowserWindow.getAllWindows().length === 0) {
        createWindow();
    }
});


// ==========================================================
// --- IPC LISTENER: 1. Handle File Selection Dialog ---
// ==========================================================
ipcMain.on('select-file', (event) => {
    // Get the focused window to anchor the dialog
    const window = BrowserWindow.getFocusedWindow();
    if (!window) return;

    dialog.showOpenDialog(window, {
        title: 'Select FASTA Input File',
        properties: ['openFile'],
        filters: [
            { name: 'FASTA Sequence Files', extensions: ['fasta', 'fa', 'seq'] },
            { name: 'All Files', extensions: ['*'] }
        ]
    }).then(result => {
        if (!result.canceled && result.filePaths.length > 0) {
            const filePath = result.filePaths[0];
            
            // Send the selected file path back to the renderer process
            event.sender.send('selected-file', filePath);
            
            console.log(`[Electron] File selected: ${filePath}`);
        } else {
            console.log("[Electron] File selection cancelled.");
        }
    }).catch(err => {
        console.error("[Electron] File selection error:", err);
        event.sender.send('blast-job-error', `File selection failed: ${err.message}`);
    });
});


// ==========================================================
// --- IPC LISTENER: 2. Handle Rust Scheduler Execution ---
// ==========================================================
ipcMain.on('run-blast', (event, inputFilePath) => {
    
    // 1. Define the binary name
    const schedulerBinaryName = 'scheduler' + (process.platform === 'win32' ? '.exe' : ''); 

    // 2. Define the CORRECT path to the compiled executable.
    // ASSUMPTION: 'application_root' is the workspace root.
    // Path: 'ui' -> 'application_root' -> 'target/release' -> 'scheduler'
    const rustBinaryPath = path.join(
        __dirname, 
        '..',         // Up to application_root/
        'target', 
        'release', 
        schedulerBinaryName 
    );

    console.log(`[Electron] Attempting to launch scheduler: ${rustBinaryPath}`);
    console.log(`[Electron] Passing input file: ${inputFilePath}`);

    try {
        // 3. Spawn the Rust process
        const rustProcess = spawn(rustBinaryPath, [inputFilePath]);

        // --- Handle Output and Errors ---

        rustProcess.stdout.on('data', (data) => {
            const output = data.toString();
            console.log(`[Rust STDOUT]: ${output}`);
            event.sender.send('blast-job-output', output);
        });

        rustProcess.stderr.on('data', (data) => {
            const error = data.toString();
            console.error(`[Rust STDERR]: ${error}`);
            event.sender.send('blast-job-error', error);
        });

        rustProcess.on('close', (code) => {
            console.log(`[Rust] Process finished with exit code ${code}`);
            event.sender.send('blast-job-status', `Process finished with code ${code}`);
        });

        rustProcess.on('error', (err) => {
            // This is the critical handler for ENOENT (File Not Found)
            console.error(`[Rust SPAWN ERROR]: Failed to start process: ${err.message}`);
            
            let errorMessage = `Failed to execute scheduler: ${err.message}.`;
            if (err.code === 'ENOENT') {
                 errorMessage += `\nError: The executable was not found at the expected path. Please ensure you have run 'cargo build --release -p scheduler' from the 'application_root/' folder.`;
            }
            errorMessage += `\nAttempted Path: ${rustBinaryPath}`;
            
            event.sender.send('blast-job-error', errorMessage);
        });

    } catch (error) {
        console.error(`[CRITICAL SPAWN ERROR]: ${error}`);
        event.sender.send('blast-job-error', `Critical Electron error during spawn: ${error.message}`);
    }
});