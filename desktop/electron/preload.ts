import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('ohmywu', {
  invoke: (channel: string, ...args: unknown[]) => ipcRenderer.invoke(channel, ...args),
  on: (channel: string, callback: (...args: unknown[]) => void) => {
    const subscription = (_event: Electron.IpcRendererEvent, ...args: unknown[]) => callback(...args)
    ipcRenderer.on(channel, subscription)
    return () => ipcRenderer.removeListener(channel, subscription)
  },
  platform: process.platform,

  window: {
    minimize: () => ipcRenderer.invoke('window:minimize'),
    maximize: () => ipcRenderer.invoke('window:maximize'),
    close: () => ipcRenderer.invoke('window:close'),
    isMaximized: () => ipcRenderer.invoke('window:isMaximized') as Promise<boolean>,
  },

  notification: {
    show: (title: string, body: string) => ipcRenderer.invoke('notification:show', title, body),
  },

  app: {
    getVersion: () => ipcRenderer.invoke('app:getVersion') as Promise<string>,
    quit: () => ipcRenderer.invoke('app:quit'),
  },
})
