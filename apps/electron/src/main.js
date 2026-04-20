const { app, BrowserWindow } = require('electron')

const defaultUrl = process.env.OHMYWU_WEB_URL || 'http://127.0.0.1:5173'

function createWindow() {
  const mainWindow = new BrowserWindow({
    width: 1440,
    height: 920,
    minWidth: 1120,
    minHeight: 720,
    title: 'OhMyWu',
    backgroundColor: '#f5efe5',
    webPreferences: {
      contextIsolation: true,
      sandbox: true,
    },
  })

  mainWindow.loadURL(defaultUrl).catch(() => {
    const fallback = encodeURIComponent(`
      <html>
        <body style="font-family:Segoe UI,sans-serif;padding:32px;background:#f5efe5;color:#1f1b16">
          <h1>OhMyWu Electron Shell</h1>
          <p>Web UI is not reachable yet.</p>
          <p>Expected URL: ${defaultUrl}</p>
        </body>
      </html>
    `)
    mainWindow.loadURL(`data:text/html;charset=utf-8,${fallback}`)
  })
}

app.whenReady().then(() => {
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})
