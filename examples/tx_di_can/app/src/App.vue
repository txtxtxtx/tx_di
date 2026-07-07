<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { api, onCanEvent } from './api/can.ts'
import { state, pushLog, handleEvent } from './store.ts'
import { t, i18n, toggleLang } from './i18n.ts'
import TraceView from './views/TraceView.vue'
import UdsView from './views/UdsView.vue'
import FlashView from './views/FlashView.vue'
import ConfigView from './views/ConfigView.vue'
import SimEcuView from './views/SimEcuView.vue'
import RecordReplayView from './views/RecordReplayView.vue'
import DbcView from './views/DbcView.vue'
import XcpView from './views/XcpView.vue'
import AuditView from './views/AuditView.vue'
import ProjectView from './views/ProjectView.vue'

const activeTab = ref('trace')
const tabs = computed(() => [
  { key: 'trace', label: t('tab.trace') },
  { key: 'uds', label: t('tab.uds') },
  { key: 'flash', label: t('tab.flash') },
  { key: 'simecu', label: t('tab.simecu') },
  { key: 'record', label: t('tab.record') },
  { key: 'dbc', label: t('tab.dbc') },
  { key: 'xcp', label: t('tab.xcp') },
  { key: 'audit', label: t('tab.audit') },
  { key: 'project', label: t('tab.project') },
  { key: 'config', label: t('tab.config') },
])

let unlisten: (() => void) | null = null

onMounted(async () => {
  try {
    unlisten = await onCanEvent(handleEvent)
    state.config = await api.getConfig()
    state.connected = await api.isConnected()
    pushLog('已连接事件总线')
  } catch (e) {
    pushLog('初始化失败: ' + String(e))
  }
})
onUnmounted(() => unlisten?.())
</script>

<template>
  <div class="app">
    <header class="topbar">
      <span class="title">CAN 诊断上位机</span>
      <span class="status" :class="{ on: state.connected }">
        {{ state.connected ? t('common.connected') : t('common.disconnected') }}
      </span>
      <button class="lang" @click="toggleLang">{{ i18n.lang === 'zh' ? 'EN' : '中' }}</button>
    </header>
    <nav class="tabs">
      <button
        v-for="ta in tabs"
        :key="ta.key"
        :class="{ active: activeTab === ta.key }"
        @click="activeTab = ta.key"
      >
        {{ ta.label }}
      </button>
    </nav>
    <main class="content">
      <TraceView v-if="activeTab === 'trace'" />
      <UdsView v-else-if="activeTab === 'uds'" />
      <FlashView v-else-if="activeTab === 'flash'" />
      <SimEcuView v-else-if="activeTab === 'simecu'" />
      <RecordReplayView v-else-if="activeTab === 'record'" />
      <DbcView v-else-if="activeTab === 'dbc'" />
      <XcpView v-else-if="activeTab === 'xcp'" />
      <AuditView v-else-if="activeTab === 'audit'" />
      <ProjectView v-else-if="activeTab === 'project'" />
      <ConfigView v-else-if="activeTab === 'config'" />
    </main>
  </div>
</template>
