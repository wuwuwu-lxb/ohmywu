"use strict";
const electron = require("electron");
electron.contextBridge.exposeInMainWorld("ohmywu", {
  invoke: (channel, ...args) => electron.ipcRenderer.invoke(channel, ...args),
  on: (channel, callback) => {
    const subscription = (_event, ...args) => callback(...args);
    electron.ipcRenderer.on(channel, subscription);
    return () => electron.ipcRenderer.removeListener(channel, subscription);
  },
  platform: process.platform
});
