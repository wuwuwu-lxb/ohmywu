import { createRouter, createWebHistory } from 'vue-router'

import ActionsView from '@/views/ActionsView.vue'
import AuditsView from '@/views/AuditsView.vue'
import ConversationView from '@/views/ConversationView.vue'
import LogsView from '@/views/LogsView.vue'
import OverviewView from '@/views/OverviewView.vue'
import ProcessesView from '@/views/ProcessesView.vue'
import ServicesView from '@/views/ServicesView.vue'
import StorageView from '@/views/StorageView.vue'
import TasksView from '@/views/TasksView.vue'

export default createRouter({
  history: createWebHistory(),
  routes: [
    // 用户模式
    { path: '/', name: 'overview', component: OverviewView },
    { path: '/tools/processes', name: 'tools-processes', component: ProcessesView },
    { path: '/tools/services', name: 'tools-services', component: ServicesView },
    { path: '/tools/storage', name: 'tools-storage', component: StorageView },
    { path: '/tools/logs', name: 'tools-logs', component: LogsView },
    { path: '/tasks', name: 'tasks', component: TasksView },
    { path: '/audits', name: 'audits', component: AuditsView },
    // Agent 模式（暂缓）
    { path: '/conversation', name: 'conversation', component: ConversationView },
    { path: '/actions', name: 'actions', component: ActionsView },
  ],
})
