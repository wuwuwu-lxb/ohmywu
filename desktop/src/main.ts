import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHashHistory } from 'vue-router'
import App from './App.vue'
import './style.css'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/overview' },
    {
      path: '/overview',
      component: () => import('./views/OverviewView.vue'),
    },
    {
      path: '/conversation',
      component: () => import('./views/ConversationView.vue'),
    },
    {
      path: '/actions',
      component: () => import('./views/ActionsView.vue'),
    },
    {
      path: '/system',
      component: () => import('./views/SystemView.vue'),
    },
  ],
})

const pinia = createPinia()
const app = createApp(App)

app.use(pinia)
app.use(router)
app.mount('#app')

declare global {
  interface Window {
    ohmywu: {
      invoke: (channel: string, ...args: unknown[]) => Promise<unknown>
      on: (channel: string, callback: (...args: unknown[]) => void) => () => void
      platform: string
    }
  }
}
