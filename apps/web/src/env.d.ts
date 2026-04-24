/// <reference types="vite/client" />

export interface OhmywuAPI {
  invoke: (channel: string, ...args: unknown[]) => Promise<unknown>
  on: (channel: string, callback: (...args: unknown[]) => void) => () => void
  platform: string
  window: {
    minimize: () => Promise<void>
    maximize: () => Promise<void>
    close: () => Promise<void>
    isMaximized: () => Promise<boolean>
  }
  notification: { show: (title: string, body: string) => Promise<void> }
  app: { getVersion: () => Promise<string>; quit: () => Promise<void> }
  pet: { enter: () => void; exit: () => void }
}

export interface PIXI {
  live2d: {
    Live2DModel: any
  }
  Application: any
}

declare global {
  interface Window {
    ohmywu?: OhmywuAPI
    PIXI?: PIXI
  }
}
