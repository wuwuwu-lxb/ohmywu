import { createRouter, createWebHistory } from 'vue-router'

import ActionsView from '@/views/ActionsView.vue'
import LogsView from '@/views/LogsView.vue'
import OverviewView from '@/views/OverviewView.vue'
import ProcessesView from '@/views/ProcessesView.vue'
import ServicesView from '@/views/ServicesView.vue'
import StorageView from '@/views/StorageView.vue'
import SystemCenterView from '@/views/SystemCenterView.vue'
import TasksOverviewView from '@/views/TasksOverviewView.vue'

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'overview', component: OverviewView },
    {
      path: '/system',
      component: SystemCenterView,
      children: [
        { path: '', redirect: '/system/processes' },
        { path: 'processes', name: 'system-processes', component: ProcessesView },
        { path: 'services', name: 'system-services', component: ServicesView },
        { path: 'storage', name: 'system-storage', component: StorageView },
        { path: 'logs', name: 'system-logs', component: LogsView },
      ],
    },
    { path: '/actions', name: 'actions', component: ActionsView },
    { path: '/tasks', name: 'tasks-overview', component: TasksOverviewView },
  ],
})
