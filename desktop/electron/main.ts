import { app, BrowserWindow, Tray, Menu, nativeImage, Notification, shell, globalShortcut, ipcMain } from 'electron'
import { join } from 'path'

const isDev = process.env.NODE_ENV !== 'production' || !app.isPackaged

let mainWindow: BrowserWindow | null = null
let tray: Tray | null = null

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1440,
    height: 920,
    minWidth: 1120,
    minHeight: 720,
    title: 'OhMyWu',
    backgroundColor: '#f5efe5',
    show: false,
    webPreferences: {
      preload: join(__dirname, 'preload.js'),
      contextIsolation: true,
      sandbox: false,
    },
  })

  if (isDev) {
    mainWindow.loadURL('http://127.0.0.1:5173')
    mainWindow.webContents.openDevTools()
  } else {
    mainWindow.loadFile(join(__dirname, '../dist/index.html'))
  }

  mainWindow.once('ready-to-show', () => {
    mainWindow?.show()
  })

  mainWindow.on('close', (event) => {
    if (tray) {
      event.preventDefault()
      mainWindow?.hide()
    }
  })

  mainWindow.on('closed', () => {
    mainWindow = null
  })

  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url)
    return { action: 'deny' }
  })
}

function createTray() {
  const icon = createDefaultIcon()
  tray = new Tray(icon)

  const contextMenu = Menu.buildFromTemplate([
    {
      label: '显示窗口',
      click: () => {
        mainWindow?.show()
        mainWindow?.focus()
      },
    },
    {
      label: '隐藏窗口',
      click: () => {
        mainWindow?.hide()
      },
    },
    { type: 'separator' },
    {
      label: '关于 OhMyWu',
      click: () => {
        mainWindow?.show()
        mainWindow?.webContents.send('navigate', '/overview')
      },
    },
    { type: 'separator' },
    {
      label: '退出',
      click: () => {
        tray?.destroy()
        tray = null
        app.quit()
      },
    },
  ])

  tray.setToolTip('OhMyWu - 智能电脑管家')
  tray.setContextMenu(contextMenu)

  tray.on('click', () => {
    if (mainWindow?.isVisible()) {
      mainWindow.hide()
    } else {
      mainWindow?.show()
      mainWindow?.focus()
    }
  })

  tray.on('double-click', () => {
    mainWindow?.show()
    mainWindow?.focus()
  })
}

function createDefaultIcon(): nativeImage {
  const size = 32
  const canvas = Buffer.alloc(size * size * 4)

  for (let i = 0; i < size * size; i++) {
    const x = i % size
    const y = Math.floor(i / size)
    const idx = i * 4

    const centerX = size / 2
    const centerY = size / 2
    const radius = size / 2 - 2
    const dist = Math.sqrt((x - centerX) ** 2 + (y - centerY) ** 2)

    if (dist <= radius) {
      canvas[idx] = 201
      canvas[idx + 1] = 169
      canvas[idx + 2] = 110
      canvas[idx + 3] = 255
    } else {
      canvas[idx] = 0
      canvas[idx + 1] = 0
      canvas[idx + 2] = 0
      canvas[idx + 3] = 0
    }
  }

  return nativeImage.createFromBuffer(canvas, { width: size, height: size })
}

function showNotification(title: string, body: string) {
  if (Notification.isSupported()) {
    const notification = new Notification({
      title,
      body,
      icon: createDefaultIcon(),
    })
    notification.on('click', () => {
      mainWindow?.show()
      mainWindow?.focus()
    })
    notification.show()
  }
}

function setupIpcHandlers() {
  ipcMain.handle('window:minimize', () => {
    mainWindow?.minimize()
  })

  ipcMain.handle('window:maximize', () => {
    if (mainWindow?.isMaximized()) {
      mainWindow.unmaximize()
    } else {
      mainWindow?.maximize()
    }
  })

  ipcMain.handle('window:close', () => {
    if (tray) {
      mainWindow?.hide()
    } else {
      mainWindow?.close()
    }
  })

  ipcMain.handle('window:isMaximized', () => {
    return mainWindow?.isMaximized() ?? false
  })

  ipcMain.handle('notification:show', (_event, title: string, body: string) => {
    showNotification(title, body)
  })

  ipcMain.handle('app:getVersion', () => {
    return app.getVersion()
  })

  ipcMain.handle('app:quit', () => {
    tray?.destroy()
    tray = null
    app.quit()
  })
}

app.whenReady().then(() => {
  setupIpcHandlers()
  createWindow()
  createTray()

  globalShortcut.register('CommandOrControl+Shift+W', () => {
    if (mainWindow?.isVisible()) {
      mainWindow.hide()
    } else {
      mainWindow?.show()
      mainWindow?.focus()
    }
  })

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    } else {
      mainWindow?.show()
    }
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    tray?.destroy()
    tray = null
    app.quit()
  }
})

app.on('will-quit', () => {
  globalShortcut.unregisterAll()
  tray?.destroy()
  tray = null
})
