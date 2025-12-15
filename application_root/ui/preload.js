// ui/preload.js

const { contextBridge, ipcRenderer } = require('electron');

// Expose the API to the renderer process under 'electronAPI' namespace
contextBridge.exposeInMainWorld('electronAPI', {
    // =======================================================
    // Functions the Renderer (HTML) calls to send messages to Main
    // =======================================================
    
    // 1. Request to open the file selection dialog (returns a Promise)
    selectFile: () => {
        return new Promise((resolve) => {
            ipcRenderer.send('select-file');
            
            // Listen for the response once
            ipcRenderer.once('selected-file', (event, filePath) => {
                resolve(filePath);
            });
            
            // Handle cancellation
            ipcRenderer.once('file-selection-cancelled', () => {
                resolve(null);
            });
        });
    },

    // 2. Start a BLAST job with the given input path
    startJob: (jobConfig) => {
        ipcRenderer.send('run-blast', jobConfig.inputPath);
    },

    // =======================================================
    // Functions for the Renderer to receive messages from Main
    // =======================================================

    // Listens for stdout output from the Rust process
    onOutput: (callback) => {
        ipcRenderer.on('blast-job-output', (event, output) => callback(output));
    },

    // Listens for errors (both spawn errors and dialog errors)
    onError: (callback) => {
        ipcRenderer.on('blast-job-error', (event, error) => callback(error));
    },

    // Listens for the process completion status
    onStatus: (callback) => {
        ipcRenderer.on('blast-job-status', (event, status) => callback(status));
    },
    
    // Remove listeners (cleanup)
    removeAllListeners: () => {
        ipcRenderer.removeAllListeners('blast-job-output');
        ipcRenderer.removeAllListeners('blast-job-error');
        ipcRenderer.removeAllListeners('blast-job-status');
    }
});