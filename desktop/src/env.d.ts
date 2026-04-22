/// <reference types="vite/client" />

interface OhMyWuAPI {
  invoke: (channel: string, ...args: unknown[]) => Promise<unknown>
  on: (channel: string, callback: (...args: unknown[]) => void) => () => void
  platform: NodeJS.Platform
  window: {
    minimize: () => Promise<void>
    maximize: () => Promise<void>
    close: () => Promise<void>
    isMaximized: () => Promise<boolean>
  }
  notification: {
    show: (title: string, body: string) => Promise<void>
  }
  app: {
    getVersion: () => Promise<string>
    quit: () => Promise<void>
  }
}

declare global {
  interface Window {
    ohmywu: OhMyWuAPI
  }
}

export {}
